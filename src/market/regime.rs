//! Generic market regime classification.
//!
//! This module provides deterministic market-context labels for signals,
//! attribution, and diagnostics. It is intentionally strategy-agnostic and has
//! no dependency on backtest execution, risk sizing, or reporting.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketRegime {
    Bullish,
    Bearish,
    Ranging,
    Unknown,
}

impl MarketRegime {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bullish => "bullish",
            Self::Bearish => "bearish",
            Self::Ranging => "ranging",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for MarketRegime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for MarketRegime {
    fn default() -> Self {
        Self::Unknown
    }
}

pub fn classify_basic_regime(close: f64, vwap: Option<f64>, ema_50: Option<f64>) -> MarketRegime {
    if !close.is_finite() || close <= 0.0 {
        return MarketRegime::Unknown;
    }

    let refs = [vwap, ema_50].into_iter().flatten().collect::<Vec<_>>();

    if refs.is_empty() {
        return MarketRegime::Unknown;
    }

    if refs.iter().any(|value| !value.is_finite() || *value <= 0.0) {
        return MarketRegime::Unknown;
    }

    let above_all = refs.iter().all(|value| close > *value);
    let below_all = refs.iter().all(|value| close < *value);

    if above_all {
        MarketRegime::Bullish
    } else if below_all {
        MarketRegime::Bearish
    } else {
        MarketRegime::Ranging
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_labels_are_exposed_as_strings() {
        assert_eq!(MarketRegime::Bullish.as_str(), "bullish");
        assert_eq!(MarketRegime::Bearish.as_str(), "bearish");
        assert_eq!(MarketRegime::Ranging.as_str(), "ranging");
        assert_eq!(MarketRegime::Unknown.as_str(), "unknown");
    }

    #[test]
    fn display_uses_stable_labels() {
        assert_eq!(MarketRegime::Bullish.to_string(), "bullish");
        assert_eq!(MarketRegime::Bearish.to_string(), "bearish");
        assert_eq!(MarketRegime::Ranging.to_string(), "ranging");
        assert_eq!(MarketRegime::Unknown.to_string(), "unknown");
    }

    #[test]
    fn default_regime_is_unknown() {
        assert_eq!(MarketRegime::default(), MarketRegime::Unknown);
    }

    #[test]
    fn classifies_bullish_when_close_above_all_available_references() {
        assert_eq!(
            classify_basic_regime(105.0, Some(100.0), Some(101.0)),
            MarketRegime::Bullish
        );
        assert_eq!(
            classify_basic_regime(105.0, Some(100.0), None),
            MarketRegime::Bullish
        );
    }

    #[test]
    fn classifies_bearish_when_close_below_all_available_references() {
        assert_eq!(
            classify_basic_regime(95.0, Some(100.0), Some(99.0)),
            MarketRegime::Bearish
        );
        assert_eq!(
            classify_basic_regime(95.0, None, Some(99.0)),
            MarketRegime::Bearish
        );
    }

    #[test]
    fn classifies_ranging_when_references_are_mixed_or_equal() {
        assert_eq!(
            classify_basic_regime(100.0, Some(99.0), Some(101.0)),
            MarketRegime::Ranging
        );
        assert_eq!(
            classify_basic_regime(100.0, Some(100.0), Some(101.0)),
            MarketRegime::Ranging
        );
    }

    #[test]
    fn classifies_unknown_when_close_is_invalid() {
        assert_eq!(
            classify_basic_regime(f64::NAN, Some(100.0), None),
            MarketRegime::Unknown
        );
        assert_eq!(
            classify_basic_regime(0.0, Some(100.0), None),
            MarketRegime::Unknown
        );
        assert_eq!(
            classify_basic_regime(-1.0, Some(100.0), None),
            MarketRegime::Unknown
        );
    }

    #[test]
    fn classifies_unknown_when_no_references_exist() {
        assert_eq!(
            classify_basic_regime(100.0, None, None),
            MarketRegime::Unknown
        );
    }

    #[test]
    fn classifies_unknown_when_any_provided_reference_is_invalid() {
        assert_eq!(
            classify_basic_regime(100.0, Some(f64::INFINITY), Some(99.0)),
            MarketRegime::Unknown
        );
        assert_eq!(
            classify_basic_regime(100.0, Some(99.0), Some(0.0)),
            MarketRegime::Unknown
        );
    }
}
