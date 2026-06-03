//! IndicatorSnapshot — passive container for current indicator values.
//!
//! This type carries only computed values — no signals, no strategy decisions,
//! no risk sizing.  Phase 4 will read these values to evaluate strategy
//! conditions; Phase 3 only populates them.
//!
//! IndicatorEngine — owns all Phase 3 indicators and updates them from one
//! validated Candle, returning an IndicatorSnapshot.

use crate::core::{Candle, NorthflowError};
use crate::indicators::{Atr, Ema, VolumeSma, Vwap};

// ── IndicatorSnapshot ────────────────────────────────────────────────────────

/// Current values for all Phase 3 indicators.  All fields are `Option<f64>`;
/// `None` means the indicator is still in its warmup phase.
#[derive(Debug, Clone, Default)]
pub struct IndicatorSnapshot {
    pub ema_8: Option<f64>,
    pub ema_21: Option<f64>,
    pub ema_50: Option<f64>,
    pub ema_200: Option<f64>,
    pub atr_14: Option<f64>,
    pub vwap: Option<f64>,
    pub volume_sma_20: Option<f64>,
}

// ── IndicatorEngine ──────────────────────────────────────────────────────────

/// Owns all Phase 3 indicators and updates them together from a single Candle.
///
/// Rules:
///   - Does not emit signals.
///   - Does not evaluate strategy conditions.
///   - Does not call the risk model.
///   - Only computes and returns indicator values.
pub struct IndicatorEngine {
    ema_8: Ema,
    ema_21: Ema,
    ema_50: Ema,
    ema_200: Ema,
    atr_14: Atr,
    vwap: Vwap,
    volume_sma_20: VolumeSma,
}

impl IndicatorEngine {
    /// Create an engine with the default Phase 3 indicator set.
    ///
    /// Returns an error if any indicator fails to initialise (should not happen
    /// with the hardcoded valid periods, but propagated for safety).
    pub fn new_default() -> Result<Self, NorthflowError> {
        Ok(Self {
            ema_8: Ema::new(8)?,
            ema_21: Ema::new(21)?,
            ema_50: Ema::new(50)?,
            ema_200: Ema::new(200)?,
            atr_14: Atr::new(14)?,
            vwap: Vwap::new(),
            volume_sma_20: VolumeSma::new(20)?,
        })
    }

    /// Feed one validated candle to all indicators and return the current snapshot.
    ///
    /// EMA is fed the close price.  ATR and VWAP consume the full candle.
    /// Volume SMA is fed the candle volume.
    ///
    /// Returns `Err` only if the candle is invalid or an indicator rejects the
    /// price (e.g. non-finite close).
    pub fn next(&mut self, candle: Candle) -> Result<IndicatorSnapshot, NorthflowError> {
        candle.validate()?;

        let ema_8 = self.ema_8.next(candle.close).ok();
        let ema_21 = self.ema_21.next(candle.close).ok();
        let ema_50 = self.ema_50.next(candle.close).ok();
        let ema_200 = self.ema_200.next(candle.close).ok();
        let atr_14 = self.atr_14.next(candle)?;
        // Use map to preserve the `Ok(Option)` → `Option` conversion.
        let vwap = self.vwap.next(candle)?;
        let volume_sma_20 = self.volume_sma_20.next(candle.volume)?;

        Ok(IndicatorSnapshot {
            ema_8,
            ema_21,
            ema_50,
            ema_200,
            atr_14,
            vwap,
            volume_sma_20,
        })
    }

    /// Reset all indicators to their initial state.
    pub fn reset(&mut self) {
        self.ema_8.reset();
        self.ema_21.reset();
        self.ema_50.reset();
        self.ema_200.reset();
        self.atr_14.reset();
        self.vwap.reset();
        self.volume_sma_20.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_candle(close: f64) -> Candle {
        Candle {
            timestamp: 1_700_000_000_000,
            open: close - 1.0,
            high: close + 2.0,
            low: close - 2.0,
            close,
            volume: 100.0,
        }
    }

    #[test]
    fn engine_can_be_created() {
        assert!(IndicatorEngine::new_default().is_ok());
    }

    #[test]
    fn engine_next_returns_snapshot() {
        let mut e = IndicatorEngine::new_default().unwrap();
        let snap = e.next(valid_candle(100.0)).unwrap();
        // EMA is ready after first price.
        assert!(snap.ema_8.is_some());
        assert!(snap.ema_21.is_some());
        assert!(snap.ema_50.is_some());
        assert!(snap.ema_200.is_some());
        // ATR needs 14 candles; VWAP ready after 1; VolumeSma needs 20.
        assert!(snap.atr_14.is_none());
        assert!(snap.vwap.is_some());
        assert!(snap.volume_sma_20.is_none());
    }

    #[test]
    fn engine_reset_clears_all_indicators() {
        let mut e = IndicatorEngine::new_default().unwrap();
        e.next(valid_candle(100.0)).unwrap();
        e.reset();
        let snap = e.next(valid_candle(100.0)).unwrap();
        // After reset, first candle again — ATR still warming up.
        assert!(snap.atr_14.is_none());
    }

    #[test]
    fn engine_rejects_invalid_candle() {
        let mut e = IndicatorEngine::new_default().unwrap();
        let bad = Candle {
            timestamp: 0,
            open: 100.0,
            high: 80.0, // high < low — invalid
            low: 90.0,
            close: 85.0,
            volume: 1.0,
        };
        assert!(e.next(bad).is_err());
    }

    #[test]
    fn snapshot_default_is_all_none() {
        let s = IndicatorSnapshot::default();
        assert!(s.ema_8.is_none());
        assert!(s.ema_21.is_none());
        assert!(s.ema_50.is_none());
        assert!(s.ema_200.is_none());
        assert!(s.atr_14.is_none());
        assert!(s.vwap.is_none());
        assert!(s.volume_sma_20.is_none());
    }
}
