//! Backtest engine — deterministic entry-timeframe replay with no lookahead.
//!
//! Flow:
//!   1. Load configured entry-timeframe CSV.
//!   2. Reject if data quality errors.
//!   3. Build CandleStore from configured timeframe roles.
//!   4. Precompute indicator snapshots for confirmation and screening roles.
//!   5. Replay entry-timeframe candles chronologically.
//!   6. For each candle:
//!      a. Handle pending entry (enter at candle open).
//!         - Compute actual adverse entry price.
//!         - Re-risk at actual price; soft-reject if invalid geometry or risk
//!           guards reject; record RiskRejection row(s).
//!      b. Check exit for open position (SL / TP / TimeExit) — even on entry candle.
//!      c. Evaluate strategy — no lookahead across configured higher timeframes; skipped on entry candle.
//!         - Initial risk assessment at signal-close price; soft-reject and record
//!           RiskRejection row(s) if rejected.
//!      d. If signal pre-approved by initial risk check, set pending entry.
//!   7. Close any remaining open position as EndOfBacktest.
//!   8. Finalise SignalFlowSummary.
//!   9. Return BacktestResult.
//!
//! Conservative intrabar rule: if SL and TP are both touched in the same candle,
//! SL is assumed first.
//!
//! No exchange calls. No LLMs. Historical simulation only.

use crate::backtest::fill_model::{FillModel, OpenSimPosition};
use crate::backtest::geometry::{adjusted_signal_for_actual_entry, EntryGeometryMode};
use crate::backtest::metrics::{BacktestSummary, EquityPoint, Metrics};
use crate::backtest::risk_trace::{RiskRejection, SignalFlowSummary};
use crate::core::{
    Candle, NorthflowError, PositionId, Side, Signal, Symbol, Timeframe, Trade, TradeExitReason,
    TradeId,
};
use crate::indicators::{IndicatorEngine, IndicatorSnapshot};
use crate::market::CandleStore;
use crate::risk::{CostModelConfig, RiskConfig, RiskContext, RiskEngine};
use crate::strategy::{MultiTimeframeInput, Strategy, StrategyContext};
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct TimeframeRoles {
    pub entry: Timeframe,
    pub confirmation: Timeframe,
    pub screening: Timeframe,
}

pub struct BacktestRunInput<'a> {
    pub symbol: Symbol,
    pub store: CandleStore,
    pub timeframes: TimeframeRoles,
    pub backtest: BacktestConfig,
    pub risk: RiskConfig,
    pub cost: CostModelConfig,
    pub strategy: &'a dyn Strategy,
    pub min_confidence: u8,
    pub entry_lookback_bars: usize,
    pub cooldown_bars: usize,
}

// ── BacktestConfig ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub initial_equity: f64,
    pub reports_dir: String,
    pub conservative_intrabar: bool,
    pub max_bars_held: u32,
    pub entry_geometry_mode: EntryGeometryMode,
}

// ── BacktestResult ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct BacktestResult {
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<EquityPoint>,
    pub summary: BacktestSummary,
    pub risk_rejections: Vec<RiskRejection>,
    pub signal_flow: SignalFlowSummary,
}

// ── BacktestEngine ────────────────────────────────────────────────────────────

pub struct BacktestEngine;

