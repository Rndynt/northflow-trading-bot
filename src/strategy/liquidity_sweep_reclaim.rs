//! LiquiditySweepReclaimV1 — event-based liquidity sweep failure strategy.
//!
//! Hypothesis:
//! BTC often sweeps local highs/lows, takes liquidity, then quickly reclaims the
//! swept level when follow-through fails. This strategy only emits a signal after
//! a sweep + reclaim event, not from generic indicator alignment.
//!
//! Research-only. Emits Signal only. No orders, no exchange calls, no LLMs.

use crate::core::{NorthflowError, Side, Signal, SignalId, StrategyId};
use crate::strategy::traits::{MultiTimeframeInput, Strategy, StrategyContext};

#[derive(Debug, Clone)]
pub struct LiquiditySweepReclaimV1;

#[derive(Debug, Clone)]
struct SweepCandidate {
    side: Side,
    level: f64,
    sweep_price: f64,
    sweep_depth_bps: f64,
    wick_ratio: f64,
    close_location: f64,
    volume_ratio: f64,
}

impl Default for LiquiditySweepReclaimV1 {
    fn default() -> Self {
        Self
    }
}

impl Strategy for LiquiditySweepReclaimV1 {
    fn strategy_id(&self) -> &'static str {
        "liquidity_sweep_reclaim_v1"
    }

    fn evaluate(
        &self,
        ctx: &StrategyContext,
        input: &MultiTimeframeInput,
    ) -> Result<Option<Signal>, NorthflowError> {
        input.entry_candle.validate()?;
        input.confirmation_candle.validate()?;
        input.screening_candle.validate()?;

        let atr = match input.entry_indicators.atr_14 {
            Some(v) if v > 0.0 => v,
            _ => return Ok(None),
        };
        let volume_sma_20 = match input.entry_indicators.volume_sma_20 {
            Some(v) if v > 0.0 => v,
            _ => return Ok(None),
        };
        let ema_21_conf = input.confirmation_indicators.ema_21;
        let ema_50_conf = input.confirmation_indicators.ema_50;
        let ema_50_screen = input.screening_indicators.ema_50;
        let ema_200_screen = input.screening_indicators.ema_200;

        let candle = input.entry_candle;
        if candle.close <= 0.0 {
            return Ok(None);
        }

        let atr_bps = atr / candle.close * 10_000.0;
        if !(8.0..=350.0).contains(&atr_bps) {
            return Ok(None);
        }

        let lookback_len = input.entry_lookback.len();
        if lookback_len < 30 {
            return Ok(None);
        }
        let start = lookback_len.saturating_sub(80);
        let lookback = &input.entry_lookback[start..];

        let swing_low = lookback.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
        let swing_high = lookback
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);

        let volume_ratio = candle.volume / volume_sma_20;
        if volume_ratio < 1.10 {
            return Ok(None);
        }

        let long_candidate = build_long_candidate(swing_low, candle, volume_ratio);
        let short_candidate = build_short_candidate(swing_high, candle, volume_ratio);

        let mut candidates: Vec<SweepCandidate> = Vec::new();
        if let Some(c) = long_candidate {
            if !higher_tf_blocks_long(
                input,
                ema_21_conf,
                ema_50_conf,
                ema_50_screen,
                ema_200_screen,
            ) {
                candidates.push(c);
            }
        }
        if let Some(c) = short_candidate {
            if !higher_tf_blocks_short(
                input,
                ema_21_conf,
                ema_50_conf,
                ema_50_screen,
                ema_200_screen,
            ) {
                candidates.push(c);
            }
        }

        let candidate = match candidates.into_iter().max_by(|a, b| {
            a.sweep_depth_bps
                .partial_cmp(&b.sweep_depth_bps)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            Some(c) => c,
            None => return Ok(None),
        };

        let entry = candle.close;
        let stop_buffer = atr * 0.10;
        let rr_target = 1.8;
        let (stop_loss, take_profit) = match candidate.side {
            Side::Long => {
                let stop = candidate.sweep_price - stop_buffer;
                let risk = entry - stop;
                if risk <= 0.0 {
                    return Ok(None);
                }
                (stop, entry + risk * rr_target)
            }
            Side::Short => {
                let stop = candidate.sweep_price + stop_buffer;
                let risk = stop - entry;
                if risk <= 0.0 {
                    return Ok(None);
                }
                (stop, entry - risk * rr_target)
            }
        };

        let risk = (entry - stop_loss).abs();
        let reward = (take_profit - entry).abs();
        if risk <= 0.0 {
            return Ok(None);
        }
        let rr = reward / risk;
        if rr < 1.5 {
            return Ok(None);
        }

        let expected_reward_bps = reward / entry * 10_000.0;
        let expected_net_edge_bps = expected_reward_bps - ctx.estimated_cost_bps;
        if expected_reward_bps < 35.0 || expected_net_edge_bps < 15.0 {
            return Ok(None);
        }

        let mut confidence: i32 = 55;
        confidence += 10; // sweep + reclaim event
        confidence += 10; // volume expansion
        if candidate.wick_ratio >= 0.45 {
            confidence += 8;
        }
        if candidate.close_location >= 0.70 {
            confidence += 7;
        }
        if expected_net_edge_bps >= 50.0 {
            confidence += 10;
        }
        let confidence = confidence.clamp(0, 100) as u8;
        if confidence < ctx.min_confidence {
            return Ok(None);
        }

        let side_label = candidate.side.as_str();
        let regime = match candidate.side {
            Side::Long => "sweep_reclaim_long".to_string(),
            Side::Short => "sweep_reclaim_short".to_string(),
        };
        let entry_reason = format!(
            "lsr_v1_{} level={:.6}, sweep_depth_bps={:.2}, wick={:.2}, close_loc={:.2}, vol={:.2}",
            side_label,
            candidate.level,
            candidate.sweep_depth_bps,
            candidate.wick_ratio,
            candidate.close_location,
            candidate.volume_ratio
        );

        let signal_id = SignalId::new(format!("SIG-BT-{:08X}", ctx.signal_index));
        Ok(Some(Signal {
            signal_id,
            symbol: ctx.symbol.clone(),
            strategy_id: StrategyId::new("liquidity_sweep_reclaim_v1"),
            side: candidate.side,
            entry_timeframe: ctx.entry_timeframe,
            screening_timeframe: ctx.screening_timeframe,
            confirmation_timeframe: ctx.confirmation_timeframe,
            entry_time: candle.timestamp,
            entry_price: entry,
            stop_loss,
            take_profit,
            confidence,
            regime,
            entry_reason,
            filters_passed: vec![
                "liquidity_sweep".to_string(),
                "level_reclaim".to_string(),
                "wick_rejection".to_string(),
                "volume_expansion".to_string(),
                "higher_tf_not_blocking".to_string(),
                "reward_risk_ok".to_string(),
                "expected_edge_ok".to_string(),
            ],
            filters_failed: vec![],
            expected_reward_bps,
            estimated_cost_bps: ctx.estimated_cost_bps,
            expected_net_edge_bps,
        }))
    }
}

