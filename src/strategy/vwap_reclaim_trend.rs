//! VwapReclaimTrendV1 — dedicated VWAP reclaim trend research strategy.
//!
//! This is the dedicated strategy form of the VWAP-only ETP preset that showed
//! the best gross behavior so far. It intentionally avoids EMA21 as a pullback
//! anchor and caps extreme expected-edge / volatility-style setups that were
//! concentrated in the losing `edge_gte_50` bucket.
//!
//! Research only. No orders, no exchange calls, no LLMs, no auto-tuning.

use crate::config::EtpConfig;
use crate::core::{NorthflowError, Side, Signal, SignalId, StrategyId};
use crate::strategy::traits::{MultiTimeframeInput, Strategy, StrategyContext};

const STRATEGY_ID: &str = "vwap_reclaim_trend_v1";
const MAX_EXPECTED_NET_EDGE_BPS: f64 = 50.0;

#[derive(Debug, Clone)]
pub struct VwapReclaimTrendV1 {
    pub cfg: EtpConfig,
}

impl VwapReclaimTrendV1 {
    pub fn new(mut cfg: EtpConfig) -> Self {
        // Force VWAP-only behavior for this dedicated strategy. Keep the rest of
        // the knobs compatible with the existing etp_* config fields so research
        // presets can be compared without adding a second config namespace yet.
        cfg.pullback_to = "vwap".to_string();
        Self { cfg }
    }
}

