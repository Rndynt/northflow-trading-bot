//! VwapReclaimShortV1 — lookback-aware bearish breakdown/retest strategy.
//!
//! Research-only strategy module. It emits short signals only and leaves risk,
//! sizing, fills, and reporting to the main engine pipeline.

use crate::config::VwapReclaimShortConfig;
use crate::core::{NorthflowError, Side, Signal, SignalId, StrategyId};
use crate::strategy::traits::{MultiTimeframeInput, Strategy, StrategyContext};

#[derive(Debug, Clone)]
pub struct VwapReclaimShortV1 {
    pub cfg: VwapReclaimShortConfig,
}

impl VwapReclaimShortV1 {
    pub fn new(cfg: VwapReclaimShortConfig) -> Self {
        Self { cfg }
    }
}

impl Strategy for VwapReclaimShortV1 {
    fn strategy_id(&self) -> &'static str {
        "vwap_reclaim_short_v1"
    }

    fn evaluate(
        &self,
        ctx: &StrategyContext,
        input: &MultiTimeframeInput,
    ) -> Result<Option<Signal>, NorthflowError> {
        input.entry_candle.validate()?;
        input.confirmation_candle.validate()?;
        input.screening_candle.validate()?;
        for candle in &input.entry_lookback {
            candle.validate()?;
        }

        macro_rules! required {
            ($value:expr) => {
                match $value {
                    Some(v) if v.is_finite() => v,
                    _ => return Ok(None),
                }
            };
        }

        let ema_8 = required!(input.entry_indicators.ema_8);
        let ema_21 = required!(input.entry_indicators.ema_21);
        let ema_50 = required!(input.entry_indicators.ema_50);
        let _ema_200 = required!(input.entry_indicators.ema_200);
        let atr = required!(input.entry_indicators.atr_14);
        let vwap = required!(input.entry_indicators.vwap);
        let volume_sma_20 = required!(input.entry_indicators.volume_sma_20);
        let conf_ema_21 = required!(input.confirmation_indicators.ema_21);
        let conf_ema_50 = required!(input.confirmation_indicators.ema_50);
        let conf_ema_200 = required!(input.confirmation_indicators.ema_200);
        let screen_ema_50 = required!(input.screening_indicators.ema_50);
        let screen_ema_200 = required!(input.screening_indicators.ema_200);

        if atr <= 0.0 || input.entry_candle.close <= 0.0 || volume_sma_20 <= 0.0 {
            return Ok(None);
        }

        let required = self.cfg.lookback_bars + self.cfg.breakout_window_bars;
        if input.entry_lookback.len() < required {
            return Ok(None);
        }
        let split = input.entry_lookback.len() - self.cfg.breakout_window_bars;
        let anchor_start = split.saturating_sub(self.cfg.lookback_bars);
        let anchor_range = &input.entry_lookback[anchor_start..split];
        let recent_window = &input.entry_lookback[split..];
        if anchor_range.is_empty() || recent_window.is_empty() {
            return Ok(None);
        }

        let screening_bearish =
            screen_ema_50 < screen_ema_200 && input.screening_candle.close < screen_ema_50;
        if !screening_bearish {
            return Ok(None);
        }

        let confirmation_bearish = conf_ema_21 < conf_ema_50
            && conf_ema_50 < conf_ema_200
            && input.confirmation_candle.close < conf_ema_21;
        if !confirmation_bearish {
            return Ok(None);
        }

        let entry_ema_alignment_short =
            ema_8 < ema_21 && ema_21 < ema_50 && input.entry_candle.close <= ema_21;
        if !entry_ema_alignment_short {
            return Ok(None);
        }

        let range_low = anchor_range
            .iter()
            .map(|c| c.low)
            .fold(f64::INFINITY, f64::min);
        let recent_breakdown = recent_window.iter().any(|c| c.close < range_low);
        if !recent_breakdown {
            return Ok(None);
        }

        let candle = input.entry_candle;
        let bearish_retest_hold = candle.high >= range_low - self.cfg.retest_tolerance_atr * atr
            && candle.close < range_low
            && candle.close < candle.open;
        if !bearish_retest_hold {
            return Ok(None);
        }

        let extension_atr = (range_low - candle.close) / atr;
        if extension_atr < 0.0 || extension_atr > self.cfg.max_extension_atr {
            return Ok(None);
        }

        if candle.close >= vwap {
            return Ok(None);
        }

        let atr_bps = atr / candle.close * 10_000.0;
        if atr_bps < self.cfg.min_atr_bps || atr_bps > self.cfg.max_atr_bps {
            return Ok(None);
        }

        let volume_ratio = candle.volume / volume_sma_20;
        if volume_ratio < self.cfg.min_volume_ratio {
            return Ok(None);
        }

        let entry = candle.close;
        let stop_loss = entry + atr * self.cfg.sl_atr_multiple;
        let take_profit = entry - atr * self.cfg.tp_atr_multiple;
        if !(take_profit < entry && entry < stop_loss) {
            return Ok(None);
        }

        let risk = stop_loss - entry;
        let reward = entry - take_profit;
        let reward_risk = reward / risk;
        if reward_risk < self.cfg.min_reward_risk {
            return Ok(None);
        }

        let expected_reward_bps = reward / entry * 10_000.0;
        let expected_net_edge_bps = expected_reward_bps - ctx.estimated_cost_bps;
        if expected_reward_bps < self.cfg.min_expected_reward_bps {
            return Ok(None);
        }
        if expected_net_edge_bps < self.cfg.min_expected_net_edge_bps {
            return Ok(None);
        }

        let confidence = 100u8;
        if confidence < ctx.min_confidence {
            return Ok(None);
        }

        Ok(Some(Signal {
            signal_id: SignalId::new(format!("SIG-BT-{:08X}", ctx.signal_index)),
            symbol: ctx.symbol.clone(),
            strategy_id: StrategyId::new("vwap_reclaim_short_v1"),
            side: Side::Short,
            entry_timeframe: ctx.entry_timeframe,
            screening_timeframe: ctx.screening_timeframe,
            confirmation_timeframe: ctx.confirmation_timeframe,
            entry_time: candle.timestamp,
            entry_price: entry,
            stop_loss,
            take_profit,
            confidence,
            regime: "bearish".to_string(),
            entry_reason: "vwap_reclaim_short_v1_range_breakdown_retest".to_string(),
            filters_passed: vec![
                "screening_bearish".to_string(),
                "confirmation_bearish".to_string(),
                "entry_ema_alignment_short".to_string(),
                "lookback_range_low_breakdown".to_string(),
                "bearish_retest_hold".to_string(),
                "below_vwap".to_string(),
                "atr_bps_in_range".to_string(),
                "volume_ratio_ok".to_string(),
                "reward_risk_ok".to_string(),
                "expected_reward_ok".to_string(),
                "expected_net_edge_ok".to_string(),
                "confidence_ok".to_string(),
            ],
            filters_failed: vec![],
            expected_reward_bps,
            estimated_cost_bps: ctx.estimated_cost_bps,
            expected_net_edge_bps,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Candle, Symbol, Timeframe};
    use crate::indicators::IndicatorSnapshot;

    fn strategy() -> VwapReclaimShortV1 {
        VwapReclaimShortV1::new(VwapReclaimShortConfig {
            lookback_bars: 5,
            breakout_window_bars: 3,
            ..Default::default()
        })
    }
    fn ctx() -> StrategyContext {
        StrategyContext {
            symbol: Symbol::new("BTCUSDT").unwrap(),
            signal_index: 1,
            estimated_cost_bps: 9.0,
            min_confidence: 70,
            entry_timeframe: Timeframe::OneMinute,
            confirmation_timeframe: Timeframe::FiveMinute,
            screening_timeframe: Timeframe::FifteenMinute,
        }
    }
    fn c(ts: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: ts,
            open,
            high,
            low,
            close,
            volume: 120.0,
        }
    }
    fn entry_snap() -> IndicatorSnapshot {
        IndicatorSnapshot {
            ema_8: Some(98.0),
            ema_21: Some(99.9),
            ema_50: Some(101.0),
            ema_200: Some(120.0),
            atr_14: Some(0.3),
            vwap: Some(100.5),
            volume_sma_20: Some(100.0),
        }
    }
    fn conf_snap() -> IndicatorSnapshot {
        IndicatorSnapshot {
            ema_21: Some(96.0),
            ema_50: Some(100.0),
            ema_200: Some(120.0),
            ..Default::default()
        }
    }
    fn screen_snap() -> IndicatorSnapshot {
        IndicatorSnapshot {
            ema_50: Some(100.0),
            ema_200: Some(120.0),
            ..Default::default()
        }
    }
    fn lookback() -> Vec<Candle> {
        vec![
            c(1, 101.0, 102.0, 100.0, 101.0),
            c(2, 101.0, 102.0, 100.5, 101.0),
            c(3, 101.0, 102.0, 100.2, 101.0),
            c(4, 101.0, 102.0, 100.1, 101.0),
            c(5, 101.0, 102.0, 100.3, 101.0),
            c(6, 100.0, 100.4, 99.8, 99.9),
            c(7, 99.9, 100.2, 99.5, 99.7),
            c(8, 99.7, 100.1, 99.4, 99.6),
        ]
    }
    fn input() -> MultiTimeframeInput {
        MultiTimeframeInput {
            entry_candle: c(9, 100.2, 100.4, 99.7, 99.8),
            entry_lookback: lookback(),
            confirmation_candle: c(10, 96.0, 97.0, 94.0, 95.0),
            screening_candle: c(15, 98.0, 99.0, 94.0, 95.0),
            entry_indicators: entry_snap(),
            confirmation_indicators: conf_snap(),
            screening_indicators: screen_snap(),
        }
    }

    #[test]
    fn strategy_id_is_stable() {
        assert_eq!(strategy().strategy_id(), "vwap_reclaim_short_v1");
    }
    #[test]
    fn no_signal_if_entry_lookback_too_short() {
        let mut i = input();
        i.entry_lookback.truncate(7);
        assert!(strategy().evaluate(&ctx(), &i).unwrap().is_none());
    }
    #[test]
    fn no_signal_if_screening_is_not_bearish() {
        let mut i = input();
        i.screening_indicators.ema_50 = Some(120.0);
        assert!(strategy().evaluate(&ctx(), &i).unwrap().is_none());
    }
    #[test]
    fn no_signal_if_confirmation_is_not_bearish() {
        let mut i = input();
        i.confirmation_indicators.ema_21 = Some(105.0);
        assert!(strategy().evaluate(&ctx(), &i).unwrap().is_none());
    }
    #[test]
    fn no_signal_if_recent_breakdown_is_absent() {
        let mut i = input();
        for c in &mut i.entry_lookback[5..] {
            c.high = c.high.max(100.2);
            c.close = 100.2;
        }
        assert!(strategy().evaluate(&ctx(), &i).unwrap().is_none());
    }
    #[test]
    fn no_signal_if_current_candle_does_not_retest_hold_below_range_low() {
        let mut i = input();
        i.entry_candle.close = 100.2;
        assert!(strategy().evaluate(&ctx(), &i).unwrap().is_none());
    }
    #[test]
    fn emits_short_signal_when_all_filters_pass() {
        let sig = strategy().evaluate(&ctx(), &input()).unwrap().unwrap();
        assert_eq!(sig.side, Side::Short);
    }
    #[test]
    fn emitted_short_signal_has_valid_geometry() {
        let sig = strategy().evaluate(&ctx(), &input()).unwrap().unwrap();
        assert!(sig.take_profit < sig.entry_price && sig.entry_price < sig.stop_loss);
    }
    #[test]
    fn emitted_signal_uses_configured_timeframe_roles() {
        let mut cx = ctx();
        cx.entry_timeframe = Timeframe::FiveMinute;
        cx.confirmation_timeframe = Timeframe::FifteenMinute;
        cx.screening_timeframe = Timeframe::OneHour;
        let sig = strategy().evaluate(&cx, &input()).unwrap().unwrap();
        assert_eq!(sig.entry_timeframe, Timeframe::FiveMinute);
        assert_eq!(sig.confirmation_timeframe, Timeframe::FifteenMinute);
        assert_eq!(sig.screening_timeframe, Timeframe::OneHour);
    }
    #[test]
    fn ignores_current_candle_in_lookback_market_structure() {
        let mut i = input();
        i.entry_lookback[0].low = 100.0;
        i.entry_candle.low = 1.0;
        let sig = strategy().evaluate(&ctx(), &i).unwrap().unwrap();
        assert_eq!(sig.entry_time, i.entry_candle.timestamp);
    }
}
