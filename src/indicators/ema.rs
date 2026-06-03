//! EMA — Exponential Moving Average (deterministic, streaming).
//!
//! Formula:
//!   alpha = 2 / (period + 1)
//!   first value: EMA = price
//!   subsequent:  EMA = prev_ema + alpha * (price - prev_ema)
//!
//! The first valid price initializes the EMA directly.
//! No SMA warmup is used; EMA is ready after the very first price.

use crate::core::NorthflowError;

#[derive(Debug, Clone)]
pub struct Ema {
    period: usize,
    alpha: f64,
    value: Option<f64>,
}

impl Ema {
    /// Create an EMA with the given period.  Returns an error if `period == 0`.
    pub fn new(period: usize) -> Result<Self, NorthflowError> {
        if period == 0 {
            return Err(NorthflowError::ConfigError(
                "EMA period must be > 0".to_string(),
            ));
        }
        Ok(Self {
            period,
            alpha: 2.0 / (period as f64 + 1.0),
            value: None,
        })
    }

    /// The period this EMA was configured with.
    pub fn period(&self) -> usize {
        self.period
    }

    /// True after the first valid price has been fed.
    pub fn is_ready(&self) -> bool {
        self.value.is_some()
    }

    /// Current EMA value, or `None` if no price has been fed yet.
    pub fn value(&self) -> Option<f64> {
        self.value
    }

    /// Feed the next price and return the updated EMA.
    ///
    /// Rejects non-finite prices and prices ≤ 0.
    pub fn next(&mut self, price: f64) -> Result<f64, NorthflowError> {
        if !price.is_finite() {
            return Err(NorthflowError::DataError(format!(
                "EMA price must be finite, got {price}"
            )));
        }
        if price <= 0.0 {
            return Err(NorthflowError::DataError(format!(
                "EMA price must be > 0, got {price}"
            )));
        }
        let next = match self.value {
            Some(prev) => prev + self.alpha * (price - prev),
            None => price,
        };
        self.value = Some(next);
        Ok(next)
    }

    /// Reset internal state — clears the current EMA value.
    pub fn reset(&mut self) {
        self.value = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── construction ────────────────────────────────────────────────────────

    #[test]
    fn ema_rejects_zero_period() {
        assert!(Ema::new(0).is_err());
    }

    #[test]
    fn ema_period_returns_period() {
        let e = Ema::new(14).unwrap();
        assert_eq!(e.period(), 14);
    }

    #[test]
    fn ema_8_can_be_created() {
        assert!(Ema::new(8).is_ok());
    }

    #[test]
    fn ema_21_can_be_created() {
        assert!(Ema::new(21).is_ok());
    }

    #[test]
    fn ema_50_can_be_created() {
        assert!(Ema::new(50).is_ok());
    }

    #[test]
    fn ema_200_can_be_created() {
        assert!(Ema::new(200).is_ok());
    }

    // ── price validation ────────────────────────────────────────────────────

    #[test]
    fn ema_rejects_nan_price() {
        let mut e = Ema::new(5).unwrap();
        assert!(e.next(f64::NAN).is_err());
    }

    #[test]
    fn ema_rejects_negative_price() {
        let mut e = Ema::new(5).unwrap();
        assert!(e.next(-1.0).is_err());
    }

    #[test]
    fn ema_rejects_zero_price() {
        let mut e = Ema::new(5).unwrap();
        assert!(e.next(0.0).is_err());
    }

    // ── behavior ────────────────────────────────────────────────────────────

    #[test]
    fn ema_not_ready_before_first_price() {
        let e = Ema::new(10).unwrap();
        assert!(!e.is_ready());
        assert_eq!(e.value(), None);
    }

    #[test]
    fn ema_first_value_equals_first_price() {
        let mut e = Ema::new(3).unwrap();
        let v = e.next(100.0).unwrap();
        assert_eq!(v, 100.0);
        assert_eq!(e.value(), Some(100.0));
        assert!(e.is_ready());
    }

    #[test]
    fn ema_second_value_uses_alpha_formula() {
        // period=3 → alpha = 2/(3+1) = 0.5
        let mut e = Ema::new(3).unwrap();
        e.next(100.0).unwrap();
        let v = e.next(110.0).unwrap();
        // ema = 100 + 0.5 * (110 - 100) = 105
        assert!((v - 105.0).abs() < 1e-10);
    }

    #[test]
    fn ema_reset_clears_value() {
        let mut e = Ema::new(3).unwrap();
        e.next(100.0).unwrap();
        assert!(e.is_ready());
        e.reset();
        assert!(!e.is_ready());
        assert_eq!(e.value(), None);
    }
}
