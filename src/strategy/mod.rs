//! Strategy layer.
//!
//! Production strategy set is intentionally minimal. The only active production
//! strategy is `basic_sample_strategy`, used as a reference implementation for
//! future strategy development.
//!
//! Strategies emit `Signal` only. They must not place orders, size positions,
//! call exchanges, mutate account state, or write reports.

pub mod basic_sample;
pub mod registry;
pub mod traits;

pub use basic_sample::BasicSampleStrategy;
pub use traits::{MultiTimeframeInput, Strategy, StrategyContext};
