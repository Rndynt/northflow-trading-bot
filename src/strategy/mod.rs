//! Strategy layer.
//!
//! Active production strategies:
//!   - `basic_sample_strategy` — reference multi-timeframe alignment strategy.
//!     Backtested with realistic risk sizing on 6yr BTCUSDT 1m data: negative
//!     expectancy (see docs/executable-signal-lifecycle-audit.md). Kept as a
//!     baseline for comparison, not recommended for live use as-is.
//!
//! `MultiTimeframeInput` carries four independent timeframe roles per
//! evaluation: entry, confirmation, screening (all tunable freely, including
//! all being the same timeframe), and regime (typically a higher timeframe,
//! for market-context classification via `market::classify_basic_regime`).
//!
//! Strategies emit `Signal` only. They must not place orders, size positions,
//! call exchanges, mutate account state, or write reports.

pub mod basic_sample;
pub mod ids;
pub mod registry;
pub mod traits;

pub use basic_sample::BasicSampleStrategy;
pub use traits::{MultiTimeframeInput, PositionAction, Strategy, StrategyContext};