impl Strategy for VwapReclaimTrendV1 {
    fn strategy_id(&self) -> &'static str {
        STRATEGY_ID
    }

    fn evaluate(
        &self,
        ctx: &StrategyContext,
        input: &MultiTimeframeInput,
    ) -> Result<Option<Signal>, NorthflowError> {
        input.entry_candle.validate()?;
        input.confirmation_candle.validate()?;
        input.screening_candle.validate()?;

        let entry = input.entry_candle;
        let confirmation = input.confirmation_candle;
        let screening = input.screening_candle;

        let ema_8 = match input.entry_indicators.ema_8 {
            Some(v) => v,
            None => return Ok(None),
        };
        let ema_21 = match input.entry_indicators.ema_21 {
            Some(v) => v,
            None => return Ok(None),
        };
        let ema_50 = match input.entry_indicators.ema_50 {
            Some(v) => v,
            None => return Ok(None),
        };
        let atr = match input.entry_indicators.atr_14 {
            Some(v) => v,
            None => return Ok(None),
        };
        let vwap = match input.entry_indicators.vwap {
            Some(v) => v,
            None => return Ok(None),
        };
        let volume_sma_20 = match input.entry_indicators.volume_sma_20 {
            Some(v) => v,
            None => return Ok(None),
        };

        let ema_21_conf = match input.confirmation_indicators.ema_21 {
            Some(v) => v,
            None => return Ok(None),
        };
        let ema_50_conf = match input.confirmation_indicators.ema_50 {
            Some(v) => v,
            None => return Ok(None),
        };
        let ema_200_conf = match input.confirmation_indicators.ema_200 {
            Some(v) => v,
            None => return Ok(None),
        };
        let ema_50_screen = match input.screening_indicators.ema_50 {
            Some(v) => v,
            None => return Ok(None),
        };
        let ema_200_screen = match input.screening_indicators.ema_200 {
            Some(v) => v,
            None => return Ok(None),
        };

        let close = entry.close;
        if close <= 0.0 || atr <= 0.0 || volume_sma_20 <= 0.0 {
            return Ok(None);
        }

        let bullish_screen = ema_50_screen > ema_200_screen && screening.close > ema_50_screen;
        let bearish_screen = ema_50_screen < ema_200_screen && screening.close < ema_50_screen;
        if !bullish_screen && !bearish_screen {
            return Ok(None);
        }

        let side = if bullish_screen { Side::Long } else { Side::Short };
        match side {
            Side::Long if !self.cfg.allow_long => return Ok(None),
            Side::Short if !self.cfg.allow_short => return Ok(None),
            _ => {}
        }

        let confirmation_ok = match side {
            Side::Long => {
                ema_21_conf > ema_50_conf && ema_50_conf > ema_200_conf && confirmation.close > ema_21_conf
            }
            Side::Short => {
                ema_21_conf < ema_50_conf && ema_50_conf < ema_200_conf && confirmation.close < ema_21_conf
            }
        };
        if !confirmation_ok {
            return Ok(None);
        }

        let entry_alignment_ok = match side {
            Side::Long => ema_8 > ema_21 && ema_21 > ema_50 && close >= ema_21,
            Side::Short => ema_8 < ema_21 && ema_21 < ema_50 && close <= ema_21,
        };
        if self.cfg.require_entry_ema_alignment && !entry_alignment_ok {
            return Ok(None);
        }

        let distance_atr = (close - vwap).abs() / atr;
        if distance_atr < self.cfg.min_pullback_distance_atr
            || distance_atr > self.cfg.max_pullback_distance_atr
        {
            return Ok(None);
        }

        let range = entry.high - entry.low;
        if range <= 0.0 {
            return Ok(None);
        }
        let body = (close - entry.open).abs();
        let body_ratio = body / range;
        let upper_wick = entry.high - entry.open.max(close);
        let lower_wick = entry.open.min(close) - entry.low;
        let upper_wick_ratio = upper_wick / range;
        let lower_wick_ratio = lower_wick / range;

        let close_reclaim = match side {
            Side::Long => entry.low <= vwap && close > vwap && close > entry.open,
            Side::Short => entry.high >= vwap && close < vwap && close < entry.open,
        };
        let wick_rejection = match side {
            Side::Long => {
                lower_wick_ratio >= self.cfg.min_wick_rejection_ratio
                    && body_ratio >= self.cfg.min_body_ratio
                    && close > entry.open
            }
            Side::Short => {
                upper_wick_ratio >= self.cfg.min_wick_rejection_ratio
                    && body_ratio >= self.cfg.min_body_ratio
                    && close < entry.open
            }
        };
        if !(close_reclaim || wick_rejection) {
            return Ok(None);
        }

        let atr_bps = atr / close * 10_000.0;
        if atr_bps < self.cfg.min_atr_bps || atr_bps > self.cfg.max_atr_bps {
            return Ok(None);
        }

        let volume_ratio = entry.volume / volume_sma_20;
        if volume_ratio < self.cfg.min_volume_ratio {
            return Ok(None);
        }

        let signal_entry = close;
        let (stop_loss, take_profit) = match side {
            Side::Long => (
                signal_entry - atr * self.cfg.sl_atr_multiple,
                signal_entry + atr * self.cfg.tp_atr_multiple,
            ),
            Side::Short => (
                signal_entry + atr * self.cfg.sl_atr_multiple,
                signal_entry - atr * self.cfg.tp_atr_multiple,
            ),
        };

        let risk = (signal_entry - stop_loss).abs();
        if risk <= 0.0 {
            return Ok(None);
        }
        let rr = (take_profit - signal_entry).abs() / risk;
        if rr < self.cfg.min_reward_risk {
            return Ok(None);
        }

        let expected_reward_bps = match side {
            Side::Long => (take_profit - signal_entry) / signal_entry * 10_000.0,
            Side::Short => (signal_entry - take_profit) / signal_entry * 10_000.0,
        };
        let expected_net_edge_bps = expected_reward_bps - ctx.estimated_cost_bps;
        if expected_reward_bps < self.cfg.min_expected_reward_bps {
            return Ok(None);
        }
        if expected_net_edge_bps < self.cfg.min_expected_net_edge_bps {
            return Ok(None);
        }
        // Dedicated VWAP research cap: prior diagnostics showed the edge_gte_50
        // bucket was the main loser. Cap this until a better volatility model exists.
        if expected_net_edge_bps > MAX_EXPECTED_NET_EDGE_BPS {
            return Ok(None);
        }

        let mut confidence = 55_i32;
        confidence += 10; // screening trend
        confidence += 10; // confirmation trend
        if entry_alignment_ok {
            confidence += 5;
        }
        if close_reclaim {
            confidence += 10;
        }
        if wick_rejection {
            confidence += 5;
        }
        confidence += 5; // ATR and volume
        let confidence = confidence.clamp(0, 100) as u8;
        if confidence < ctx.min_confidence {
            return Ok(None);
        }

        let mut filters_passed = Vec::new();
        match side {
            Side::Long => filters_passed.push("15m_trend_bullish".to_string()),
            Side::Short => filters_passed.push("15m_trend_bearish".to_string()),
        }
        match side {
            Side::Long => filters_passed.push("5m_confirmation_bullish".to_string()),
            Side::Short => filters_passed.push("5m_confirmation_bearish".to_string()),
        }
        if entry_alignment_ok {
            match side {
                Side::Long => filters_passed.push("1m_ema_alignment_long".to_string()),
                Side::Short => filters_passed.push("1m_ema_alignment_short".to_string()),
            }
        }
        filters_passed.push("pullback_near_vwap".to_string());
        if close_reclaim {
            match side {
                Side::Long => filters_passed.push("vwap_close_reclaim_long".to_string()),
                Side::Short => filters_passed.push("vwap_close_reclaim_short".to_string()),
            }
        }
        if wick_rejection {
            match side {
                Side::Long => filters_passed.push("vwap_wick_rejection_long".to_string()),
                Side::Short => filters_passed.push("vwap_wick_rejection_short".to_string()),
            }
        }
        filters_passed.push("atr_bps_in_range".to_string());
        filters_passed.push("volume_ratio_ok".to_string());
        filters_passed.push("reward_risk_ok".to_string());
        filters_passed.push("expected_reward_ok".to_string());
        filters_passed.push("expected_net_edge_min_ok".to_string());
        filters_passed.push("expected_net_edge_max_ok".to_string());
        filters_passed.push("confidence_ok".to_string());

        let regime = if bullish_screen { "bullish" } else { "bearish" }.to_string();
        let trigger = if close_reclaim { "close_reclaim" } else { "wick_rejection" };
        let entry_reason = format!("vwap_reclaim_trend_{}_{}", regime, trigger);
        let signal_id = SignalId::new(format!("SIG-BT-{:08X}", ctx.signal_index));

        Ok(Some(Signal {
            signal_id,
            symbol: ctx.symbol.clone(),
            strategy_id: StrategyId::new(STRATEGY_ID),
            side,
            entry_timeframe: ctx.entry_timeframe,
            screening_timeframe: ctx.screening_timeframe,
            confirmation_timeframe: ctx.confirmation_timeframe,
            entry_time: entry.timestamp,
            entry_price: signal_entry,
            stop_loss,
            take_profit,
            confidence,
            regime,
            entry_reason,
            filters_passed,
            filters_failed: vec![],
            expected_reward_bps,
            estimated_cost_bps: ctx.estimated_cost_bps,
            expected_net_edge_bps,
        }))
    }
}
