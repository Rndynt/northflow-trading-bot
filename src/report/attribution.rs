//! Attribution engine — groups trades by regime, exit reason, side, and filter.
//!
//! All calculations are deterministic.
//! Buckets are sorted by key ascending for stable output.
//! No external dependencies.

use std::collections::BTreeMap;

use crate::core::Trade;

// ── AttributionSummary ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AttributionSummary {
    pub total_trades: usize,
    pub total_signals_with_trades: usize,
    pub unique_signal_ids: usize,
    pub unique_trade_ids: usize,
    pub avg_expected_edge_bps: f64,
    pub avg_actual_edge_bps: f64,
    pub edge_realization_bps: f64,
    pub positive_expected_edge_trades: usize,
    pub positive_actual_edge_trades: usize,
    pub filters_passed_count: usize,
    pub filters_failed_count: usize,
}

// ── AttributionBucket ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AttributionBucket {
    pub key: String,
    pub trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub net_pnl: f64,
    pub gross_pnl: f64,
    pub total_fee: f64,
    pub total_slippage: f64,
    pub avg_net_pnl: f64,
    pub avg_expected_edge_bps: f64,
    pub avg_actual_edge_bps: f64,
    pub avg_bars_held: f64,
}

// ── AttributionReport ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AttributionReport {
    pub summary: AttributionSummary,
    pub by_regime: Vec<AttributionBucket>,
    pub by_exit_reason: Vec<AttributionBucket>,
    pub by_side: Vec<AttributionBucket>,
    pub by_filter: Vec<AttributionBucket>,
}

// ── AttributionEngine ─────────────────────────────────────────────────────────

pub struct AttributionEngine;

impl AttributionEngine {
    /// Build a full attribution report from a slice of closed trades.
    ///
    /// Returns zero-valued summary and empty bucket lists when `trades` is empty.
    pub fn build(trades: &[Trade]) -> AttributionReport {
        if trades.is_empty() {
            return AttributionReport {
                summary: AttributionSummary {
                    total_trades: 0,
                    total_signals_with_trades: 0,
                    unique_signal_ids: 0,
                    unique_trade_ids: 0,
                    avg_expected_edge_bps: 0.0,
                    avg_actual_edge_bps: 0.0,
                    edge_realization_bps: 0.0,
                    positive_expected_edge_trades: 0,
                    positive_actual_edge_trades: 0,
                    filters_passed_count: 0,
                    filters_failed_count: 0,
                },
                by_regime: vec![],
                by_exit_reason: vec![],
                by_side: vec![],
                by_filter: vec![],
            };
        }

        // ── Summary ───────────────────────────────────────────────────────────
        let total_trades = trades.len();

        let mut signal_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut trade_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut sum_expected = 0.0_f64;
        let mut sum_actual = 0.0_f64;
        let mut pos_expected = 0_usize;
        let mut pos_actual = 0_usize;
        let mut filters_passed_count = 0_usize;
        let mut filters_failed_count = 0_usize;

        for t in trades {
            signal_ids.insert(t.signal_id.as_str().to_string());
            trade_ids.insert(t.trade_id.as_str().to_string());
            sum_expected += t.expected_edge_bps;
            sum_actual += t.actual_edge_bps;
            if t.expected_edge_bps > 0.0 {
                pos_expected += 1;
            }
            if t.actual_edge_bps > 0.0 {
                pos_actual += 1;
            }
            filters_passed_count += t.filters_passed.len();
            filters_failed_count += t.filters_failed.len();
        }

        let avg_expected_edge_bps = sum_expected / total_trades as f64;
        let avg_actual_edge_bps = sum_actual / total_trades as f64;
        let edge_realization_bps = avg_actual_edge_bps - avg_expected_edge_bps;

        let summary = AttributionSummary {
            total_trades,
            total_signals_with_trades: signal_ids.len(),
            unique_signal_ids: signal_ids.len(),
            unique_trade_ids: trade_ids.len(),
            avg_expected_edge_bps,
            avg_actual_edge_bps,
            edge_realization_bps,
            positive_expected_edge_trades: pos_expected,
            positive_actual_edge_trades: pos_actual,
            filters_passed_count,
            filters_failed_count,
        };

        // ── Bucketed attribution ──────────────────────────────────────────────
        let by_regime = Self::bucket_by(trades, |t| t.regime.clone());
        let by_exit_reason = Self::bucket_by(trades, |t| t.exit_reason.as_str().to_string());
        let by_side = Self::bucket_by(trades, |t| t.side.as_str().to_string());
        let by_filter = Self::build_filter_buckets(trades);

        AttributionReport {
            summary,
            by_regime,
            by_exit_reason,
            by_side,
            by_filter,
        }
    }

    // ── private helpers ───────────────────────────────────────────────────────

    fn bucket_by<F>(trades: &[Trade], key_fn: F) -> Vec<AttributionBucket>
    where
        F: Fn(&Trade) -> String,
    {
        let mut map: BTreeMap<String, Vec<&Trade>> = BTreeMap::new();
        for t in trades {
            map.entry(key_fn(t)).or_default().push(t);
        }
        // BTreeMap iterates in key-ascending order — output is deterministic.
        map.into_iter()
            .map(|(k, v)| Self::make_bucket(k, &v))
            .collect()
    }

