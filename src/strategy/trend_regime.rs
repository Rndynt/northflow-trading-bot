//! Trend-regime strategy — real, active strategy #2.
//!
//! Unlike `basic_sample_strategy` (which requires three timeframes to
//! independently align before it will look at entry at all), this strategy
//! answers the user's explicit design request directly: classify the
//! prevailing regime first, and only generate an entry signal once the
//! regime is known and tradeable.
//!
//! Order of operations:
//!   1. Classify regime from the screening timeframe (higher-timeframe
//!      context) using the same `classify_basic_regime` used elsewhere in
//!      this codebase for attribution.
//!   2. If regime is `Ranging` or `Unknown`, emit no signal at all — this
//!      strategy is trend-following and is not expected to have edge when
//!      there is no dominant trend.
//!   3. If regime is `Bullish`/`Bearish`, require entry-timeframe momentum
//!      (EMA-8 vs EMA-21) and value-area position (close vs VWAP) to agree
//!      with that regime before emitting a signal.
//!
//! Stop-loss is 2x ATR(14); take-profit is 4x ATR(14) (reward:risk 2.0, same
//! ratio as the first version of this strategy, but with wider absolute
//! distance). This tests a specific diagnostic finding from the first
//! version's backtest: avg_expected_edge_bps was positive (~22-30 bps) but
//! avg_actual_edge_bps was deeply negative (~-10 bps), an average edge
//! realization gap of -33 to -40 bps per trade — far larger than cost alone
//! (~12 bps) explains. The leading hypothesis is that a 1x-ATR stop is too
//! tight for noisy 1-minute BTCUSDT bars and gets triggered by noise before
//! the identified trend has room to develop.

use crate::core::{NorthflowError, Side, Signal, SignalId, StrategyId};
use crate::market::{classify_basic_regime, MarketRegime};
use crate::strategy::ids::TREND_REGIME_STRATEGY_ID;
use crate::strategy::traits::{MultiTimeframeInput, Strategy, StrategyContext};

#[derive(Debug, Clone)]
pub struct TrendRegimeStrategy;

impl Default for TrendRegimeStrategy {
    fn default() -> Self {
        Self
    }
}

