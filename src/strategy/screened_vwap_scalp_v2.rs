//! ScreenedVwapScalpV2 — cost-aware multi-timeframe scalp strategy.
//!
//! This variant is intentionally stricter than the old EMA-ribbon-only trigger.
//! A signal now requires regime alignment, EMA alignment, VWAP/EMA21 location,
//! volume, expected edge, and a real price-action trigger candle.

use crate::config::V2Config;
use crate::core::{NorthflowError, Side, Signal, SignalId, StrategyId};
use crate::strategy::regime::{classify_screening_regime, MarketRegime};
use crate::strategy::traits::{MultiTimeframeInput, Strategy, StrategyContext};

const MIN_TRIGGER_BODY_RATIO: f64 = 0.25;
const MAX_RECLAIM_DISTANCE_ATR: f64 = 0.35;

#[derive(Debug, Clone)]
pub struct ScreenedVwapScalpV2 {
    pub cfg: V2Config,
}

impl ScreenedVwapScalpV2 {
    pub fn new(cfg: V2Config) -> Self {
        Self { cfg }
    }
}

impl Strategy for ScreenedVwapScalpV2 {
    fn strategy_id(&self) -> &'static str {
        "screened_vwap_scalp_v2"
    }

    fn evaluate(
        &self,
        ctx: &StrategyContext,
        input: &MultiTimeframeInput,
    ) -> Result<Option<Signal>, NorthflowError> {
        input.entry_candle.validate()?;
        input.confirmation_candle.validate()?;
        input.screening_candle.validate()?;

        let ema_8 = required(input.entry_indicators.ema_8)?;
        let ema_21 = required(input.entry_indicators.ema_21)?;
        let ema_50_entry = required(input.entry_indicators.ema_50)?;
        let atr = required(input.entry_indicators.atr_14)?;
        let vwap = required(input.entry_indicators.vwap)?;
        let volume_sma_20 = required(input.entry_indicators.volume_sma_20)?;

        if !ema_8.is_finite()
            || !ema_21.is_finite()
            || !ema_50_entry.is_finite()
            || !atr.is_finite()
            || !vwap.is_finite()
            || !volume_sma_20.is_finite()
        {
            return Ok(None);
        }

        let candle = input.entry_candle;
        let close = candle.close;
        if atr <= 0.0 || close <= 0.0 || volume_sma_20 <= 0.0 {
            return Ok(None);
        }

        let screening_regime =
            classify_screening_regime(input.screening_candle, &input.screening_indicators);
        let confirmation_regime =
            classify_screening_regime(input.confirmation_candle, &input.confirmation_indicators);

        let side = match screening_regime {
            MarketRegime::Bullish => Side::Long,
            MarketRegime::Bearish => Side::Short,
            _ => return Ok(None),
        };

        match side {
            Side::Long if !self.cfg.enable_long => return Ok(None),
            Side::Short if !self.cfg.enable_short => return Ok(None),
            _ => {}
        }

        if !confirmation_passes(side, confirmation_regime, &self.cfg) {
            return Ok(None);
        }

        if self.cfg.require_ema_ribbon_alignment
            && !ema_ribbon_passes(side, ema_8, ema_21, ema_50_entry, close)
        {
            return Ok(None);
        }

        let atr_bps = atr / close * 10_000.0;
        if atr_bps < self.cfg.min_atr_bps || atr_bps > self.cfg.max_atr_bps {
            return Ok(None);
        }

        let distance_to_vwap = (close - vwap).abs();
        let distance_to_ema21 = (close - ema_21).abs();
        let nearest_distance = distance_to_vwap.min(distance_to_ema21);
        let distance_atr = nearest_distance / atr;
        if distance_atr < self.cfg.vwap_distance_atr_min
            || distance_atr > self.cfg.vwap_distance_atr_max
        {
            return Ok(None);
        }

        let volume_ratio = candle.volume / volume_sma_20;
        if volume_ratio < self.cfg.min_volume_ratio {
            return Ok(None);
        }

        let trigger = price_action_trigger(side, candle, ema_21, vwap, atr);
        if !trigger.passes {
            return Ok(None);
        }

        let (stop_loss, take_profit) = match side {
            Side::Long => (
                close - atr * self.cfg.sl_atr_multiple,
                close + atr * self.cfg.tp_atr_multiple,
            ),
            Side::Short => (
                close + atr * self.cfg.sl_atr_multiple,
                close - atr * self.cfg.tp_atr_multiple,
            ),
        };

        let expected_reward_bps = (take_profit - close).abs() / close * 10_000.0;
        let estimated_cost_bps = ctx.estimated_cost_bps;
        let expected_net_edge_bps = expected_reward_bps - estimated_cost_bps;

        if expected_reward_bps < self.cfg.min_expected_reward_bps {
            return Ok(None);
        }
        if expected_net_edge_bps < self.cfg.min_expected_net_edge_bps {
            return Ok(None);
        }

        let mut confidence: i16 = 50;
        confidence += 10;
        confidence += 10;
        confidence += 10;
        confidence += 10;
        confidence += 10;
        let confidence = confidence.clamp(0, 100) as u8;

        if confidence < ctx.min_confidence {
            return Ok(None);
        }

        let mut filters_passed = filters_for_side(side, self.cfg.require_ema_ribbon_alignment);
        filters_passed.push("atr_bps_in_range".to_string());
        filters_passed.push("near_vwap_or_ema21".to_string());
        filters_passed.push("volume_ratio_ok".to_string());
        filters_passed.push("price_action_trigger_ok".to_string());
        filters_passed.push("trigger_body_ratio_ok".to_string());
        filters_passed.push("expected_reward_ok".to_string());
        filters_passed.push("expected_net_edge_ok".to_string());
        filters_passed.push("direction_enabled".to_string());
        filters_passed.push("confidence_ok".to_string());

        let entry_reason = match side {
            Side::Long => format!(
                "bullish regime, bullish entry trigger, close above VWAP/EMA21, body_ratio={:.2}, reclaim_distance_atr={:.2}, volume_ratio={:.2}, atr_bps={:.1}",
                trigger.body_ratio, trigger.reclaim_distance_atr, volume_ratio, atr_bps
            ),
            Side::Short => format!(
                "bearish regime, bearish entry trigger, close below VWAP/EMA21, body_ratio={:.2}, reject_distance_atr={:.2}, volume_ratio={:.2}, atr_bps={:.1}",
                trigger.body_ratio, trigger.reclaim_distance_atr, volume_ratio, atr_bps
            ),
        };

        let signal = Signal {
            signal_id: make_signal_id(ctx.signal_index),
            symbol: ctx.symbol.clone(),
            strategy_id: StrategyId::new(self.strategy_id()),
            side,
            entry_timeframe: ctx.entry_timeframe,
            screening_timeframe: ctx.screening_timeframe,
            confirmation_timeframe: ctx.confirmation_timeframe,
            entry_time: candle.timestamp,
            entry_price: close,
            stop_loss,
            take_profit,
            confidence,
            regime: screening_regime.as_str().to_string(),
            entry_reason,
            filters_passed,
            filters_failed: vec![],
            expected_reward_bps,
            estimated_cost_bps,
            expected_net_edge_bps,
        };

        signal.validate()?;
        Ok(Some(signal))
    }
}