impl BacktestEngine {
    /// Run the backtest for one prepared symbol/store/strategy input.
    ///
    /// The engine performs deterministic replay only: callers own config parsing,
    /// file IO, data loading, candle-store construction, strategy resolution, and
    /// report writing.
    pub fn run(input: BacktestRunInput<'_>) -> Result<BacktestResult, NorthflowError> {
        let BacktestRunInput {
            symbol: symbol_obj,
            store,
            timeframes,
            backtest: bt_cfg,
            risk: risk_cfg,
            cost: cost_cfg,
            strategy,
            min_confidence,
            entry_lookback_bars,
            cooldown_bars,
        } = input;

        if store.entry_candles.is_empty() {
            return Err(NorthflowError::DataError(
                "prepared candle store has no entry candles".to_string(),
            ));
        }

        let entry_tf = timeframes.entry;
        let confirmation_tf = timeframes.confirmation;
        let screening_tf = timeframes.screening;

        // Precompute confirmation-TF snapshots (no-lookahead lookup at eval time).
        let mut eng_conf = IndicatorEngine::new_default()?;
        let mut snaps_conf: Vec<(i64, IndicatorSnapshot, Candle)> = Vec::new();
        for &c in &store.confirmation_candles {
            let snap = eng_conf.next(c)?;
            snaps_conf.push((c.timestamp, snap, c));
        }

        // Precompute screening-TF snapshots.
        let mut eng_screen = IndicatorEngine::new_default()?;
        let mut snaps_screen: Vec<(i64, IndicatorSnapshot, Candle)> = Vec::new();
        for &c in &store.screening_candles {
            let snap = eng_screen.next(c)?;
            snaps_screen.push((c.timestamp, snap, c));
        }

        // ── Main replay loop ──────────────────────────────────────────────────

        let mut equity = bt_cfg.initial_equity;
        let mut peak_equity = equity;
        let mut daily_realized_pnl = 0.0_f64;
        let mut current_day = -1_i64;

        let mut trades: Vec<Trade> = Vec::new();
        let mut equity_curve: Vec<EquityPoint> = Vec::new();

        // Initial equity point.
        if let Some(first) = store.entry_candles.first() {
            equity_curve.push(EquityPoint {
                timestamp: first.timestamp,
                equity,
                drawdown_pct: 0.0,
            });
        }

        let mut signal_counter: u64 = 0;
        let mut pending_entry: Option<Signal> = None;
        let mut open_position: Option<OpenSimPosition> = None;
        let mut risk_rejections: Vec<RiskRejection> = Vec::new();
        let mut signal_flow = SignalFlowSummary::default();

        let mut last_signal_bar: Option<usize> = None;
        let mut eng_entry = IndicatorEngine::new_default()?;

        let entry_candles = &store.entry_candles;
        let n = entry_candles.len();
        let mut progress = BacktestProgress::new(n);
        progress.tick(0, 0);

        for i in 0..n {
            let candle = entry_candles[i];

            // Update entry-timeframe indicator engine.
            let entry_snapshot = eng_entry.next(candle)?;

            // Day boundary — reset daily PnL.
            let day = candle.timestamp / 86_400_000;
            if day != current_day {
                current_day = day;
                daily_realized_pnl = 0.0;
            }

            // ── A. Handle pending entry ───────────────────────────────────────
            // A signal from the previous candle is entered at THIS candle's open.
            // Actual entry price is computed with adverse slippage at candle open.
            // Risk is re-assessed at the actual price; invalid geometry or risk
            // rejection are recorded as soft rejections (not fatal).
            // After this block, fall through to B so exit checks run on the entry candle.
            let mut entered_this_bar = false;
            if let Some(signal) = pending_entry.take() {
                if open_position.is_none() && equity > 0.0 {
                    let actual_price = FillModel::adverse_entry_price(
                        signal.side,
                        candle.open,
                        cost_cfg.slippage_bps,
                    );
                    let adjusted = adjusted_signal_for_actual_entry(
                        &signal,
                        actual_price,
                        bt_cfg.entry_geometry_mode,
                    );

                    if !adjusted.valid_geometry() {
                        // Adverse slippage made the trade geometry invalid — soft reject.
                        signal_flow.signals_rejected_actual_entry += 1;
                        risk_rejections.push(build_rejection(
                            &adjusted,
                            "actual_entry",
                            bt_cfg.entry_geometry_mode.as_str(),
                            candle.timestamp,
                            equity,
                            peak_equity,
                            daily_realized_pnl,
                            "actual_entry_invalid_geometry",
                            adjusted.expected_reward_bps,
                            adjusted.estimated_cost_bps,
                            adjusted.expected_net_edge_bps,
                        ));
                    } else {
                        let risk_ctx = RiskContext {
                            equity,
                            peak_equity,
                            daily_realized_pnl,
                            open_positions: usize::from(open_position.is_some()),
                        };
                        match RiskEngine::assess(&risk_cfg, &cost_cfg, &risk_ctx, &adjusted) {
                            Err(NorthflowError::InvalidSignal(_)) => {
                                // Geometry looked valid but signal failed deeper validation —
                                // treat as soft actual-entry rejection.
                                signal_flow.signals_rejected_actual_entry += 1;
                                risk_rejections.push(build_rejection(
                                    &adjusted,
                                    "actual_entry",
                                    bt_cfg.entry_geometry_mode.as_str(),
                                    candle.timestamp,
                                    equity,
                                    peak_equity,
                                    daily_realized_pnl,
                                    "actual_entry_risk_error",
                                    adjusted.expected_reward_bps,
                                    adjusted.estimated_cost_bps,
                                    adjusted.expected_net_edge_bps,
                                ));
                            }
                            Err(e) => return Err(e),
                            Ok(assessment) if !assessment.approved => {
                                signal_flow.signals_rejected_actual_entry += 1;
                                for reason in &assessment.failed {
                                    risk_rejections.push(build_rejection(
                                        &adjusted,
                                        "actual_entry",
                                        bt_cfg.entry_geometry_mode.as_str(),
                                        candle.timestamp,
                                        equity,
                                        peak_equity,
                                        daily_realized_pnl,
                                        reason,
                                        assessment.expected_reward_bps,
                                        assessment.expected_cost_bps,
                                        assessment.expected_net_edge_bps,
                                    ));
                                }
                            }
                            Ok(assessment) => {
                                if let Some(qty) = assessment.qty {
                                    if qty > 0.0 {
                                        let entry = FillModel::simulate_entry(
                                            &adjusted,
                                            qty,
                                            &candle,
                                            cost_cfg.slippage_bps,
                                            cost_cfg.taker_fee_bps,
                                        );
                                        open_position = Some(OpenSimPosition {
                                            signal: adjusted,
                                            qty,
                                            entry_time: entry.time,
                                            entry_price: entry.price,
                                            entry_fee: entry.fee,
                                            entry_slippage: entry.slippage,
                                            bars_held: 0,
                                        });
                                        signal_flow.trades_opened += 1;
                                        entered_this_bar = true;
                                    }
                                }
                            }
                        }
                    }
                }
                // Do NOT continue — fall through to B so exit checks run on the
                // entry candle.  Strategy evaluation (C) is skipped via the flag.
            }

            // ── B. Check exits for open position ─────────────────────────────
            let mut closed_this_bar = false;
            if let Some(ref mut pos) = open_position {
                pos.bars_held += 1;

                let exit_fill = FillModel::check_exit(
                    pos,
                    &candle,
                    bt_cfg.conservative_intrabar,
                    cost_cfg.slippage_bps,
                    cost_cfg.taker_fee_bps,
                    bt_cfg.max_bars_held,
                );

                if let Some(exit) = exit_fill {
                    let trade = build_trade(pos, &exit, symbol_obj.clone(), &cost_cfg);
                    equity += trade.net_pnl;
                    daily_realized_pnl += trade.net_pnl;
                    peak_equity = peak_equity.max(equity);
                    let dd = drawdown_pct(peak_equity, equity);

                    equity_curve.push(EquityPoint {
                        timestamp: candle.timestamp,
                        equity,
                        drawdown_pct: dd,
                    });
                    trades.push(trade);
                    closed_this_bar = true;
                }
            }

            if closed_this_bar {
                open_position = None;
                if equity <= 0.0 {
                    break;
                }
            }

            // ── C. Evaluate strategy — no lookahead ───────────────────────────
            // Skipped on the candle where an entry was just opened to avoid
            // evaluating a new signal before the just-opened trade has had a
            // chance to develop.
            // Cooldown: if strategy cooldown > 0, skip strategy evaluation for
            // that many bars after the last signal was preapproved.
            let in_cooldown = cooldown_bars > 0
                && last_signal_bar.map_or(false, |last| i.saturating_sub(last) <= cooldown_bars);
            if !entered_this_bar && open_position.is_none() && equity > 0.0 && !in_cooldown {
                // No-lookahead rule (generic):
                //   signal_time = candle.timestamp + entry_tf.to_millis()
                //   conf available: conf_ts + conf_tf.to_millis() <= signal_time
                //     → conf_ts <= candle.ts + entry_tf.to_millis() - conf_tf.to_millis()
                //   screen available: screen_ts + screen_tf.to_millis() <= signal_time
                //     → screen_ts <= candle.ts + entry_tf.to_millis() - screen_tf.to_millis()
                let max_conf_ts =
                    candle.timestamp + entry_tf.to_millis() - confirmation_tf.to_millis();
                let max_screen_ts =
                    candle.timestamp + entry_tf.to_millis() - screening_tf.to_millis();

                let confirmation_snapshot = latest_snap(&snaps_conf, max_conf_ts);
                let screening_snapshot = latest_snap(&snaps_screen, max_screen_ts);

                if let (
                    Some((_, confirmation_indicators, confirmation_candle)),
                    Some((_, screening_indicators, screening_candle)),
                ) = (confirmation_snapshot, screening_snapshot)
                {
                    let estimated_cost = cost_cfg.taker_fee_bps * 2.0
                        + cost_cfg.slippage_bps * 2.0
                        + cost_cfg.spread_bps;

                    let ctx = StrategyContext {
                        symbol: symbol_obj.clone(),
                        signal_index: signal_counter + 1,
                        estimated_cost_bps: estimated_cost,
                        min_confidence,
                        entry_timeframe: entry_tf,
                        confirmation_timeframe: confirmation_tf,
                        screening_timeframe: screening_tf,
                    };

                    let entry_lookback = entry_lookback_for(entry_candles, i, entry_lookback_bars);

                    let input = MultiTimeframeInput {
                        entry_candle: candle,
                        entry_lookback,
                        confirmation_candle: *confirmation_candle,
                        screening_candle: *screening_candle,
                        entry_indicators: entry_snapshot.clone(),
                        confirmation_indicators: confirmation_indicators.clone(),
                        screening_indicators: screening_indicators.clone(),
                    };

                    match strategy.evaluate(&ctx, &input) {
                        Ok(None) => {}
                        Ok(Some(signal)) => {
                            signal_counter += 1;
                            signal_flow.signals_generated += 1;

                            let risk_ctx = RiskContext {
                                equity,
                                peak_equity,
                                daily_realized_pnl,
                                open_positions: usize::from(open_position.is_some()),
                            };

                            match RiskEngine::assess(&risk_cfg, &cost_cfg, &risk_ctx, &signal) {
                                Err(e) => return Err(e),
                                Ok(assessment) if !assessment.approved => {
                                    signal_flow.signals_rejected_initial_risk += 1;
                                    for reason in &assessment.failed {
                                        risk_rejections.push(build_rejection(
                                            &signal,
                                            "initial_risk",
                                            bt_cfg.entry_geometry_mode.as_str(),
                                            candle.timestamp,
                                            equity,
                                            peak_equity,
                                            daily_realized_pnl,
                                            reason,
                                            assessment.expected_reward_bps,
                                            assessment.expected_cost_bps,
                                            assessment.expected_net_edge_bps,
                                        ));
                                    }
                                }
                                Ok(_) => {
                                    signal_flow.signals_preapproved += 1;
                                    last_signal_bar = Some(i);
                                    pending_entry = Some(signal);
                                }
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            }

            progress.tick(i + 1, trades.len());
        }

        // ── End of backtest: close any remaining position ─────────────────────
        if let Some(ref pos) = open_position {
            if let Some(&last) = entry_candles.last() {
                let exit = FillModel::end_of_backtest_exit(
                    pos,
                    &last,
                    cost_cfg.slippage_bps,
                    cost_cfg.taker_fee_bps,
                );
                let trade = build_trade(pos, &exit, symbol_obj.clone(), &cost_cfg);
                equity += trade.net_pnl;
                peak_equity = peak_equity.max(equity);
                let dd = drawdown_pct(peak_equity, equity);
                equity_curve.push(EquityPoint {
                    timestamp: last.timestamp,
                    equity,
                    drawdown_pct: dd,
                });
                trades.push(trade);
            }
        }

        // Finalise signal flow counters.
        signal_flow.entry_geometry_mode = bt_cfg.entry_geometry_mode.as_str().to_string();
        signal_flow.finalise(&risk_rejections, trades.len());

        progress.finish(trades.len());

        let summary = Metrics::summarize(&trades, &equity_curve);

        Ok(BacktestResult {
            trades,
            equity_curve,
            summary,
            risk_rejections,
            signal_flow,
        })
    }
}

struct BacktestProgress {
    total: usize,
    last_bucket: Option<usize>,
    started_at: Instant,
}

impl BacktestProgress {
    fn new(total: usize) -> Self {
        Self {
            total,
            last_bucket: None,
            started_at: Instant::now(),
        }
    }

    fn tick(&mut self, current: usize, trades: usize) {
        let percent = if self.total == 0 {
            100.0
        } else {
            current as f64 / self.total as f64 * 100.0
        };
        let bucket = if current >= self.total {
            10
        } else {
            (percent.floor() as usize / 10).min(9)
        };
        if self.last_bucket == Some(bucket) {
            return;
        }
        self.last_bucket = Some(bucket);
        println!(
            "  {}",
            format_progress_line(
                current.min(self.total),
                self.total,
                trades,
                self.started_at.elapsed().as_secs(),
                30
            )
        );
    }

    fn finish(&mut self, trades: usize) {
        self.tick(self.total, trades);
    }
}

fn format_progress_line(
    current: usize,
    total: usize,
    trades: usize,
    elapsed_secs: u64,
    width: usize,
) -> String {
    let percent = if total == 0 {
        100.0
    } else {
        current as f64 / total as f64 * 100.0
    };
    let filled = if total == 0 {
        width
    } else {
        ((percent / 100.0) * width as f64).round() as usize
    }
    .min(width);
    format!(
        "[{}{}] {:>5.1}% {}/{} candles | trades: {} | elapsed: {}",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled)),
        percent,
        format_usize(current),
        format_usize(total),
        format_usize(trades),
        format_elapsed(elapsed_secs)
    )
}

fn format_usize(value: usize) -> String {
    let s = value.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let first_group = s.len() % 3;
    for (idx, ch) in s.chars().enumerate() {
        if idx > 0 && (idx == first_group || (idx > first_group && (idx - first_group) % 3 == 0)) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn format_elapsed(seconds: u64) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{minutes:02}:{secs:02}")
}

// ── Helper: latest completed snapshot with ts <= max_ts ───────────────────────
//
// Uses partition_point (binary search) for O(log n) lookup.
// The slice is always sorted chronologically because snapshots are precomputed
// from sorted candles.  No-lookahead semantics are unchanged: only snapshots
// with ts <= max_ts are eligible.

fn latest_snap<'a>(
    snaps: &'a [(i64, IndicatorSnapshot, Candle)],
    max_ts: i64,
) -> Option<&'a (i64, IndicatorSnapshot, Candle)> {
    let idx = snaps.partition_point(|(ts, _, _)| *ts <= max_ts);

    if idx == 0 {
        None
    } else {
        Some(&snaps[idx - 1])
    }
}

// ── Helper: drawdown percentage ───────────────────────────────────────────────

fn drawdown_pct(peak: f64, equity: f64) -> f64 {
    if peak <= 0.0 {
        return 0.0;
    }
    ((peak - equity) / peak * 100.0).max(0.0)
}

// ── Helper: build a Trade from a closed position ──────────────────────────────

fn build_trade(
    pos: &OpenSimPosition,
    exit: &crate::backtest::fill_model::ExitFill,
    symbol: Symbol,
    cost_cfg: &CostModelConfig,
) -> Trade {
    let entry_notional = pos.entry_price * pos.qty;
    let spread_cost = entry_notional * cost_cfg.spread_bps / 10_000.0;
    let market_impact_cost = entry_notional * cost_cfg.market_impact_bps / 10_000.0;
    let stop_slippage_cost = if exit.reason == TradeExitReason::StopLoss {
        entry_notional * cost_cfg.stop_slippage_bps / 10_000.0
    } else {
        0.0
    };

    let fee = pos.entry_fee + exit.fee;
    // Accounting model A: fill prices already include adverse entry/exit slippage.
    // `Trade.slippage` is diagnostic embedded-price slippage only; net PnL must
    // not subtract it again. Explicit costs below remain nominal deductions.
    let slippage = pos.entry_slippage + exit.slippage;
    let explicit_cost = spread_cost + market_impact_cost + stop_slippage_cost;

    let gross_pnl = match pos.signal.side {
        Side::Long => (exit.price - pos.entry_price) * pos.qty,
        Side::Short => (pos.entry_price - exit.price) * pos.qty,
    };
    let net_pnl = gross_pnl - fee - explicit_cost;

    let actual_edge_bps = if entry_notional > 0.0 {
        net_pnl / entry_notional * 10_000.0
    } else {
        0.0
    };

    let risk = (pos.entry_price - pos.signal.stop_loss).abs();
    let reward = (pos.signal.take_profit - pos.entry_price).abs();
    let reward_risk = if risk > 0.0 { reward / risk } else { 0.0 };

    let sig_id = pos.signal.signal_id.as_str();
    let position_id = PositionId::new(format!("POS-{sig_id}"));
    let trade_id = TradeId::new(format!("TRD-{sig_id}"));

    Trade {
        trade_id,
        signal_id: pos.signal.signal_id.clone(),
        position_id,
        symbol,
        strategy_id: pos.signal.strategy_id.clone(),
        regime: pos.signal.regime.clone(),
        side: pos.signal.side,
        entry_time: pos.entry_time,
        exit_time: exit.time,
        entry_price: pos.entry_price,
        exit_price: exit.price,
        stop_loss: pos.signal.stop_loss,
        take_profit: pos.signal.take_profit,
        quantity: pos.qty,
        gross_pnl,
        fee,
        slippage,
        net_pnl,
        reward_risk,
        bars_held: exit.bars_held,
        exit_reason: exit.reason,
        entry_reason: pos.signal.entry_reason.clone(),
        filters_passed: pos.signal.filters_passed.clone(),
        filters_failed: pos.signal.filters_failed.clone(),
        expected_edge_bps: pos.signal.expected_net_edge_bps,
        actual_edge_bps,
    }
}

// ── Helper: build a RiskRejection record ──────────────────────────────────────

fn build_rejection(
    signal: &Signal,
    stage: &str,
    entry_geometry_mode: &str,
    timestamp: i64,
    equity: f64,
    peak_equity: f64,
    daily_realized_pnl: f64,
    reason: &str,
    expected_reward_bps: f64,
    expected_cost_bps: f64,
    expected_net_edge_bps: f64,
) -> RiskRejection {
    let drawdown_pct = if peak_equity > 0.0 && equity < peak_equity {
        (peak_equity - equity) / peak_equity * 100.0
    } else {
        0.0
    };
    RiskRejection {
        signal_id: signal.signal_id.as_str().to_string(),
        stage: stage.to_string(),
        entry_geometry_mode: entry_geometry_mode.to_string(),
        timestamp,
        side: signal.side.as_str().to_string(),
        regime: signal.regime.clone(),
        reason: reason.to_string(),
        equity,
        peak_equity,
        drawdown_pct,
        daily_realized_pnl,
        expected_reward_bps,
        expected_cost_bps,
        expected_net_edge_bps,
    }
}

fn entry_lookback_for(
    entry_candles: &[Candle],
    current_index: usize,
    lookback_bars: usize,
) -> Vec<Candle> {
    let lookback_start = current_index.saturating_sub(lookback_bars);
    entry_candles[lookback_start..current_index].to_vec()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{SignalId, StrategyId};
    use crate::market::CandleStore;
    use crate::risk::RiskConfig;

    #[derive(Debug)]
    struct NoopStrategy;

    impl Strategy for NoopStrategy {
        fn strategy_id(&self) -> &'static str {
            "test_noop_strategy"
        }

        fn evaluate(
            &self,
            _ctx: &StrategyContext,
            _input: &MultiTimeframeInput,
        ) -> Result<Option<Signal>, NorthflowError> {
            Ok(None)
        }
    }