    /// Build filter buckets.
    ///
    /// Key format:
    ///   passed filters → `passed:<filter_name>`
    ///   failed filters → `failed:<filter_name>`
    ///
    /// Trades with no filters at all produce no buckets.
    fn build_filter_buckets(trades: &[Trade]) -> Vec<AttributionBucket> {
        let mut map: BTreeMap<String, Vec<&Trade>> = BTreeMap::new();
        for t in trades {
            for f in &t.filters_passed {
                map.entry(format!("passed:{f}")).or_default().push(t);
            }
            for f in &t.filters_failed {
                map.entry(format!("failed:{f}")).or_default().push(t);
            }
        }
        map.into_iter()
            .map(|(k, v)| Self::make_bucket(k, &v))
            .collect()
    }

    fn make_bucket(key: String, trades: &[&Trade]) -> AttributionBucket {
        let n = trades.len();
        let wins = trades.iter().filter(|t| t.net_pnl > 0.0).count();
        let losses = n - wins;
        let win_rate = if n == 0 {
            0.0
        } else {
            wins as f64 / n as f64 * 100.0
        };
        let net_pnl: f64 = trades.iter().map(|t| t.net_pnl).sum();
        let gross_pnl: f64 = trades.iter().map(|t| t.gross_pnl).sum();
        let total_fee: f64 = trades.iter().map(|t| t.fee).sum();
        let total_slippage: f64 = trades.iter().map(|t| t.slippage).sum();
        let avg_net_pnl = if n == 0 { 0.0 } else { net_pnl / n as f64 };
        let avg_expected_edge_bps = if n == 0 {
            0.0
        } else {
            trades.iter().map(|t| t.expected_edge_bps).sum::<f64>() / n as f64
        };
        let avg_actual_edge_bps = if n == 0 {
            0.0
        } else {
            trades.iter().map(|t| t.actual_edge_bps).sum::<f64>() / n as f64
        };
        let avg_bars_held = if n == 0 {
            0.0
        } else {
            trades.iter().map(|t| t.bars_held as f64).sum::<f64>() / n as f64
        };

        AttributionBucket {
            key,
            trades: n,
            wins,
            losses,
            win_rate,
            net_pnl,
            gross_pnl,
            total_fee,
            total_slippage,
            avg_net_pnl,
            avg_expected_edge_bps,
            avg_actual_edge_bps,
            avg_bars_held,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        Trade, position::PositionId, side::Side, signal::SignalId, signal::StrategyId,
        symbol::Symbol, trade::TradeExitReason, trade::TradeId,
    };

    fn make_trade(
        trade_id: &str,
        signal_id: &str,
        regime: &str,
        side: Side,
        exit_reason: TradeExitReason,
        net_pnl: f64,
        expected_edge_bps: f64,
        actual_edge_bps: f64,
        bars_held: u32,
        filters_passed: Vec<&str>,
        filters_failed: Vec<&str>,
    ) -> Trade {
        Trade {
            trade_id: TradeId::new(trade_id),
            signal_id: SignalId::new(signal_id),
            position_id: PositionId::new(format!("POS-{signal_id}")),
            symbol: Symbol::new("BTCUSDT").unwrap(),
            strategy_id: StrategyId::new("screened_vwap_scalp"),
            regime: regime.to_string(),
            side,
            entry_time: 1_700_000_000_000,
            exit_time: 1_700_000_060_000,
            entry_price: 30_000.0,
            exit_price: 30_600.0,
            stop_loss: 29_700.0,
            take_profit: 30_600.0,
            quantity: 0.1,
            gross_pnl: net_pnl + 5.0,
            fee: 3.0,
            slippage: 2.0,
            net_pnl,
            reward_risk: 2.0,
            bars_held,
            exit_reason,
            entry_reason: "ema_cross".to_string(),
            filters_passed: filters_passed.iter().map(|s| s.to_string()).collect(),
            filters_failed: filters_failed.iter().map(|s| s.to_string()).collect(),
            expected_edge_bps,
            actual_edge_bps,
        }
    }

    fn win() -> Trade {
        make_trade(
            "TRD-SIG-BT-00000001",
            "SIG-BT-00000001",
            "bullish",
            Side::Long,
            TradeExitReason::TakeProfit,
            50.0,
            192.0,
            200.0,
            10,
            vec!["atr_valid"],
            vec![],
        )
    }

    fn loss() -> Trade {
        make_trade(
            "TRD-SIG-BT-00000002",
            "SIG-BT-00000002",
            "bearish",
            Side::Short,
            TradeExitReason::StopLoss,
            -30.0,
            150.0,
            -120.0,
            5,
            vec!["atr_valid"],
            vec!["volume_below_threshold"],
        )
    }

    #[test]
    fn attribution_empty_trades_has_zero_summary() {
        let r = AttributionEngine::build(&[]);
        assert_eq!(r.summary.total_trades, 0);
        assert_eq!(r.summary.unique_signal_ids, 0);
        assert_eq!(r.summary.unique_trade_ids, 0);
        assert_eq!(r.summary.avg_expected_edge_bps, 0.0);
        assert_eq!(r.summary.avg_actual_edge_bps, 0.0);
        assert_eq!(r.summary.edge_realization_bps, 0.0);
        assert!(r.by_regime.is_empty());
        assert!(r.by_exit_reason.is_empty());
        assert!(r.by_side.is_empty());
        assert!(r.by_filter.is_empty());
    }

    #[test]
    fn attribution_counts_unique_signal_ids() {
        let r = AttributionEngine::build(&[win(), loss()]);
        assert_eq!(r.summary.unique_signal_ids, 2);
    }

    #[test]
    fn attribution_counts_unique_trade_ids() {
        let r = AttributionEngine::build(&[win(), loss()]);
        assert_eq!(r.summary.unique_trade_ids, 2);
    }

    #[test]
    fn attribution_calculates_avg_expected_edge() {
        let r = AttributionEngine::build(&[win(), loss()]);
        // (192 + 150) / 2 = 171
        let expected = (192.0 + 150.0) / 2.0;
        assert!((r.summary.avg_expected_edge_bps - expected).abs() < 1e-9);
    }

    #[test]
    fn attribution_calculates_avg_actual_edge() {
        let r = AttributionEngine::build(&[win(), loss()]);
        // (200 + (-120)) / 2 = 40
        let expected = (200.0 + (-120.0)) / 2.0;
        assert!((r.summary.avg_actual_edge_bps - expected).abs() < 1e-9);
    }

    #[test]
    fn attribution_calculates_edge_realization() {
        let r = AttributionEngine::build(&[win(), loss()]);
        let expected = r.summary.avg_actual_edge_bps - r.summary.avg_expected_edge_bps;
        assert!((r.summary.edge_realization_bps - expected).abs() < 1e-9);
    }

    #[test]
    fn attribution_groups_by_regime() {
        let r = AttributionEngine::build(&[win(), loss()]);
        // win → bullish, loss → bearish
        assert_eq!(r.by_regime.len(), 2);
        let bearish = r.by_regime.iter().find(|b| b.key == "bearish").unwrap();
        assert_eq!(bearish.trades, 1);
        assert_eq!(bearish.losses, 1);
        let bullish = r.by_regime.iter().find(|b| b.key == "bullish").unwrap();
        assert_eq!(bullish.wins, 1);
    }

    #[test]
    fn attribution_groups_by_exit_reason() {
        let r = AttributionEngine::build(&[win(), loss()]);
        assert_eq!(r.by_exit_reason.len(), 2);
        let sl = r
            .by_exit_reason
            .iter()
            .find(|b| b.key == "stop_loss")
            .unwrap();
        assert_eq!(sl.trades, 1);
        assert_eq!(sl.losses, 1);
    }

    #[test]
    fn attribution_groups_by_side() {
        let r = AttributionEngine::build(&[win(), loss()]);
        // win → long, loss → short
        assert_eq!(r.by_side.len(), 2);
        let long_b = r.by_side.iter().find(|b| b.key == "long").unwrap();
        assert_eq!(long_b.wins, 1);
        let short_b = r.by_side.iter().find(|b| b.key == "short").unwrap();
        assert_eq!(short_b.losses, 1);
    }

    #[test]
    fn attribution_groups_by_passed_filter() {
        let r = AttributionEngine::build(&[win(), loss()]);
        // both trades have "atr_valid" in filters_passed
        let atr = r
            .by_filter
            .iter()
            .find(|b| b.key == "passed:atr_valid")
            .unwrap();
        assert_eq!(atr.trades, 2);
    }

    #[test]
    fn attribution_groups_by_failed_filter() {
        let r = AttributionEngine::build(&[win(), loss()]);
        // loss has "volume_below_threshold" in filters_failed
        let failed = r
            .by_filter
            .iter()
            .find(|b| b.key == "failed:volume_below_threshold")
            .unwrap();
        assert_eq!(failed.trades, 1);
    }

    #[test]
    fn attribution_buckets_are_sorted_by_key() {
        let r = AttributionEngine::build(&[win(), loss()]);
        let regime_keys: Vec<&str> = r.by_regime.iter().map(|b| b.key.as_str()).collect();
        let mut sorted = regime_keys.clone();
        sorted.sort();
        assert_eq!(regime_keys, sorted, "by_regime must be sorted by key");

        let side_keys: Vec<&str> = r.by_side.iter().map(|b| b.key.as_str()).collect();
        let mut sorted = side_keys.clone();
        sorted.sort();
        assert_eq!(side_keys, sorted, "by_side must be sorted by key");

        let filter_keys: Vec<&str> = r.by_filter.iter().map(|b| b.key.as_str()).collect();
        let mut sorted = filter_keys.clone();
        sorted.sort();
        assert_eq!(filter_keys, sorted, "by_filter must be sorted by key");
    }
}
