//! CandleStore — holds candles for four configurable timeframe roles.
//!
//! Built from validated, sorted 1m candles using TimeframeBuilder.
//! Timeframe roles are configured at build time (entry, confirmation,
//! screening, regime). Immutable once built. No global state. No exchange
//! calls.
//!
//! `regime` is intentionally decoupled from entry/confirmation/screening: it
//! exists so a strategy can classify market regime from a higher timeframe
//! (e.g. 1h/4h) while entry/confirmation/screening are all tuned freely
//! (including all being set to the same timeframe) for signal generation.
//!
//! Always retains raw 1m candles for indicator pre-computation.

use crate::core::{Candle, NorthflowError, Timeframe};
use crate::market::timeframe_builder::TimeframeBuilder;

#[derive(Debug)]
pub struct CandleStore {
    /// Raw 1m candles — always present, used for indicator pre-computation.
    pub raw_1m: Vec<Candle>,

    /// The entry timeframe configured for this run.
    pub entry_tf: Timeframe,
    /// The confirmation timeframe configured for this run.
    pub confirmation_tf: Timeframe,
    /// The screening timeframe configured for this run.
    pub screening_tf: Timeframe,
    /// The regime timeframe configured for this run — independent of
    /// entry/confirmation/screening, typically a higher timeframe used for
    /// market-regime classification only.
    pub regime_tf: Timeframe,

    /// Candles for the entry timeframe (may be same as raw_1m if entry_tf == OneMinute).
    pub entry_candles: Vec<Candle>,
    /// Candles for the confirmation timeframe.
    pub confirmation_candles: Vec<Candle>,
    /// Candles for the screening timeframe.
    pub screening_candles: Vec<Candle>,
    /// Candles for the regime timeframe.
    pub regime_candles: Vec<Candle>,
}

impl CandleStore {
    /// Build a CandleStore from sorted, validated 1m candles with configurable TF roles.
    ///
    /// Entry candles are built from 1m if entry_tf != OneMinute, else use raw 1m directly.
    /// Confirmation, screening, and regime candles are always built from 1m via
    /// TimeframeBuilder. Incomplete higher-timeframe buckets are silently dropped.
    ///
    /// Ordering rule: entry <= confirmation <= screening (equal roles are
    /// allowed — e.g. entry = confirmation = screening = 5m is valid).
    /// `regime_tf` only needs to be >= `entry_tf`; it has no required relation
    /// to confirmation/screening, since it is a separate, independently
    /// configurable role.
    pub fn build(
        candles_1m: Vec<Candle>,
        entry_tf: Timeframe,
        confirmation_tf: Timeframe,
        screening_tf: Timeframe,
        regime_tf: Timeframe,
    ) -> Result<Self, NorthflowError> {
        if entry_tf > confirmation_tf {
            return Err(NorthflowError::ConfigError(format!(
                "entry_timeframe ({entry_tf}) must not be longer than confirmation_timeframe ({confirmation_tf})"
            )));
        }
        if confirmation_tf > screening_tf {
            return Err(NorthflowError::ConfigError(format!(
                "confirmation_timeframe ({confirmation_tf}) must not be longer than screening_timeframe ({screening_tf})"
            )));
        }
        if regime_tf < entry_tf {
            return Err(NorthflowError::ConfigError(format!(
                "regime_timeframe ({regime_tf}) must not be shorter than entry_timeframe ({entry_tf})"
            )));
        }

        // Build entry candles.
        let entry_candles = if entry_tf == Timeframe::OneMinute {
            candles_1m.clone()
        } else {
            TimeframeBuilder::build(&candles_1m, entry_tf)?
        };

        let confirmation_candles = TimeframeBuilder::build(&candles_1m, confirmation_tf)?;
        let screening_candles = TimeframeBuilder::build(&candles_1m, screening_tf)?;
        let regime_candles = TimeframeBuilder::build(&candles_1m, regime_tf)?;

        Ok(Self {
            raw_1m: candles_1m,
            entry_tf,
            confirmation_tf,
            screening_tf,
            regime_tf,
            entry_candles,
            confirmation_candles,
            screening_candles,
            regime_candles,
        })
    }

    /// Legacy constructor — builds with the original 1m/5m/15m roles and a
    /// 1h regime role.
    ///
    /// Preserved for backward compatibility with tests.
    pub fn build_from_1m(candles_1m: Vec<Candle>) -> Result<Self, NorthflowError> {
        Self::build(
            candles_1m,
            Timeframe::OneMinute,
            Timeframe::FiveMinute,
            Timeframe::FifteenMinute,
            Timeframe::OneHour,
        )
    }

    /// Number of candles for the given timeframe role.
    pub fn entry_len(&self) -> usize {
        self.entry_candles.len()
    }
    pub fn confirmation_len(&self) -> usize {
        self.confirmation_candles.len()
    }
    pub fn screening_len(&self) -> usize {
        self.screening_candles.len()
    }
    pub fn regime_len(&self) -> usize {
        self.regime_candles.len()
    }

