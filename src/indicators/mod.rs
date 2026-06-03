//! Phase 3 indicators — deterministic, streaming, no network, no strategy logic.
//!
//! Exports:
//!   Ema          — Exponential Moving Average (periods: 8, 21, 50, 200)
//!   Atr          — Average True Range with Wilder smoothing (period: 14)
//!   Vwap         — Volume Weighted Average Price (session-cumulative)
//!   VolumeSma    — Simple Moving Average of volume (period: 20)
//!
//! Optional helpers:
//!   IndicatorSnapshot — passive value container (no strategy fields)
//!   IndicatorEngine   — owns all indicators; updates from one Candle

mod atr;
mod ema;
mod snapshot;
mod volume;
mod vwap;

pub use atr::Atr;
pub use ema::Ema;
pub use snapshot::{IndicatorEngine, IndicatorSnapshot};
pub use volume::VolumeSma;
pub use vwap::Vwap;
