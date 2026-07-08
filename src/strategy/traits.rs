//! Strategy trait — deterministic, single-candle, multi-timeframe signal evaluation.
//!
//! Rules:
//!   - Strategies may only read candles and indicator snapshots.
//!   - Strategies must not call exchange APIs, LLMs, or mutate account state.
//!   - Output is Result<Option<Signal>, NorthflowError> — a Signal, not an order.

use crate::core::{Candle, NorthflowError, Side, Signal, Symbol, Timeframe};
use crate::indicators::IndicatorSnapshot;

// ── StrategyContext ───────────────────────────────────────────────────────────

/// Caller-supplied context for a single strategy evaluation.
///
/// Does not include candles or snapshots — those travel in [`MultiTimeframeInput`].
#[derive(Debug, Clone)]
pub struct StrategyContext {
    /// Symbol being evaluated.
    pub symbol: Symbol,
    /// Monotonically increasing index for deterministic signal ID generation.
    pub signal_index: u64,
    /// Estimated round-trip cost in basis points (taker fee + slippage + spread).
    pub estimated_cost_bps: f64,
    /// Minimum confidence score required to emit a signal (0–100).
    pub min_confidence: u8,
    /// Entry timeframe for this run (e.g. 1m, 5m).
    pub entry_timeframe: Timeframe,
    /// Confirmation timeframe for this run (e.g. 5m, 1h).
    pub confirmation_timeframe: Timeframe,
    /// Screening timeframe for this run (e.g. 15m, 4h).
    pub screening_timeframe: Timeframe,
}

// ── MultiTimeframeInput ───────────────────────────────────────────────────────

/// The complete, role-labelled multi-timeframe input for one evaluation.
///
/// Timeframe roles are explicit — never inferred from array order.
/// The actual timeframe values are configured at run time via ResearchConfig.
#[derive(Debug, Clone)]
pub struct MultiTimeframeInput {
    /// Entry-timeframe candle at the evaluation moment.
    pub entry_candle: Candle,
    /// Completed entry-timeframe candles before `entry_candle`.
    ///
    /// This history is built from the configured entry timeframe, excludes the
    /// current entry candle, and must not contain future candles.
    pub entry_lookback: Vec<Candle>,
    /// Confirmation-timeframe candle at the evaluation moment.
    pub confirmation_candle: Candle,
    /// Screening-timeframe candle at the evaluation moment.
    pub screening_candle: Candle,
    /// Indicator snapshot computed from the entry-timeframe candle stream.
    pub entry_indicators: IndicatorSnapshot,
    /// Indicator snapshot computed from the confirmation-timeframe candle stream.
    pub confirmation_indicators: IndicatorSnapshot,
    /// Indicator snapshot computed from the screening-timeframe candle stream.
    pub screening_indicators: IndicatorSnapshot,
}

// ── PositionAction ────────────────────────────────────────────────────────────

/// Decision produced by [`Strategy::audit_position`] once per entry-timeframe
/// bar while a position opened by this strategy is still open — the
/// "screening → entry → audit posisi → action" cycle's audit/action step.
#[derive(Debug, Clone, PartialEq)]
pub enum PositionAction {
    /// Keep holding. Static stop-loss/take-profit/time-exit levels set at
    /// entry continue to apply unchanged.
    Hold,
    /// Close the position now, at this bar's close, before any static exit
    /// level is reached. Recorded as `TradeExitReason::ManualClose`.
    CloseNow { reason: String },
}

// ── Strategy trait ────────────────────────────────────────────────────────────

/// A deterministic, stateless signal generator.
///
/// Implementations must:
///   - Be deterministic: same inputs → same output.
///   - Never call external APIs.
///   - Never mutate account state.
///   - Never place orders.
///   - Emit only `Option<Signal>`, never an order or fill.
pub trait Strategy {
    /// Stable, unique identifier for this strategy (e.g. `"basic_sample_strategy"`).
    fn strategy_id(&self) -> &'static str;

    /// Evaluate one multi-timeframe input and optionally emit a signal.
    ///
    /// Only called while this strategy has no open position (see
    /// `audit_position` for the open-position cycle).
    ///
    /// Returns:
    ///   - `Ok(None)`       — conditions not met; no signal.
    ///   - `Ok(Some(sig))`  — all conditions met; signal ready for risk review.
    ///   - `Err(e)`         — invalid input or internal error.
    fn evaluate(
        &self,
        ctx: &StrategyContext,
        input: &MultiTimeframeInput,
    ) -> Result<Option<Signal>, NorthflowError>;

    /// Called once per entry-timeframe bar while a position opened by this
    /// strategy is open, after the static stop-loss/take-profit/time-exit
    /// check for this bar has already found no exit. Lets a strategy
    /// proactively close a position when the original reason for entering no
    /// longer holds (e.g. the regime that justified entry has flipped),
    /// instead of always waiting passively for a price level or bar-count
    /// timeout.
    ///
    /// Default: always `Hold` — preserves pure static-bracket-order behavior
    /// for strategies that do not override this.
    fn audit_position(
        &self,
        _ctx: &StrategyContext,
        _input: &MultiTimeframeInput,
        _open_side: Side,
    ) -> PositionAction {
        PositionAction::Hold
    }
}
