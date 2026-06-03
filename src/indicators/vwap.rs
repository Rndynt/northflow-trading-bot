//! VWAP — Volume Weighted Average Price (deterministic, streaming, session-cumulative).
//!
//! Formula:
//!   typical_price = (high + low + close) / 3
//!   vwap = Σ(typical_price × volume) / Σ(volume)
//!
//! Zero-volume candles do not update cumulative state.
//! Division by zero is explicitly guarded.

use crate::core::{Candle, NorthflowError};

#[derive(Debug, Clone, Default)]
pub struct Vwap {
    pv_sum: f64,
    volume_sum: f64,
}

impl Vwap {
    pub fn new() -> Self {
        Self::default()
    }

    /// True once at least one candle with volume > 0 has been fed.
    pub fn is_ready(&self) -> bool {
        self.volume_sum > 0.0
    }

    /// Current VWAP value, or `None` if no volume has been accumulated yet.
    pub fn value(&self) -> Option<f64> {
        if self.volume_sum > 0.0 {
            Some(self.pv_sum / self.volume_sum)
        } else {
            None
        }
    }

    /// Feed the next candle and return the current VWAP.
    ///
    /// Zero-volume candles: does not update state.
    ///   - Returns `Ok(None)` if no volume accumulated yet.
    ///   - Returns `Ok(Some(vwap))` if already ready (previous value preserved).
    /// Invalid candles return `Err`.
    pub fn next(&mut self, candle: Candle) -> Result<Option<f64>, NorthflowError> {
        candle.validate()?;

        if candle.volume > 0.0 {
            let typical = (candle.high + candle.low + candle.close) / 3.0;
            self.pv_sum += typical * candle.volume;
            self.volume_sum += candle.volume;
        }
        // Zero-volume: state unchanged; return whatever value() currently holds.
        Ok(self.value())
    }

    /// Reset cumulative state.
    pub fn reset(&mut self) {
        self.pv_sum = 0.0;
        self.volume_sum = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: 1_000_000,
            open: low,
            high,
            low,
            close,
            volume,
        }
    }

    fn zero_vol() -> Candle {
        c(110.0, 90.0, 100.0, 0.0)
    }

    fn invalid() -> Candle {
        // high < low
        Candle {
            timestamp: 0,
            open: 100.0,
            high: 80.0,
            low: 90.0,
            close: 85.0,
            volume: 1.0,
        }
    }

    #[test]
    fn vwap_starts_not_ready() {
        let v = Vwap::new();
        assert!(!v.is_ready());
        assert_eq!(v.value(), None);
    }

    #[test]
    fn vwap_first_nonzero_volume_candle_calculates_value() {
        let mut v = Vwap::new();
        // typical = (110 + 90 + 100) / 3 = 100
        // vwap = 100 * 10 / 10 = 100
        let result = v.next(c(110.0, 90.0, 100.0, 10.0)).unwrap();
        assert!(v.is_ready());
        assert!((result.unwrap() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn vwap_accumulates_multiple_candles() {
        let mut v = Vwap::new();
        // Candle 1: typical=100, vol=10 → pv=1000, vol_sum=10
        v.next(c(110.0, 90.0, 100.0, 10.0)).unwrap();
        // Candle 2: typical=110, vol=10 → pv=1000+1100=2100, vol_sum=20 → vwap=105
        let result = v.next(c(120.0, 100.0, 110.0, 10.0)).unwrap();
        assert!((result.unwrap() - 105.0).abs() < 1e-10);
    }

    #[test]
    fn vwap_zero_volume_before_ready_returns_none() {
        let mut v = Vwap::new();
        let result = v.next(zero_vol()).unwrap();
        assert_eq!(result, None);
        assert!(!v.is_ready());
    }

    #[test]
    fn vwap_zero_volume_after_ready_returns_existing_value() {
        let mut v = Vwap::new();
        // Establish a VWAP value first.
        v.next(c(110.0, 90.0, 100.0, 10.0)).unwrap();
        let before = v.value().unwrap();
        // Zero-volume candle must not change state.
        let result = v.next(zero_vol()).unwrap();
        assert_eq!(result, Some(before));
        assert!((v.value().unwrap() - before).abs() < 1e-10);
    }

    #[test]
    fn vwap_rejects_invalid_candle() {
        let mut v = Vwap::new();
        assert!(v.next(invalid()).is_err());
    }

    #[test]
    fn vwap_reset_clears_state() {
        let mut v = Vwap::new();
        v.next(c(110.0, 90.0, 100.0, 10.0)).unwrap();
        assert!(v.is_ready());
        v.reset();
        assert!(!v.is_ready());
        assert_eq!(v.value(), None);
    }
}