#[derive(Debug, Clone, Copy)]
struct TriggerCheck {
    passes: bool,
    body_ratio: f64,
    reclaim_distance_atr: f64,
}

fn required(value: Option<f64>) -> Result<f64, NorthflowError> {
    Ok(value.filter(|v| v.is_finite()).unwrap_or(f64::NAN))
}

fn confirmation_passes(side: Side, confirmation_regime: MarketRegime, cfg: &V2Config) -> bool {
    match side {
        Side::Long => {
            confirmation_regime == MarketRegime::Bullish
                || (!cfg.require_strict_confirmation
                    && cfg.allow_neutral_confirmation
                    && confirmation_regime == MarketRegime::Neutral)
        }
        Side::Short => {
            confirmation_regime == MarketRegime::Bearish
                || (!cfg.require_strict_confirmation
                    && cfg.allow_neutral_confirmation
                    && confirmation_regime == MarketRegime::Neutral)
        }
    }
}

fn ema_ribbon_passes(side: Side, ema_8: f64, ema_21: f64, ema_50: f64, close: f64) -> bool {
    match side {
        Side::Long => ema_8 > ema_21 && ema_21 > ema_50 && close > ema_21,
        Side::Short => ema_8 < ema_21 && ema_21 < ema_50 && close < ema_21,
    }
}

fn price_action_trigger(
    side: Side,
    candle: crate::core::Candle,
    ema_21: f64,
    vwap: f64,
    atr: f64,
) -> TriggerCheck {
    let range = candle.high - candle.low;
    let body_ratio = if range > 0.0 {
        (candle.close - candle.open).abs() / range
    } else {
        0.0
    };

    if body_ratio < MIN_TRIGGER_BODY_RATIO || atr <= 0.0 {
        return TriggerCheck {
            passes: false,
            body_ratio,
            reclaim_distance_atr: f64::INFINITY,
        };
    }

    match side {
        Side::Long => {
            let trigger_level = ema_21.max(vwap);
            let touched_level = candle.low <= trigger_level + MAX_RECLAIM_DISTANCE_ATR * atr;
            let reclaimed_level = candle.close > trigger_level;
            let bullish_body = candle.close > candle.open;
            TriggerCheck {
                passes: bullish_body && touched_level && reclaimed_level,
                body_ratio,
                reclaim_distance_atr: ((candle.low - trigger_level).max(0.0)) / atr,
            }
        }
        Side::Short => {
            let trigger_level = ema_21.min(vwap);
            let touched_level = candle.high >= trigger_level - MAX_RECLAIM_DISTANCE_ATR * atr;
            let rejected_level = candle.close < trigger_level;
            let bearish_body = candle.close < candle.open;
            TriggerCheck {
                passes: bearish_body && touched_level && rejected_level,
                body_ratio,
                reclaim_distance_atr: ((trigger_level - candle.high).max(0.0)) / atr,
            }
        }
    }
}

fn filters_for_side(side: Side, include_ribbon: bool) -> Vec<String> {
    let mut filters = Vec::new();
    match side {
        Side::Long => {
            filters.push("screening_bullish".to_string());
            filters.push("confirmation_bullish_or_allowed_neutral".to_string());
            if include_ribbon {
                filters.push("ema_ribbon_long".to_string());
            }
        }
        Side::Short => {
            filters.push("screening_bearish".to_string());
            filters.push("confirmation_bearish_or_allowed_neutral".to_string());
            if include_ribbon {
                filters.push("ema_ribbon_short".to_string());
            }
        }
    }
    filters
}

fn make_signal_id(index: u64) -> SignalId {
    SignalId::new(format!("SIG-BT-{index:08}"))
}
