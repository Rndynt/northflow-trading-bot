//! VolumeSma — Simple Moving Average of volume (deterministic, streaming).
//!
//! Keeps a rolling window of the last `period` volume values using a VecDeque
//! and a running sum for O(1) updates.  Returns `None` until `period` samples
//! have been collected.

use std::collections::VecDeque;

use crate::core::NorthflowError;

#[derive(Debug, Clone)]
pub struct VolumeSma {
    period: usize,
    window: VecDeque<f64>,
    sum: f64,
}

impl VolumeSma {
    /// Create a VolumeSma with the given period.  Returns an error if `period == 0`.
    pub fn new(period: usize) -> Result<Self, NorthflowError> {
        if period == 0 {
            return Err(NorthflowError::ConfigError(
                "VolumeSma period must be > 0".to_string(),
            ));
        }
        Ok(Self {
            period,
            window: VecDeque::with_capacity(period),
            sum: 0.0,
        })
    }

    /// The period this VolumeSma was configured with.
    pub fn period(&self) -> usize {
        self.period
    }

    /// True once `period` samples have been collected.
    pub fn is_ready(&self) -> bool {
        self.window.len() == self.period
    }

    /// Current SMA value, or `None` during warmup.
    pub fn value(&self) -> Option<f64> {
        if self.is_ready() {
            Some(self.sum / self.period as f64)
        } else {
            None
        }
    }

    /// Feed the next volume sample and return the current SMA.
    ///
    /// Returns `Ok(None)` during warmup (fewer than `period` samples).
    /// Returns `Ok(Some(sma))` once ready.
    /// Rejects non-finite volume and volume < 0.
    pub fn next(&mut self, volume: f64) -> Result<Option<f64>, NorthflowError> {
        if !volume.is_finite() {
            return Err(NorthflowError::DataError(format!(
                "VolumeSma volume must be finite, got {volume}"
            )));
        }
        if volume < 0.0 {
            return Err(NorthflowError::DataError(format!(
                "VolumeSma volume must be >= 0, got {volume}"
            )));
        }

        if self.window.len() == self.period {
            // Drop the oldest sample from the rolling sum.
            self.sum -= self.window.pop_front().unwrap_or(0.0);
        }
        self.window.push_back(volume);
        self.sum += volume;

        Ok(self.value())
    }

    /// Reset all internal state.
    pub fn reset(&mut self) {
        self.window.clear();
        self.sum = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── construction ────────────────────────────────────────────────────────

    #[test]
    fn volume_sma_rejects_zero_period() {
        assert!(VolumeSma::new(0).is_err());
    }

    #[test]
    fn volume_sma_20_can_be_created() {
        assert!(VolumeSma::new(20).is_ok());
    }

    // ── volume validation ────────────────────────────────────────────────────

    #[test]
    fn volume_sma_rejects_nan_volume() {
        let mut s = VolumeSma::new(3).unwrap();
        assert!(s.next(f64::NAN).is_err());
    }

    #[test]
    fn volume_sma_rejects_negative_volume() {
        let mut s = VolumeSma::new(3).unwrap();
        assert!(s.next(-1.0).is_err());
    }

    // ── behavior ────────────────────────────────────────────────────────────

    #[test]
    fn volume_sma_not_ready_until_period() {
        let mut s = VolumeSma::new(3).unwrap();
        assert_eq!(s.next(10.0).unwrap(), None);
        assert_eq!(s.next(20.0).unwrap(), None);
        assert!(!s.is_ready());
        let v = s.next(30.0).unwrap();
        assert!(v.is_some());
        assert!(s.is_ready());
    }

    #[test]
    fn volume_sma_computes_average() {
        let mut s = VolumeSma::new(3).unwrap();
        s.next(10.0).unwrap();
        s.next(20.0).unwrap();
        let v = s.next(30.0).unwrap().unwrap();
        // (10 + 20 + 30) / 3 = 20
        assert!((v - 20.0).abs() < 1e-10);
    }

    #[test]
    fn volume_sma_rolls_window_forward() {
        let mut s = VolumeSma::new(3).unwrap();
        s.next(10.0).unwrap();
        s.next(20.0).unwrap();
        s.next(30.0).unwrap(); // window: [10, 20, 30] → avg=20
        let v = s.next(40.0).unwrap().unwrap(); // window: [20, 30, 40] → avg=30
        assert!((v - 30.0).abs() < 1e-10);
    }

    #[test]
    fn volume_sma_reset_clears_state() {
        let mut s = VolumeSma::new(2).unwrap();
        s.next(10.0).unwrap();
        s.next(20.0).unwrap();
        assert!(s.is_ready());
        s.reset();
        assert!(!s.is_ready());
        assert_eq!(s.value(), None);
    }

    #[test]
    fn volume_sma_accepts_zero_volume() {
        let mut s = VolumeSma::new(2).unwrap();
        s.next(0.0).unwrap();
        let v = s.next(10.0).unwrap().unwrap();
        assert!((v - 5.0).abs() < 1e-10);
    }
}
