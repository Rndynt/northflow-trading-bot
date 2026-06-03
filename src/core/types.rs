//! DEPRECATED — superseded by Phase 1 modular core types.
//!
//! This file is intentionally NOT included in `src/core/mod.rs`.
//! It is preserved here only to avoid breaking git history.
//!
//! The active replacements are:
//!   src/core/side.rs       — Side::Long / Side::Short
//!   src/core/candle.rs     — Candle with validate()
//!   src/core/signal.rs     — Signal with mandatory signal_id
//!   src/core/trade.rs      — Trade (replaces SimTrade)
//!
//! DO NOT import from this file. DO NOT add new code here.
