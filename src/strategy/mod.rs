//! Strategy engine — Phase 4.
//!
//! Active strategies:
//!   screened_vwap_scalp    — original deterministic strategy
//!   screened_vwap_scalp_v2 — stricter, cost-aware research variant
//!
//! Emits Signal only. No orders, no risk sizing, no backtest execution.
//!
//! Timeframe roles (explicit — never inferred from array order):
//!   entry_timeframe        = "1m"   (entry and execution)
//!   confirmation_timeframe = "5m"   (intermediate confirmation)
//!   screening_timeframe    = "15m"  (market regime / bias)
//!
//! Downstream phases:
//!   Phase 5 — risk and cost model
//!   Phase 6 — backtest engine
//!   Phase 7 — report writers

pub mod regime;
pub mod screened_vwap_scalp;
pub mod screened_vwap_scalp_v2;
pub mod traits;

pub use regime::{MarketRegime, classify_screening_regime};
pub use screened_vwap_scalp::ScreenedVwapScalp;
pub use screened_vwap_scalp_v2::ScreenedVwapScalpV2;
pub use traits::{MultiTimeframeInput, Strategy, StrategyContext};