fn build_long_candidate(
    swing_low: f64,
    candle: crate::core::Candle,
    volume_ratio: f64,
) -> Option<SweepCandidate> {
    if !swing_low.is_finite() || swing_low <= 0.0 {
        return None;
    }
    if candle.low >= swing_low || candle.close <= swing_low {
        return None;
    }
    let range = candle.high - candle.low;
    if range <= 0.0 {
        return None;
    }
    let sweep_depth_bps = (swing_low - candle.low) / swing_low * 10_000.0;
    if !(2.0..=120.0).contains(&sweep_depth_bps) {
        return None;
    }
    let lower_wick = candle.open.min(candle.close) - candle.low;
    let wick_ratio = lower_wick / range;
    let close_location = (candle.close - candle.low) / range;
    if wick_ratio < 0.35 || close_location < 0.60 {
        return None;
    }
    Some(SweepCandidate {
        side: Side::Long,
        level: swing_low,
        sweep_price: candle.low,
        sweep_depth_bps,
        wick_ratio,
        close_location,
        volume_ratio,
    })
}

fn build_short_candidate(
    swing_high: f64,
    candle: crate::core::Candle,
    volume_ratio: f64,
) -> Option<SweepCandidate> {
    if !swing_high.is_finite() || swing_high <= 0.0 {
        return None;
    }
    if candle.high <= swing_high || candle.close >= swing_high {
        return None;
    }
    let range = candle.high - candle.low;
    if range <= 0.0 {
        return None;
    }
    let sweep_depth_bps = (candle.high - swing_high) / swing_high * 10_000.0;
    if !(2.0..=120.0).contains(&sweep_depth_bps) {
        return None;
    }
    let upper_wick = candle.high - candle.open.max(candle.close);
    let wick_ratio = upper_wick / range;
    let close_location = (candle.high - candle.close) / range;
    if wick_ratio < 0.35 || close_location < 0.60 {
        return None;
    }
    Some(SweepCandidate {
        side: Side::Short,
        level: swing_high,
        sweep_price: candle.high,
        sweep_depth_bps,
        wick_ratio,
        close_location,
        volume_ratio,
    })
}

fn higher_tf_blocks_long(
    input: &MultiTimeframeInput,
    ema_21_conf: Option<f64>,
    ema_50_conf: Option<f64>,
    ema_50_screen: Option<f64>,
    ema_200_screen: Option<f64>,
) -> bool {
    let conf_block = match (ema_21_conf, ema_50_conf) {
        (Some(e21), Some(e50)) => e21 < e50 && input.confirmation_candle.close < e21,
        _ => false,
    };
    let screen_block = match (ema_50_screen, ema_200_screen) {
        (Some(e50), Some(e200)) => e50 < e200 && input.screening_candle.close < e200,
        _ => false,
    };
    conf_block && screen_block
}

fn higher_tf_blocks_short(
    input: &MultiTimeframeInput,
    ema_21_conf: Option<f64>,
    ema_50_conf: Option<f64>,
    ema_50_screen: Option<f64>,
    ema_200_screen: Option<f64>,
) -> bool {
    let conf_block = match (ema_21_conf, ema_50_conf) {
        (Some(e21), Some(e50)) => e21 > e50 && input.confirmation_candle.close > e21,
        _ => false,
    };
    let screen_block = match (ema_50_screen, ema_200_screen) {
        (Some(e50), Some(e200)) => e50 > e200 && input.screening_candle.close > e200,
        _ => false,
    };
    conf_block && screen_block
}
