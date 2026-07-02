//! Audit engine — validates every trade for completeness and traceability.
//!
//! Every trade must be explainable and auditable back to its signal_id.
//!
//! Error  → broken attribution or invalid trade field; audit_report.passed = false.
//! Warning → incomplete but non-fatal explainability; does not fail the audit.
//! Info    → informational, no action required.

use std::collections::HashSet;

use crate::core::Trade;

// ── AuditSeverity ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

// ── AuditIssue ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuditIssue {
    pub severity: AuditSeverity,
    pub code: String,
    pub message: String,
    pub trade_id: Option<String>,
    pub signal_id: Option<String>,
}

// ── AuditReport ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuditReport {
    /// `true` when error_count == 0.
    pub passed: bool,
    pub total_trades: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub issues: Vec<AuditIssue>,
}

// ── ReportAuditor ─────────────────────────────────────────────────────────────

pub struct ReportAuditor;

impl ReportAuditor {
    /// Audit all trades and return a report.
    ///
    /// An empty trade list returns `AuditReport { passed: true, .. }` with zero counts.
    pub fn audit_trades(trades: &[Trade]) -> AuditReport {
        let mut issues: Vec<AuditIssue> = Vec::new();

        let mut seen_trade_ids: HashSet<String> = HashSet::new();
        let mut seen_signal_ids: HashSet<String> = HashSet::new();

        for t in trades {
            let tid = t.trade_id.as_str().to_string();
            let sid = t.signal_id.as_str().to_string();
            let ctx_tid = if tid.is_empty() {
                None
            } else {
                Some(tid.clone())
            };
            let ctx_sid = if sid.is_empty() {
                None
            } else {
                Some(sid.clone())
            };

            // ── Required string fields ────────────────────────────────────────
            if tid.is_empty() {
                issues.push(Self::err(
                    "empty_trade_id",
                    "trade_id is empty",
                    None,
                    ctx_sid.clone(),
                ));
            }
            if sid.is_empty() {
                issues.push(Self::err(
                    "empty_signal_id",
                    "signal_id is empty",
                    ctx_tid.clone(),
                    None,
                ));
            }
            if t.symbol.as_str().is_empty() {
                issues.push(Self::err(
                    "empty_symbol",
                    "symbol is empty",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if t.strategy_id.as_str().is_empty() {
                issues.push(Self::err(
                    "empty_strategy_id",
                    "strategy_id is empty",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if t.regime.is_empty() {
                issues.push(Self::err(
                    "empty_regime",
                    "regime is empty",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if t.entry_reason.is_empty() {
                issues.push(Self::err(
                    "empty_entry_reason",
                    "entry_reason is empty",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if t.exit_reason.as_str().is_empty() {
                issues.push(Self::err(
                    "empty_exit_reason",
                    "exit_reason is empty",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }

            // ── Duplicate checks ──────────────────────────────────────────────
            if !tid.is_empty() {
                if seen_trade_ids.contains(&tid) {
                    issues.push(Self::err(
                        "duplicate_trade_id",
                        &format!("duplicate trade_id: {tid}"),
                        ctx_tid.clone(),
                        ctx_sid.clone(),
                    ));
                } else {
                    seen_trade_ids.insert(tid.clone());
                }
            }
            if !sid.is_empty() {
                if seen_signal_ids.contains(&sid) {
                    issues.push(Self::err(
                        "duplicate_signal_id",
                        &format!("duplicate signal_id: {sid}"),
                        ctx_tid.clone(),
                        ctx_sid.clone(),
                    ));
                } else {
                    seen_signal_ids.insert(sid.clone());
                }
            }

            // ── Time checks ───────────────────────────────────────────────────
            if t.entry_time <= 0 {
                issues.push(Self::err(
                    "invalid_entry_time",
                    "entry_time must be > 0",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if t.exit_time <= 0 {
                issues.push(Self::err(
                    "invalid_exit_time",
                    "exit_time must be > 0",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if t.exit_time < t.entry_time {
                issues.push(Self::err(
                    "exit_before_entry",
                    "exit_time < entry_time",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }

            // ── Price / quantity checks ───────────────────────────────────────
            for (val, code, msg) in [
                (
                    t.entry_price,
                    "invalid_entry_price",
                    "entry_price must be finite and > 0",
                ),
                (
                    t.exit_price,
                    "invalid_exit_price",
                    "exit_price must be finite and > 0",
                ),
                (
                    t.stop_loss,
                    "invalid_stop_loss",
                    "stop_loss must be finite and > 0",
                ),
                (
                    t.take_profit,
                    "invalid_take_profit",
                    "take_profit must be finite and > 0",
                ),
            ] {
                if !val.is_finite() || val <= 0.0 {
                    issues.push(Self::err(code, msg, ctx_tid.clone(), ctx_sid.clone()));
                }
            }
            if !t.quantity.is_finite() || t.quantity <= 0.0 {
                issues.push(Self::err(
                    "invalid_quantity",
                    "quantity must be finite and > 0",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if !t.gross_pnl.is_finite() {
                issues.push(Self::err(
                    "non_finite_gross_pnl",
                    "gross_pnl is not finite",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if !t.fee.is_finite() || t.fee < 0.0 {
                issues.push(Self::err(
                    "invalid_fee",
                    "fee must be finite and >= 0",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if !t.slippage.is_finite() || t.slippage < 0.0 {
                issues.push(Self::err(
                    "invalid_slippage",
                    "slippage must be finite and >= 0",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if !t.net_pnl.is_finite() {
                issues.push(Self::err(
                    "non_finite_net_pnl",
                    "net_pnl is not finite",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if !t.reward_risk.is_finite() || t.reward_risk <= 0.0 {
                issues.push(Self::err(
                    "invalid_reward_risk",
                    "reward_risk must be finite and > 0",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if !t.actual_edge_bps.is_finite() {
                issues.push(Self::err(
                    "non_finite_actual_edge",
                    "actual_edge_bps is not finite",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }

            // ── Traceability checks ───────────────────────────────────────────
            // trade_id must embed signal_id
            if !sid.is_empty() && !tid.is_empty() && !tid.contains(&sid) {
                issues.push(Self::err(
                    "trade_id_missing_signal_id",
                    &format!("trade_id '{tid}' does not contain signal_id '{sid}'"),
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            // position_id must embed signal_id
            let pos_id = t.position_id.as_str();
            if !sid.is_empty() && !pos_id.is_empty() && !pos_id.contains(&sid) {
                issues.push(Self::err(
                    "position_id_missing_signal_id",
                    &format!("position_id '{pos_id}' does not contain signal_id '{sid}'"),
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }

            // ── Warnings ─────────────────────────────────────────────────────
            if t.filters_passed.is_empty() {
                issues.push(Self::warn(
                    "empty_filters_passed",
                    "filters_passed is empty",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            // Empty filters_failed means all filters were passed — normal for
            // high-quality signals.  Downgraded from Warning to Info so that
            // strategies that pass all filters do not inflate warning counts.
            if t.filters_failed.is_empty() {
                issues.push(Self::info(
                    "empty_filters_failed",
                    "filters_failed is empty (all filters passed)",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if !t.expected_edge_bps.is_finite() {
                issues.push(Self::warn(
                    "non_finite_expected_edge",
                    "expected_edge_bps is not finite",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            } else if t.expected_edge_bps <= 0.0 {
                issues.push(Self::warn(
                    "non_positive_expected_edge",
                    "expected_edge_bps <= 0",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
            if t.bars_held == 0 {
                issues.push(Self::warn(
                    "zero_bars_held",
                    "bars_held is 0",
                    ctx_tid.clone(),
                    ctx_sid.clone(),
                ));
            }
        }

        let error_count = issues
            .iter()
            .filter(|i| i.severity == AuditSeverity::Error)
            .count();
        let warning_count = issues
            .iter()
            .filter(|i| i.severity == AuditSeverity::Warning)
            .count();
        let passed = error_count == 0;

        AuditReport {
            passed,
            total_trades: trades.len(),
            error_count,
            warning_count,
            issues,
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn err(
        code: &str,
        msg: &str,
        trade_id: Option<String>,
        signal_id: Option<String>,
    ) -> AuditIssue {
        AuditIssue {
            severity: AuditSeverity::Error,
            code: code.to_string(),
            message: msg.to_string(),
            trade_id,
            signal_id,
        }
    }

    fn warn(
        code: &str,
        msg: &str,
        trade_id: Option<String>,
        signal_id: Option<String>,
    ) -> AuditIssue {
        AuditIssue {
            severity: AuditSeverity::Warning,
            code: code.to_string(),
            message: msg.to_string(),
            trade_id,
            signal_id,
        }
    }

    fn info(
        code: &str,
        msg: &str,
        trade_id: Option<String>,
        signal_id: Option<String>,
    ) -> AuditIssue {
        AuditIssue {
            severity: AuditSeverity::Info,
            code: code.to_string(),
            message: msg.to_string(),
            trade_id,
            signal_id,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        position::PositionId, side::Side, signal::SignalId, signal::StrategyId, symbol::Symbol,
        trade::TradeExitReason, trade::TradeId, Trade,
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
            entry_reason: "ema_cross_above_vwap".to_string(),
            filters_passed: vec!["vwap_filter".to_string()],
            filters_failed: vec!["some_failed".to_string()],
            expected_edge_bps: 192.0,
            actual_edge_bps: 173.3,
        }
    }

    fn has_error(report: &AuditReport, code: &str) -> bool {
        report
            .issues
            .iter()
            .any(|i| i.severity == AuditSeverity::Error && i.code == code)
    }

    fn has_warning(report: &AuditReport, code: &str) -> bool {
        report
            .issues
            .iter()
            .any(|i| i.severity == AuditSeverity::Warning && i.code == code)
    }

    #[test]
    fn audit_passes_empty_trade_list() {
        let r = ReportAuditor::audit_trades(&[]);
        assert!(r.passed);
        assert_eq!(r.total_trades, 0);
        assert_eq!(r.error_count, 0);
        assert_eq!(r.warning_count, 0);
        assert!(r.issues.is_empty());
    }

    #[test]
    fn audit_passes_valid_trade() {
        let r = ReportAuditor::audit_trades(&[valid_trade()]);
        assert!(
            r.passed,
            "valid trade must pass audit; errors: {:?}",
            r.issues
                .iter()
                .filter(|i| i.severity == AuditSeverity::Error)
                .collect::<Vec<_>>()
        );
        assert_eq!(r.error_count, 0);
    }

    #[test]
    fn audit_rejects_empty_trade_id() {
        let mut t = valid_trade();
        t.trade_id = TradeId::new("");
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "empty_trade_id"));
    }

    #[test]
    fn audit_rejects_empty_signal_id() {
        let mut t = valid_trade();
        t.signal_id = SignalId::new("");
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "empty_signal_id"));
    }

    #[test]
    fn audit_rejects_duplicate_trade_id() {
        let t1 = valid_trade();
        let mut t2 = valid_trade();
        t2.signal_id = SignalId::new("SIG-BT-00000002");
        // Keep same trade_id → duplicate
        let r = ReportAuditor::audit_trades(&[t1, t2]);
        assert!(!r.passed);
        assert!(has_error(&r, "duplicate_trade_id"));
    }

    #[test]
    fn audit_rejects_duplicate_signal_id_for_single_position_model() {
        let t1 = valid_trade();
        let mut t2 = valid_trade();
        t2.trade_id = TradeId::new("TRD-SIG-BT-00000002");
        // Keep same signal_id → duplicate
        let r = ReportAuditor::audit_trades(&[t1, t2]);
        assert!(!r.passed);
        assert!(has_error(&r, "duplicate_signal_id"));
    }

    #[test]
    fn audit_rejects_invalid_entry_price() {
        let mut t = valid_trade();
        t.entry_price = -1.0;
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "invalid_entry_price"));
    }

    #[test]
    fn audit_rejects_invalid_exit_price() {
        let mut t = valid_trade();
        t.exit_price = f64::NAN;
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "invalid_exit_price"));
    }

    #[test]
    fn audit_rejects_invalid_quantity() {
        let mut t = valid_trade();
        t.quantity = 0.0;
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "invalid_quantity"));
    }

    #[test]
    fn audit_rejects_negative_fee() {
        let mut t = valid_trade();
        t.fee = -1.0;
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "invalid_fee"));
    }

    #[test]
    fn audit_rejects_negative_slippage() {
        let mut t = valid_trade();
        t.slippage = -0.01;
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "invalid_slippage"));
    }

    #[test]
    fn audit_rejects_non_finite_net_pnl() {
        let mut t = valid_trade();
        t.net_pnl = f64::INFINITY;
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "non_finite_net_pnl"));
    }

    #[test]
    fn audit_rejects_trade_id_missing_signal_id() {
        let mut t = valid_trade();
        t.trade_id = TradeId::new("TRD-COMPLETELY-DIFFERENT-ID");
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "trade_id_missing_signal_id"));
    }

    #[test]
    fn audit_rejects_position_id_missing_signal_id() {
        let mut t = valid_trade();
        t.position_id = PositionId::new("POS-COMPLETELY-DIFFERENT");
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(!r.passed);
        assert!(has_error(&r, "position_id_missing_signal_id"));
    }

    #[test]
    fn audit_warns_empty_filters_passed() {
        let mut t = valid_trade();
        t.filters_passed = vec![];
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(r.passed, "empty filters_passed is a warning only");
        assert!(has_warning(&r, "empty_filters_passed"));
    }

    fn has_info(report: &AuditReport, code: &str) -> bool {
        report
            .issues
            .iter()
            .any(|i| i.severity == AuditSeverity::Info && i.code == code)
    }

    #[test]
    fn audit_does_not_warn_when_filters_failed_empty() {
        let mut t = valid_trade();
        t.filters_failed = vec![];
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(r.passed, "empty filters_failed must not fail the audit");
        assert!(
            !has_warning(&r, "empty_filters_failed"),
            "empty filters_failed must not produce a warning (downgraded to Info)"
        );
        assert!(
            has_info(&r, "empty_filters_failed"),
            "empty filters_failed should produce an Info issue"
        );
    }

    #[test]
    fn audit_warns_non_positive_expected_edge() {
        let mut t = valid_trade();
        t.expected_edge_bps = -5.0;
        let r = ReportAuditor::audit_trades(&[t]);
        assert!(r.passed, "non-positive expected edge is a warning only");
        assert!(has_warning(&r, "non_positive_expected_edge"));
    }
}
