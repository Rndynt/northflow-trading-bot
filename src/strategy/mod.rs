//! Strategy layer.
//!
//! Active production strategies:
//!   - `basic_sample_strategy` — reference multi-timeframe alignment strategy.
//!     Backtested with realistic risk sizing on 6yr BTCUSDT 1m data: negative
//!     expectancy (see docs/executable-signal-lifecycle-audit.md). Kept as a
//!     baseline for comparison, not recommended for live use as-is.
//!   - `trend_regime_strategy` — regime-gated trend strategy. Classifies
//!     screening-timeframe regime first, skips non-trending periods entirely,
//!     then requires entry-timeframe momentum/value-area to agree, with a
//!     wider 2:1 reward:risk than basic_sample_strategy.
//!
//! Strategies emit `Signal` only. They must not place orders, size positions,
//! call exchanges, mutate account state, or write reports.

pub mod basic_sample;
pub mod ids;
pub mod registry;
pub mod traits;

pub use basic_sample::BasicSampleStrategy;
pub use traits::{MultiTimeframeInput, PositionAction, Strategy, StrategyContext};