impl Strategy for TrendRegimeStrategy {
    fn strategy_id(&self) -> &'static str {
        TREND_REGIME_STRATEGY_ID
    }

    fn evaluate(
        &self,
        ctx: &StrategyContext,
        input: &MultiTimeframeInput,
    ) -> Result<Option<Signal>, NorthflowError> {
        input.entry_candle.validate()?;
        input.confirmation_candle.validate()?;
        input.screening_candle.validate()?;

        let regime = classify_basic_regime(
            input.screening_candle.close,
            input.screening_indicators.vwap,
            input.screening_indicators.ema_50,
        );

        // Regime first: skip entirely when there is no dominant trend on the
        // higher timeframe, or when regime cannot be determined.
        let side = match regime {
            MarketRegime::Bullish => Side::Long,
            MarketRegime::Bearish => Side::Short,
            MarketRegime::Ranging | MarketRegime::Unknown => return Ok(None),
        };

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
        if !atr.is_finite() || atr <= 0.0 {
            return Ok(None);
        }

        // Entry-timeframe momentum and value-area must agree with the
        // higher-timeframe regime before a signal is emitted.
        let long_ok = side == Side::Long
            && entry_ema_8 > entry_ema_21
            && input.entry_candle.close > entry_vwap;
        let short_ok = side == Side::Short
            && entry_ema_8 < entry_ema_21
            && input.entry_candle.close < entry_vwap;

        if !long_ok && !short_ok {
            return Ok(None);
        }

        let entry = input.entry_candle.close;
        let (stop_loss, take_profit, entry_reason, filters_passed) = match side {
            Side::Long => (
                entry - atr * 2.0,
                entry + atr * 4.0,
                "trend-regime long: screening regime bullish, entry momentum and value-area confirm (wide stop)",
                vec![
                    "screening_regime_bullish",
                    "entry_ema8_above_entry_ema21",
                    "entry_close_above_entry_vwap",
                    "atr_positive_finite",
                ],
            ),
            Side::Short => (
                entry + atr * 2.0,
                entry - atr * 4.0,
                "trend-regime short: screening regime bearish, entry momentum and value-area confirm (wide stop)",
                vec![
                    "screening_regime_bearish",
                    "entry_ema8_below_entry_ema21",
                    "entry_close_below_entry_vwap",
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

    fn input(entry_close: f64, screening_close: f64) -> MultiTimeframeInput {
        MultiTimeframeInput {
            entry_candle: candle(entry_close),
            entry_lookback: vec![],
            confirmation_candle: candle(entry_close),
            screening_candle: candle(screening_close),
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
                vwap: Some(100.0),
                ema_50: Some(100.0),
                ..Default::default()
            },
        }
    }

    #[test]
    fn strategy_id_is_trend_regime_strategy() {
        assert_eq!(TrendRegimeStrategy.strategy_id(), "trend_regime_strategy");
    }

    #[test]
    fn emits_long_signal_when_regime_bullish_and_entry_confirms() {
        let mut i = input(102.0, 105.0);
        i.entry_indicators.ema_8 = Some(101.0);
        i.entry_indicators.ema_21 = Some(100.0);
        i.entry_indicators.vwap = Some(100.0);
        let sig = TrendRegimeStrategy.evaluate(&ctx(), &i).unwrap().unwrap();
        assert_eq!(sig.side, Side::Long);
        assert_eq!(sig.regime, "bullish");
        assert!(sig.valid_geometry());
        // reward:risk should be 2.0 (take_profit is 2x ATR, stop is 1x ATR)
        let risk = (sig.entry_price - sig.stop_loss).abs();
        let reward = (sig.take_profit - sig.entry_price).abs();
        assert!((reward / risk - 2.0).abs() < 1e-9);
    }

    #[test]
    fn emits_short_signal_when_regime_bearish_and_entry_confirms() {
        let mut i = input(98.0, 95.0);
        i.entry_indicators.ema_8 = Some(99.0);
        i.entry_indicators.ema_21 = Some(100.0);
        i.entry_indicators.vwap = Some(100.0);
        let sig = TrendRegimeStrategy.evaluate(&ctx(), &i).unwrap().unwrap();
        assert_eq!(sig.side, Side::Short);
        assert_eq!(sig.regime, "bearish");
        assert!(sig.valid_geometry());
    }

    #[test]
    fn no_signal_when_screening_regime_is_ranging() {
        // screening close between vwap and ema_50 -> ranging, regardless of
        // entry-timeframe momentum.
        let mut i = input(102.0, 100.0);
        i.screening_indicators.vwap = Some(98.0);
        i.screening_indicators.ema_50 = Some(103.0);
        assert!(TrendRegimeStrategy.evaluate(&ctx(), &i).unwrap().is_none());
    }

    #[test]
    fn no_signal_when_entry_momentum_disagrees_with_regime() {
        // screening regime bullish, but entry-timeframe momentum is bearish.
        let mut i = input(102.0, 105.0);
        i.entry_indicators.ema_8 = Some(99.0);
        i.entry_indicators.ema_21 = Some(100.0);
        assert!(TrendRegimeStrategy.evaluate(&ctx(), &i).unwrap().is_none());
    }

    #[test]
    fn no_signal_when_atr_invalid() {
        let mut i = input(102.0, 105.0);
        i.entry_indicators.atr_14 = Some(0.0);
        assert!(TrendRegimeStrategy.evaluate(&ctx(), &i).unwrap().is_none());
        let mut i = input(102.0, 105.0);
        i.entry_indicators.atr_14 = Some(f64::NAN);
        assert!(TrendRegimeStrategy.evaluate(&ctx(), &i).unwrap().is_none());
    }
}