    fn make_candle(ts: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: ts,
            open,
            high,
            low,
            close,
            volume: 1000.0,
        }
    }

    fn prepared_input(strategy: &dyn Strategy) -> BacktestRunInput<'_> {
        let candles = (0..240)
            .map(|i| make_candle(1_700_000_000_000 + i * 60_000, 100.0, 101.0, 99.0, 100.0))
            .collect::<Vec<_>>();
        let entry = Timeframe::from_str("1m").unwrap();
        let confirmation = Timeframe::from_str("5m").unwrap();
        let screening = Timeframe::from_str("15m").unwrap();
        let store = CandleStore::build(candles, entry, confirmation, screening).unwrap();
        BacktestRunInput {
            symbol: Symbol::new("BTCUSDT").unwrap(),
            store,
            timeframes: TimeframeRoles {
                entry,
                confirmation,
                screening,
            },
            backtest: BacktestConfig {
                initial_equity: 5000.0,
                reports_dir: "reports/test".to_string(),
                conservative_intrabar: true,
                max_bars_held: 18,
                entry_geometry_mode: EntryGeometryMode::PreserveSignalLevels,
            },
            risk: RiskConfig {
                risk_per_trade_pct: 0.15,
                max_open_positions: 1,
                max_leverage: 3.0,
                min_reward_risk: 1.3,
                max_daily_loss_pct: 3.0,
                max_drawdown_pct: 100.0,
            },
            cost: CostModelConfig {
                taker_fee_bps: 4.0,
                slippage_bps: 2.0,
                spread_bps: 1.0,
                market_impact_bps: 1.0,
                stop_slippage_bps: 5.0,
            },
            strategy,
            min_confidence: 90,
            entry_lookback_bars: 120,
            cooldown_bars: 0,
        }
    }

    #[test]
    fn progress_formatter_renders_key_percentages() {
        assert!(format_progress_line(0, 100, 0, 0, 10).contains("  0.0%"));
        assert!(format_progress_line(50, 100, 3, 1, 10).contains(" 50.0%"));
        assert!(format_progress_line(100, 100, 9, 2, 10).contains("100.0%"));
    }

    #[test]
    fn engine_runs_with_prepared_store_and_stub_strategy() {
        let strategy = NoopStrategy;
        let result = BacktestEngine::run(prepared_input(&strategy)).unwrap();
        assert_eq!(result.summary.total_trades, 0);
        assert!(!result.equity_curve.is_empty());
    }

    #[test]
    fn entry_lookback_excludes_current_candle() {
        let candles = vec![
            make_candle(1, 10.0, 11.0, 9.0, 10.0),
            make_candle(2, 20.0, 21.0, 19.0, 20.0),
            make_candle(3, 30.0, 31.0, 29.0, 30.0),
        ];
        let lookback = entry_lookback_for(&candles, 2, 2);
        assert_eq!(
            lookback.iter().map(|c| c.timestamp).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn ids_preserve_signal_relationship() {
        let signal_id = SignalId::new("SIG-BT-00000001");
        let trade_id = TradeId::new(format!("TRD-{}", signal_id.as_str()));
        assert_eq!(trade_id.as_str(), "TRD-SIG-BT-00000001");
        assert_eq!(StrategyId::new("test").as_str(), "test");
    }
}
