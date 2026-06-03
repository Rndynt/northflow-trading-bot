//! Validation helpers — field-level trade validation used by the audit layer.
//!
//! Provides `TradeValidator` for composable per-field checks, and
//! `ValidationResult` as a lightweight summary suitable for early-exit guards.

use crate::core::Trade;
use crate::report::audit::{AuditReport, ReportAuditor};

// ── ValidationResult ──────────────────────────────────────────────────────────

/// Lightweight validation outcome.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
}

impl ValidationResult {
    /// Create from an `AuditReport`.
    pub fn from_audit(report: &AuditReport) -> Self {
        ValidationResult {
            valid: report.passed,
            error_count: report.error_count,
            warning_count: report.warning_count,
        }
    }
}

// ── TradeValidator ────────────────────────────────────────────────────────────

/// Composable field-level validators.
pub struct TradeValidator;

impl TradeValidator {
    /// Returns `true` when all required price fields are finite and positive.
    pub fn prices_valid(t: &Trade) -> bool {
        [t.entry_price, t.exit_price, t.stop_loss, t.take_profit]
            .iter()
            .all(|&v| v.is_finite() && v > 0.0)
    }

    /// Returns `true` when timestamps are positive and exit >= entry.
    pub fn times_valid(t: &Trade) -> bool {
        t.entry_time > 0 && t.exit_time > 0 && t.exit_time >= t.entry_time
    }

    /// Returns `true` when quantity and cost fields are valid.
    pub fn costs_valid(t: &Trade) -> bool {
        t.quantity.is_finite()
            && t.quantity > 0.0
            && t.fee.is_finite()
            && t.fee >= 0.0
            && t.slippage.is_finite()
            && t.slippage >= 0.0
            && t.gross_pnl.is_finite()
            && t.net_pnl.is_finite()
    }

    /// Returns `true` when trade_id contains signal_id (traceability rule).
    pub fn trade_id_traceable(t: &Trade) -> bool {
        let sid = t.signal_id.as_str();
        let tid = t.trade_id.as_str();
        !sid.is_empty() && !tid.is_empty() && tid.contains(sid)
    }

    /// Returns `true` when position_id contains signal_id (traceability rule).
    pub fn position_id_traceable(t: &Trade) -> bool {
        let sid = t.signal_id.as_str();
        let pid = t.position_id.as_str();
        !sid.is_empty() && !pid.is_empty() && pid.contains(sid)
    }

    /// Validate a slice of trades and return a `ValidationResult`.
    /// Delegates to `ReportAuditor` for consistent rules.
    pub fn validate(trades: &[Trade]) -> ValidationResult {
        let report = ReportAuditor::audit_trades(trades);
        ValidationResult::from_audit(&report)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        Trade, position::PositionId, side::Side, signal::SignalId, signal::StrategyId,
        symbol::Symbol, trade::TradeExitReason, trade::TradeId,
    };

    fn valid_trade() -> Trade {
        Trade {
            trade_id: TradeId::new("TRD-SIG-BT-00000001"),
            signal_id: SignalId::new("SIG-BT-00000001"),
            position_id: PositionId::new("POS-SIG-BT-00000001"),
            symbol: Symbol::new("BTCUSDT").unwrap(),
            strategy_id: StrategyId::new("screened_vwap_scalp"),
            regime: "bullish".to_string(),
            side: Side::Long,
            entry_time: 1_700_000_000_000,
            exit_time: 1_700_000_060_000,
            entry_price: 30_000.0,
            exit_price: 30_600.0,
            stop_loss: 29_700.0,
            take_profit: 30_600.0,
            quantity: 0.1,
            gross_pnl: 60.0,
            fee: 5.0,
            slippage: 3.0,
            net_pnl: 52.0,
            reward_risk: 2.0,
            bars_held: 10,
            exit_reason: TradeExitReason::TakeProfit,
            entry_reason: "ema_cross".to_string(),
            filters_passed: vec!["vwap_filter".to_string()],
            filters_failed: vec!["some_filter".to_string()],
            expected_edge_bps: 192.0,
            actual_edge_bps: 173.3,
        }
    }

    #[test]
    fn prices_valid_on_correct_trade() {
        assert!(TradeValidator::prices_valid(&valid_trade()));
    }

    #[test]
    fn prices_invalid_when_nan() {
        let mut t = valid_trade();
        t.entry_price = f64::NAN;
        assert!(!TradeValidator::prices_valid(&t));
    }

    #[test]
    fn times_valid_on_correct_trade() {
        assert!(TradeValidator::times_valid(&valid_trade()));
    }

    #[test]
    fn times_invalid_when_exit_before_entry() {
        let mut t = valid_trade();
        t.exit_time = t.entry_time - 1;
        assert!(!TradeValidator::times_valid(&t));
    }

    #[test]
    fn costs_valid_on_correct_trade() {
        assert!(TradeValidator::costs_valid(&valid_trade()));
    }

    #[test]
    fn trade_id_traceable_on_correct_ids() {
        assert!(TradeValidator::trade_id_traceable(&valid_trade()));
    }

    #[test]
    fn trade_id_not_traceable_when_missing_signal() {
        let mut t = valid_trade();
        t.trade_id = TradeId::new("TRD-UNRELATED");
        assert!(!TradeValidator::trade_id_traceable(&t));
    }

    #[test]
    fn position_id_traceable_on_correct_ids() {
        assert!(TradeValidator::position_id_traceable(&valid_trade()));
    }

    #[test]
    fn validate_returns_valid_for_empty_list() {
        let r = TradeValidator::validate(&[]);
        assert!(r.valid);
        assert_eq!(r.error_count, 0);
    }

    #[test]
    fn validate_returns_valid_for_correct_trade() {
        let r = TradeValidator::validate(&[valid_trade()]);
        assert!(r.valid, "valid trade must produce valid result");
        assert_eq!(r.error_count, 0);
    }
}
