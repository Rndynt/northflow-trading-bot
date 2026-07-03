//! Basic sample strategy — the only active production strategy.
//!
//! This intentionally simple reference implementation demonstrates how a
//! strategy reads prepared candles and indicator snapshots and emits a `Signal`.
//! It does not place orders, size positions, call external services, or mutate
//! account state.

use crate::core::{NorthflowError, Side, Signal, SignalId, StrategyId};
use crate::market::classify_basic_regime;
use crate::strategy::ids::BASIC_SAMPLE_STRATEGY_ID;
use crate::strategy::traits::{MultiTimeframeInput, Strategy, StrategyContext};

#[derive(Debug, Clone)]
pub struct BasicSampleStrategy;

impl Default for BasicSampleStrategy {
    fn default() -> Self {
        Self
    }
}

impl Strategy for BasicSampleStrategy {
    fn strategy_id(&self) -> &'static str {
        BASIC_SAMPLE_STRATEGY_ID
    }

    fn evaluate(
        &self,
        ctx: &StrategyContext,
        input: &MultiTimeframeInput,
    ) -> Result<Option<Signal>, NorthflowError> {
        input.entry_candle.validate()?;
        input.confirmation_candle.validate()?;
        input.screening_candle.validate()?;

        let Some(entry_vwap) = input.entry_indicators.vwap else {
            return Ok(None);
        };
        let Some(entry_ema_8) = input.entry_indicators.ema_8 else {
            return Ok(None);
        };
        let Some(entry_ema_21) = input.entry_indicators.ema_21 else {
            return Ok(None);
        };
        let Some(atr) = input.entry_indicators.atr_14 else {
            return Ok(None);
        };
        let Some(confirmation_vwap) = input.confirmation_indicators.vwap else {
            return Ok(None);
        };
        let Some(screening_ema_50) = input.screening_indicators.ema_50 else {
            return Ok(None);
        };

        if !atr.is_finite() || atr <= 0.0 {
            return Ok(None);
        }

        let long_setup = input.entry_candle.close > entry_vwap
            && entry_ema_8 > entry_ema_21
            && input.confirmation_candle.close > confirmation_vwap
            && input.screening_candle.close > screening_ema_50;
        let short_setup = input.entry_candle.close < entry_vwap
            && entry_ema_8 < entry_ema_21
            && input.confirmation_candle.close < confirmation_vwap
            && input.screening_candle.close < screening_ema_50;

        let side = if long_setup {
            Side::Long
        } else if short_setup {
            Side::Short
        } else {
            return Ok(None);
        };

        let entry = input.entry_candle.close;
        let regime = classify_basic_regime(
            input.screening_candle.close,
            input.screening_indicators.vwap,
            input.screening_indicators.ema_50,
        );
        let (stop_loss, take_profit, entry_reason, filters_passed) = match side {
            Side::Long => (
                entry - atr,
                entry + atr * 1.5,
                "sample long: entry above VWAP with EMA and higher-timeframe alignment",
                vec![
                    "entry_close_above_entry_vwap",
                    "entry_ema8_above_entry_ema21",
                    "confirmation_close_above_vwap",
                    "screening_close_above_ema50",
                    "atr_positive_finite",
                ],
            ),
            Side::Short => (
                entry + atr,
                entry - atr * 1.5,
                "sample short: entry below VWAP with EMA and higher-timeframe alignment",
                vec![
                    "entry_close_below_entry_vwap",
                    "entry_ema8_below_entry_ema21",
                    "confirmation_close_below_vwap",
                    "screening_close_below_ema50",
                    "atr_positive_finite",
                ],
            ),
        };

        let expected_reward_bps = (take_profit - entry).abs() / entry * 10_000.0;
        let estimated_cost_bps = ctx.estimated_cost_bps;
        let confidence = ctx.min_confidence.max(70).min(100);

        let signal = Signal {
            signal_id: SignalId::new(format!("SIG-BT-{idx:08}", idx = ctx.signal_index)),
            symbol: ctx.symbol.clone(),
            strategy_id: StrategyId::new(self.strategy_id()),
            side,
            entry_timeframe: ctx.entry_timeframe,
            screening_timeframe: ctx.screening_timeframe,
            confirmation_timeframe: ctx.confirmation_timeframe,
            entry_time: input.entry_candle.timestamp,
            entry_price: entry,
            stop_loss,
            take_profit,
            confidence,
            regime: regime.as_str().to_string(),
            entry_reason: entry_reason.to_string(),
            filters_passed: filters_passed.into_iter().map(str::to_string).collect(),
            filters_failed: vec![],
            expected_reward_bps,
            estimated_cost_bps,
            expected_net_edge_bps: expected_reward_bps - estimated_cost_bps,
        };

        signal.validate()?;
        Ok(Some(signal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Candle, Symbol, Timeframe};
    use crate::indicators::IndicatorSnapshot;

    fn candle(close: f64) -> Candle {
        Candle {
            timestamp: 1_700_000_000,
            open: close,
            high: close + 2.0,
            low: close - 2.0,
            close,
            volume: 100.0,
        }
    }

    fn ctx() -> StrategyContext {
        StrategyContext {
            symbol: Symbol::new("BTCUSDT").unwrap(),
            signal_index: 1,
            estimated_cost_bps: 9.0,
            min_confidence: 65,
            entry_timeframe: Timeframe::OneMinute,
            confirmation_timeframe: Timeframe::FiveMinute,
            screening_timeframe: Timeframe::FifteenMinute,
        }
    }

    fn input(entry_close: f64) -> MultiTimeframeInput {
        MultiTimeframeInput {
            entry_candle: candle(entry_close),
            entry_lookback: vec![],
            confirmation_candle: candle(entry_close),
            screening_candle: candle(entry_close),
            entry_indicators: IndicatorSnapshot {
                ema_8: Some(101.0),
                ema_21: Some(100.0),
                atr_14: Some(2.0),
                vwap: Some(100.0),
                ..Default::default()
            },
            confirmation_indicators: IndicatorSnapshot {
                vwap: Some(100.0),
                ..Default::default()
            },
            screening_indicators: IndicatorSnapshot {
                ema_50: Some(100.0),
                ..Default::default()
            },
        }
    }

    #[test]
    fn strategy_id_is_basic_sample_strategy() {
        assert_eq!(BasicSampleStrategy.strategy_id(), "basic_sample_strategy");
    }

    #[test]
    fn emits_long_signal_on_clear_long_input() {
        let sig = BasicSampleStrategy
            .evaluate(&ctx(), &input(102.0))
            .unwrap()
            .unwrap();
        assert_eq!(sig.side, Side::Long);
        assert_eq!(sig.signal_id.as_str(), "SIG-BT-00000001");
        assert_eq!(sig.strategy_id.as_str(), "basic_sample_strategy");
        assert!(sig.valid_geometry());
        assert_eq!(sig.entry_timeframe, Timeframe::OneMinute);
        assert_eq!(sig.confirmation_timeframe, Timeframe::FiveMinute);
        assert_eq!(sig.screening_timeframe, Timeframe::FifteenMinute);
        assert_eq!(sig.regime, "bullish");
        assert_ne!(sig.regime, "sample_bullish");
        assert!(["bullish", "bearish", "ranging", "unknown"].contains(&sig.regime.as_str()));
    }

    #[test]
    fn emits_short_signal_on_clear_short_input() {
        let mut i = input(98.0);
        i.entry_indicators.ema_8 = Some(99.0);
        i.entry_indicators.ema_21 = Some(100.0);
        i.entry_indicators.vwap = Some(100.0);
        i.confirmation_indicators.vwap = Some(100.0);
        i.screening_indicators.ema_50 = Some(100.0);
        let sig = BasicSampleStrategy.evaluate(&ctx(), &i).unwrap().unwrap();
        assert_eq!(sig.side, Side::Short);
        assert!(sig.valid_geometry());
        assert_eq!(sig.regime, "bearish");
        assert_ne!(sig.regime, "sample_bearish");
        assert!(["bullish", "bearish", "ranging", "unknown"].contains(&sig.regime.as_str()));
    }

    #[test]
    fn returns_none_when_atr_invalid_or_no_setup() {
        let mut i = input(102.0);
        i.entry_indicators.atr_14 = Some(0.0);
        assert!(BasicSampleStrategy.evaluate(&ctx(), &i).unwrap().is_none());
        let mut i = input(100.0);
        i.entry_indicators.atr_14 = Some(f64::NAN);
        assert!(BasicSampleStrategy.evaluate(&ctx(), &i).unwrap().is_none());
        let i = input(100.0);
        assert!(BasicSampleStrategy.evaluate(&ctx(), &i).unwrap().is_none());
    }
}