    /// Number of candles for any supported timeframe (by value).
    /// Returns 0 for unsupported timeframes.
    pub fn len(&self, tf: Timeframe) -> usize {
        if tf == self.entry_tf {
            return self.entry_candles.len();
        }
        if tf == self.confirmation_tf {
            return self.confirmation_candles.len();
        }
        if tf == self.screening_tf {
            return self.screening_candles.len();
        }
        if tf == self.regime_tf {
            return self.regime_candles.len();
        }
        if tf == Timeframe::OneMinute {
            return self.raw_1m.len();
        }
        0
    }

    /// Whether the candle list for the given timeframe is empty (or unsupported).
    pub fn is_empty(&self, tf: Timeframe) -> bool {
        self.len(tf) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candle(ts_ms: i64) -> Candle {
        Candle {
            timestamp: ts_ms,
            open: 100.0,
            high: 110.0,
            low: 90.0,
            close: 105.0,
            volume: 10.0,
        }
    }

    fn fifteen_1m() -> Vec<Candle> {
        (0..15).map(|i| make_candle(i as i64 * 60_000)).collect()
    }

    #[test]
    fn builds_default_1m_5m_15m_1h_regime() {
        let s = CandleStore::build_from_1m(fifteen_1m()).unwrap();
        assert_eq!(s.entry_len(), 15);
        assert_eq!(s.confirmation_len(), 3);
        assert_eq!(s.screening_len(), 1);
        assert_eq!(s.entry_tf, Timeframe::OneMinute);
        assert_eq!(s.confirmation_tf, Timeframe::FiveMinute);
        assert_eq!(s.screening_tf, Timeframe::FifteenMinute);
        assert_eq!(s.regime_tf, Timeframe::OneHour);
        assert_eq!(s.regime_len(), 0); // only 15 1m candles, not enough for 1h
    }

    #[test]
    fn builds_5m_15m_1h_with_4h_regime() {
        // Need at least 240 1m candles for 4h.
        let candles: Vec<Candle> = (0..240).map(|i| make_candle(i * 60_000)).collect();
        let s = CandleStore::build(
            candles,
            Timeframe::FiveMinute,
            Timeframe::FifteenMinute,
            Timeframe::OneHour,
            Timeframe::FourHour,
        )
        .unwrap();
        assert_eq!(s.entry_tf, Timeframe::FiveMinute);
        assert_eq!(s.confirmation_tf, Timeframe::FifteenMinute);
        assert_eq!(s.screening_tf, Timeframe::OneHour);
        assert_eq!(s.regime_tf, Timeframe::FourHour);
        assert_eq!(s.entry_len(), 48); // 240/5
        assert_eq!(s.confirmation_len(), 16); // 240/15
        assert_eq!(s.screening_len(), 4); // 240/60
        assert_eq!(s.regime_len(), 1); // 240/240
    }

    #[test]
    fn entry_confirmation_screening_may_all_be_equal() {
        // Decoupled regime role means entry/confirmation/screening no longer
        // need to be distinct — e.g. all set to 5m is a valid config.
        let candles: Vec<Candle> = (0..300).map(|i| make_candle(i * 60_000)).collect();
        let s = CandleStore::build(
            candles,
            Timeframe::FiveMinute,
            Timeframe::FiveMinute,
            Timeframe::FiveMinute,
            Timeframe::OneHour,
        )
        .unwrap();
        assert_eq!(s.entry_tf, Timeframe::FiveMinute);
        assert_eq!(s.confirmation_tf, Timeframe::FiveMinute);
        assert_eq!(s.screening_tf, Timeframe::FiveMinute);
        assert_eq!(s.regime_tf, Timeframe::OneHour);
    }

    #[test]
    fn len_lookup_by_tf_value() {
        let s = CandleStore::build_from_1m(fifteen_1m()).unwrap();
        assert_eq!(s.len(Timeframe::OneMinute), 15);
        assert_eq!(s.len(Timeframe::FiveMinute), 3);
        assert_eq!(s.len(Timeframe::FifteenMinute), 1);
        assert_eq!(s.len(Timeframe::OneHour), 0); // regime role, but not enough 1m data
        assert_eq!(s.len(Timeframe::FourHour), 0); // not in this store at all
    }

    #[test]
    fn rejects_entry_longer_than_confirmation() {
        let err = CandleStore::build(
            fifteen_1m(),
            Timeframe::FifteenMinute,
            Timeframe::FiveMinute,
            Timeframe::FifteenMinute,
            Timeframe::OneHour,
        )
        .unwrap_err();
        assert!(err.to_string().contains("entry_timeframe"));
    }

    #[test]
    fn rejects_confirmation_longer_than_screening() {
        let err = CandleStore::build(
            fifteen_1m(),
            Timeframe::OneMinute,
            Timeframe::FifteenMinute,
            Timeframe::FiveMinute,
            Timeframe::OneHour,
        )
        .unwrap_err();
        assert!(err.to_string().contains("confirmation_timeframe"));
    }

    #[test]
    fn rejects_regime_shorter_than_entry() {
        let err = CandleStore::build(
            fifteen_1m(),
            Timeframe::FifteenMinute,
            Timeframe::FifteenMinute,
            Timeframe::OneHour,
            Timeframe::FiveMinute,
        )
        .unwrap_err();
        assert!(err.to_string().contains("regime_timeframe"));
    }
}
