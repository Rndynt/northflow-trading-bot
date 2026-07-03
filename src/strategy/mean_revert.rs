use crate::config::V2Config;
use crate::core::{NorthflowError, Side, Signal, SignalId, StrategyId};
use crate::strategy::regime::{classify_screening_regime, MarketRegime};
use crate::strategy::traits::{MultiTimeframeInput, Strategy, StrategyContext};

const MIN_WICK: f64 = 0.35;
const MIN_LOCATION: f64 = 0.55;
const MAX_BODY: f64 = 0.70;

#[derive(Debug, Clone)]
pub struct MeanRevertV1 {
    pub cfg: V2Config,
}

impl MeanRevertV1 {
    pub fn new(cfg: V2Config) -> Self {
        Self { cfg }
    }
}

impl Strategy for MeanRevertV1 {
    fn strategy_id(&self) -> &'static str {
        "mean_revert_v1"
    }

    fn evaluate(
        &self,
        ctx: &StrategyContext,
        input: &MultiTimeframeInput,
    ) -> Result<Option<Signal>, NorthflowError> {
        input.entry_candle.validate()?;
        input.confirmation_candle.validate()?;
        input.screening_candle.validate()?;

        let Some(atr) = finite(input.entry_indicators.atr_14) else {
            return Ok(None);
        };
        let Some(vwap) = finite(input.entry_indicators.vwap) else {
            return Ok(None);
        };
        let Some(ema21) = finite(input.entry_indicators.ema_21) else {
            return Ok(None);
        };
        let Some(vol_sma) = finite(input.entry_indicators.volume_sma_20) else {
            return Ok(None);
        };

        let c = input.entry_candle;
        if c.close <= 0.0 || atr <= 0.0 || vol_sma <= 0.0 {
            return Ok(None);
        }

        let atr_bps = atr / c.close * 10_000.0;
        if atr_bps < self.cfg.min_atr_bps || atr_bps > self.cfg.max_atr_bps {
            return Ok(None);
        }

        let mean = (vwap + ema21) / 2.0;
        let ext_atr = (c.close - mean) / atr;
        let dist_atr = ext_atr.abs();

        if dist_atr < self.cfg.vwap_distance_atr_min || dist_atr > self.cfg.vwap_distance_atr_max {
            return Ok(None);
        }

        let vol_ratio = c.volume / vol_sma;
        if vol_ratio < self.cfg.min_volume_ratio {
            return Ok(None);
        }

        let side = if ext_atr > 0.0 {
            Side::Short
        } else {
            Side::Long
        };

        if matches!(side, Side::Long) && !self.cfg.enable_long {
            return Ok(None);
        }
        if matches!(side, Side::Short) && !self.cfg.enable_short {
            return Ok(None);
        }

        let screen = classify_screening_regime(input.screening_candle, &input.screening_indicators);
        let confirm =
            classify_screening_regime(input.confirmation_candle, &input.confirmation_indicators);

        if blocked_by_trend(side, screen, confirm) {
            return Ok(None);
        }

        let r = rejection(side, c);
        if !r.ok {
            return Ok(None);
        }

        let (stop_level, target_level) = match side {
            Side::Long => (c.close - atr * self.cfg.sl_atr_multiple, mean),
            Side::Short => (c.close + atr * self.cfg.sl_atr_multiple, mean),
        };

        let reward = (target_level - c.close).abs();
        let risk = (c.close - stop_level).abs();

        if reward <= 0.0 || risk <= 0.0 || reward / risk < 1.0 {
            return Ok(None);
        }

        let expected_reward_bps = reward / c.close * 10_000.0;
        let estimated_cost_bps = ctx.estimated_cost_bps;
        let expected_net_edge_bps = expected_reward_bps - estimated_cost_bps;

        if expected_reward_bps < self.cfg.min_expected_reward_bps
            || expected_net_edge_bps < self.cfg.min_expected_net_edge_bps
        {
            return Ok(None);
        }

        let confidence = 85;
        if confidence < ctx.min_confidence {
            return Ok(None);
        }

        let signal = Signal {
            signal_id: SignalId::new(format!("SIG-BT-{:08}", ctx.signal_index)),
            symbol: ctx.symbol.clone(),
            strategy_id: StrategyId::new(self.strategy_id()),
            side,
            entry_timeframe: ctx.entry_timeframe,
            screening_timeframe: ctx.screening_timeframe,
            confirmation_timeframe: ctx.confirmation_timeframe,
            entry_time: c.timestamp,
            entry_price: c.close,
            stop_loss: stop_level,
            take_profit: target_level,
            confidence,
            regime: screen.as_str().to_string(),
            entry_reason: format!(
                "mean_revert_v1 ext_atr={:.2}, wick={:.2}, close_location={:.2}, vol={:.2}, atr_bps={:.1}",
                ext_atr, r.wick, r.location, vol_ratio, atr_bps
            ),
            filters_passed: vec![
                "extension_ok".to_string(),
                "rejection_ok".to_string(),
                "trend_block_ok".to_string(),
                "expected_edge_ok".to_string(),
            ],
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
struct Reject {
    ok: bool,
    wick: f64,
    location: f64,
}

fn finite(v: Option<f64>) -> Option<f64> {
    v.filter(|x| x.is_finite())
}

fn blocked_by_trend(side: Side, screen: MarketRegime, confirm: MarketRegime) -> bool {
    match side {
        Side::Long => screen == MarketRegime::Bearish && confirm == MarketRegime::Bearish,
        Side::Short => screen == MarketRegime::Bullish && confirm == MarketRegime::Bullish,
    }
}

fn rejection(side: Side, c: crate::core::Candle) -> Reject {
    let range = c.high - c.low;
    if range <= 0.0 {
        return Reject {
            ok: false,
            wick: 0.0,
            location: 0.0,
        };
    }

    let body = (c.close - c.open).abs() / range;
    if body > MAX_BODY {
        return Reject {
            ok: false,
            wick: 0.0,
            location: 0.0,
        };
    }

    match side {
        Side::Long => {
            let wick = (c.open.min(c.close) - c.low).max(0.0) / range;
            let location = (c.close - c.low) / range;
            Reject {
                ok: wick >= MIN_WICK && location >= MIN_LOCATION,
                wick,
                location,
            }
        }
        Side::Short => {
            let wick = (c.high - c.open.max(c.close)).max(0.0) / range;
            let location = (c.high - c.close) / range;
            Reject {
                ok: wick >= MIN_WICK && location >= MIN_LOCATION,
                wick,
                location,
            }
        }
    }
}
