//! ATR — Average True Range with Wilder smoothing (deterministic, streaming).
//!
//! True Range:
//!   TR = max(high - low, |high - prev_close|, |low - prev_close|)
//!   For the first candle (no prev_close): TR = high - low
//!
//! Wilder smoothing:
//!   Warmup: collect `period` true ranges.
//!   Initial ATR = mean of those `period` TRs.
//!   Subsequent: ATR = (prev_atr * (period - 1) + current_tr) / period

use crate::core::{Candle, NorthflowError};

#[derive(Debug, Clone)]
pub struct Atr {
    period: usize,
    prev_close: Option<f64>,
    /// TRs accumulated during the warmup phase (until we have `period` of them).
    warmup: Vec<f64>,
    /// Current smoothed ATR value (set once warmup is complete).
    value: Option<f64>,
}

impl Atr {
    /// Create an ATR with the given period.  Returns an error if `period == 0`.
    pub fn new(period: usize) -> Result<Self, NorthflowError> {
        if period == 0 {
            return Err(NorthflowError::ConfigError(
                "ATR period must be > 0".to_string(),
            ));
        }
        Ok(Self {
            period,
            prev_close: None,
            warmup: Vec::with_capacity(period),
            value: None,
        })
    }

    /// The period this ATR was configured with.
    pub fn period(&self) -> usize {
        self.period
    }

    /// True once `period` candles have been fed and the initial ATR is computed.
    pub fn is_ready(&self) -> bool {
        self.value.is_some()
    }

    /// Current ATR value, or `None` during warmup.
    pub fn value(&self) -> Option<f64> {
        self.value
    }

    /// Feed the next candle and return the current ATR.
    ///
    /// Returns `Ok(None)` during the warmup phase.
    /// Returns `Ok(Some(atr))` once ready.
    /// Returns `Err` if the candle is invalid.
    pub fn next(&mut self, candle: Candle) -> Result<Option<f64>, NorthflowError> {
        candle.validate()?;

        let tr = match self.prev_close {
            Some(prev) => (candle.high - candle.low)
                .max((candle.high - prev).abs())
                .max((candle.low - prev).abs()),
            None => candle.high - candle.low,
        };
        self.prev_close = Some(candle.close);

        if self.value.is_none() {
            // Warmup phase: accumulate TRs until we have `period` of them.
            self.warmup.push(tr);
            if self.warmup.len() == self.period {
                let initial = self.warmup.iter().sum::<f64>() / self.period as f64;
                self.value = Some(initial);
                self.warmup.clear();
            }
        } else {
            // Wilder smoothing.
            let prev_atr = self.value.unwrap();
            self.value = Some((prev_atr * (self.period as f64 - 1.0) + tr) / self.period as f64);
        }

        Ok(self.value)
    }

    /// Reset all internal state.
    pub fn reset(&mut self) {
        self.prev_close = None;
        self.warmup.clear();
        self.value = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: 0,
            open: low,
            high,
            low,
            close,
            volume: 1.0,
        }
    }

    fn invalid_candle() -> Candle {
        // high < low — invalid geometry
        Candle {
            timestamp: 0,
            open: 100.0,
            high: 80.0,
            low: 90.0,
            close: 85.0,
            volume: 1.0,
        }
    }

    // ── construction ────────────────────────────────────────────────────────

    #[test]
    fn atr_rejects_zero_period() {
        assert!(Atr::new(0).is_err());
    }

    #[test]
    fn atr_14_can_be_created() {
        assert!(Atr::new(14).is_ok());
    }

    // ── candle validation ────────────────────────────────────────────────────

    #[test]
    fn atr_rejects_invalid_candle() {
        let mut a = Atr::new(3).unwrap();
        assert!(a.next(invalid_candle()).is_err());
    }

    // ── warmup / readiness ───────────────────────────────────────────────────

    #[test]
    fn atr_first_values_not_ready_until_period() {
        let mut a = Atr::new(3).unwrap();
        assert_eq!(a.next(candle(110.0, 90.0, 100.0)).unwrap(), None);
        assert_eq!(a.next(candle(115.0, 95.0, 105.0)).unwrap(), None);
        assert!(!a.is_ready());
        // Third candle completes warmup.
        let v = a.next(candle(120.0, 100.0, 110.0)).unwrap();
        assert!(v.is_some());
        assert!(a.is_ready());
    }

    #[test]
    fn atr_initial_value_is_average_true_range() {
        // period=2, two constant candles with no prev_close on first.
        // Candle 1: TR = high - low = 20
        // Candle 2: TR = max(high-low=20, |high-prev_close|=5, |low-prev_close|=15) = 20
        // Initial ATR = (20 + 20) / 2 = 20
        let mut a = Atr::new(2).unwrap();
        a.next(candle(110.0, 90.0, 100.0)).unwrap(); // TR=20, warmup
        let v = a.next(candle(115.0, 95.0, 105.0)).unwrap(); // TR=20, ready
        assert!((v.unwrap() - 20.0).abs() < 1e-10);
    }

    #[test]
    fn atr_uses_previous_close_for_true_range() {
        // candle 1: close=100
        // candle 2 (period=1): TR = max(high-low=5, |115-100|=15, |110-100|=10) = 15
        let mut a = Atr::new(1).unwrap();
        a.next(candle(110.0, 90.0, 100.0)).unwrap(); // warmup candle (period=1, immediately ready)
        // After the first candle the warmup is complete (period=1 means 1 TR needed).
        // ATR = 20 (high-low, no prev close for first candle).
        assert!(a.is_ready());
        // Third candle uses prev_close=100 for TR calculation.
        let v = a.next(candle(115.0, 110.0, 112.0)).unwrap();
        // TR = max(115-110=5, |115-100|=15, |110-100|=10) = 15
        // Wilder: (20 * 0 + 15) / 1 = 15
        assert!((v.unwrap() - 15.0).abs() < 1e-10);
    }

    #[test]
    fn atr_wilder_smoothing_after_ready() {
        // period=2
        // Candle 1: TR=20 (no prev)
        // Candle 2: TR=20 → initial ATR = 20
        // Candle 3: TR = max(10, |...|, |...|). Let's use a simple case:
        //   candle3: high=106, low=104, close=105 with prev_close=105
        //   TR = max(2, |106-105|=1, |104-105|=1) = 2
        //   Wilder: (20 * 1 + 2) / 2 = 11
        let mut a = Atr::new(2).unwrap();
        a.next(candle(110.0, 90.0, 100.0)).unwrap();
        a.next(candle(120.0, 100.0, 105.0)).unwrap();
        let v = a.next(candle(106.0, 104.0, 105.0)).unwrap();
        // TR3 = max(2, 1, 1) = 2
        // prev_atr = (20+20)/2 = 20
        // new_atr = (20*1 + 2)/2 = 11
        assert!((v.unwrap() - 11.0).abs() < 1e-10);
    }

    #[test]
    fn atr_reset_clears_state() {
        let mut a = Atr::new(1).unwrap();
        a.next(candle(110.0, 90.0, 100.0)).unwrap();
        assert!(a.is_ready());
        a.reset();
        assert!(!a.is_ready());
        assert_eq!(a.value(), None);
        // After reset, next call should not use the old prev_close.
        let v = a.next(candle(110.0, 90.0, 100.0)).unwrap();
        // First candle again — TR = high - low = 20, period=1, immediately ready.
        assert!((v.unwrap() - 20.0).abs() < 1e-10);
    }
}
