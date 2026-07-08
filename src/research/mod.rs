//! Research orchestrator — Phase 7 + Strategy Comparison Runner.
//!
//! Supports three strategy run modes:
//!   single     — run one strategy, write reports to reports_dir
//!   comparison — run multiple strategies independently, each in its own subfolder
//!   multi      — reserved for future portfolio mode; currently returns ConfigError
//!
//! Single mode preserves current behavior exactly.
//!
//! Paper and live modes remain disabled.

pub mod comparison;
pub mod output;

use crate::backtest::{
    BacktestConfig, BacktestEngine, BacktestRunInput, ReportWriter, TimeframeRoles,
};
use crate::config::ResearchConfig;
use crate::core::{Symbol, Timeframe};
use crate::market::{CandleStore, DataQualityIssueKind, OhlcvLoader};
use crate::report::{
    AttributionEngine, AttributionSummary, AttributionWriter, AuditSeverity, DiagnosticEngine,
    DiagnosticWriter, ManifestWriter, ReportAuditor, TradeDistributionSummary,
};
use crate::strategy::registry::build_strategy_runtime;

use comparison::{ComparisonRunResult, ComparisonSummary, ComparisonWriter};
use output as out;

// ── CompletedResearchRun ──────────────────────────────────────────────────────

/// Data returned from a completed single symbol × strategy backtest run.
///
/// Used by single mode for verbose printing and by comparison mode for
/// building the aggregate summary.
struct CompletedResearchRun {
    // Core metrics
    total_trades: usize,
    win_rate: f64,
    net_pnl: f64,
    gross_pnl: f64,
    total_fee: f64,
    total_slippage: f64,
    profit_factor: f64,
    expectancy: f64,
    max_drawdown: f64,
    max_consecutive_losses: usize,
    // Signal flow
    signals_generated: usize,
    signals_preapproved: usize,
    signals_rejected_initial_risk: usize,
    signals_rejected_actual_entry: usize,
    trades_opened: usize,
    trades_closed: usize,
    risk_rejections: usize,
    rejections_max_drawdown: usize,
    rejections_daily_loss: usize,
    rejections_reward_risk: usize,
    rejections_expected_net_edge: usize,
    rejections_other: usize,
    // Diagnostics
    trade_distribution: TradeDistributionSummary,
    // Attribution
    attr_unique_signal_ids: usize,
    attr_avg_expected_edge_bps: f64,
    attr_avg_actual_edge_bps: f64,
    attr_edge_realization_bps: f64,
    // Audit
    audit_passed: bool,
    audit_error_count: usize,
    audit_warning_count: usize,
    audit_errors: Vec<(String, String)>,
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run Phase 7 research with strategy comparison runner support.
///
/// Validates config, dispatches by strategy_run_mode, and runs the appropriate
/// single-strategy or comparison backtest.
pub fn run_research(cfg: &ResearchConfig) -> Result<(), String> {
    cfg.validate_timeframes().map_err(|e| format!("{e}"))?;
    cfg.validate_strategy_config().map_err(|e| format!("{e}"))?;
    cfg.validate_strategy_runner_config()
        .map_err(|e| format!("{e}"))?;

    let strategies = cfg.selected_strategies().map_err(|e| format!("{e}"))?;

    out::section("Northflow Research");
    out::subsection("Run Plan");
    out::key_value("Mode", "research");
    out::key_value("Run mode", &cfg.strategy_run_mode);
    out::key_value("Strategy", strategies.join(", "));
    out::key_value("Symbols", cfg.symbols.join(", "));
    out::key_value("Source TF", &cfg.source_timeframe);
    out::key_value("Entry TF", &cfg.entry_timeframe);
    out::key_value("Confirm TF", &cfg.confirmation_timeframe);
    out::key_value("Screen TF", &cfg.screening_timeframe);
    out::key_value("Reports Dir", &cfg.reports_dir);
    out::key_value("Entry geometry", &cfg.entry_geometry_mode);
    out::blank();

    out::subsection("Runtime Guardrails");
    out::key_value("Paper trading", "disabled");
    out::key_value("Live trading", "disabled");
    out::key_value("Exchange calls", "disabled");
    out::blank();

    out::subsection("Engine");
    out::key_value("Strategy output", "Signal only");
    out::key_value("Risk output", "RiskAssessment only");
    out::key_value("Backtest model", "conservative intrabar fill");
    out::key_value("Lookahead", "disabled across configured higher timeframes");
    out::key_value("Signal IDs", "deterministic SIG-BT-XXXXXXXX");
    out::blank();

    match cfg.strategy_run_mode.as_str() {
        "single" => run_single_strategy(cfg),
        "comparison" => run_strategy_comparison(cfg),
        "multi" => Err("multi-strategy portfolio backtest is not implemented yet; \
             use strategy_run_mode = \"comparison\""
            .to_string()),
        other => Err(format!(
            "unknown strategy_run_mode: '{other}'. \
             Valid values: 'single', 'comparison', 'multi'"
        )),
    }
}

// ── Single mode ───────────────────────────────────────────────────────────────

fn run_single_strategy(cfg: &ResearchConfig) -> Result<(), String> {
    let strategies = cfg.selected_strategies().map_err(|e| format!("{e}"))?;
    let strategy_id = strategies
        .into_iter()
        .next()
        .unwrap_or_else(|| cfg.strategy_id.clone());

    let run_cfg = cfg.with_strategy_for_run(&strategy_id, cfg.reports_dir.clone());
    for symbol in &run_cfg.symbols {
        run_symbol_verbose(&run_cfg, symbol);
    }

    Ok(())
}

// ── Comparison mode ───────────────────────────────────────────────────────────

fn run_strategy_comparison(cfg: &ResearchConfig) -> Result<(), String> {
    let base_reports_dir = cfg.reports_dir.clone();
    let strategies = cfg.selected_strategies().map_err(|e| format!("{e}"))?;

    println!("Backtest run mode: comparison");
    println!("Strategies:");
    for s in &strategies {
        println!("  - {s}");
    }
    println!("Base reports dir: {base_reports_dir}");
    println!();

    let mut runs: Vec<ComparisonRunResult> = Vec::new();

    for symbol in &cfg.symbols {
        for strategy_id in &strategies {
            let reports_dir = if cfg.symbols.len() == 1 {
                format!("{base_reports_dir}/{strategy_id}")
            } else {
                format!("{base_reports_dir}/{symbol}/{strategy_id}")
            };

            println!("Running comparison strategy:");
            println!("  symbol: {symbol}");
            println!("  strategy_id: {strategy_id}");
            println!("  reports_dir: {reports_dir}");

            let run_cfg = cfg.with_strategy_for_run(strategy_id, reports_dir.clone());

            match run_symbol_strategy(&run_cfg, symbol) {
                Err(e) => {
                    println!("  Error: {e}");
                    println!();
                    runs.push(ComparisonRunResult::error(
                        symbol,
                        strategy_id,
                        &reports_dir,
                        &e,
                    ));
                }
                Ok(None) => {
                    let msg = format!(
                        "no historical CSV found at {}",
                        cfg.historical_paths_for(symbol)
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    println!("  {msg}");
                    println!();
                    runs.push(ComparisonRunResult::error(
                        symbol,
                        strategy_id,
                        &reports_dir,
                        &msg,
                    ));
                }
                Ok(Some(run)) => {
                    println!(
                        "  Done: {} trades, net_pnl {:.2}",
                        run.total_trades, run.net_pnl
                    );
                    println!();
                    let dist = &run.trade_distribution;
                    let summary_stub = crate::backtest::metrics::BacktestSummary {
                        total_trades: run.total_trades,
                        win_rate: run.win_rate,
                        net_pnl: run.net_pnl,
                        gross_pnl: run.gross_pnl,
                        total_fee: run.total_fee,
                        total_slippage: run.total_slippage,
                        profit_factor: run.profit_factor,
                        expectancy: run.expectancy,
                        avg_win: 0.0,
                        avg_loss: 0.0,
                        max_drawdown: run.max_drawdown,
                        max_consecutive_losses: run.max_consecutive_losses,
                        avg_trade_duration: 0.0,
                    };
                    let flow_stub = crate::backtest::risk_trace::SignalFlowSummary::default();
                    runs.push(ComparisonRunResult::ok(
                        symbol,
                        strategy_id,
                        &reports_dir,
                        &summary_stub,
                        &flow_stub,
                        dist,
                    ));
                }
            }
        }
    }

    let summary = ComparisonSummary { runs };

    match ComparisonWriter::write_all(&base_reports_dir, &summary) {
        Ok(()) => {
            println!("Comparison summary written:");
            println!("  {base_reports_dir}/comparison_summary.csv");
            println!("  {base_reports_dir}/comparison_summary.json");
        }
        Err(e) => {
            println!("Warning: could not write comparison summary: {e}");
        }
    }
    println!();

    Ok(())
}

// ── Core symbol × strategy runner (shared by both modes) ─────────────────────

/// Run a single symbol × strategy backtest, write all report files, and return
/// the completed run data.
///
/// Returns `Ok(None)` when the historical CSV does not exist.
/// Returns `Err` on data quality errors or backtest failures.
fn run_symbol_strategy(
    cfg: &ResearchConfig,
    symbol: &str,
) -> Result<Option<CompletedResearchRun>, String> {
    let data_paths = cfg.historical_paths_for(symbol);

    if data_paths.iter().any(|path| !path.exists()) {
        return Ok(None);
    }

    let load_result = if data_paths.len() > 1 {
        OhlcvLoader::load_files(&data_paths)
    } else {
        OhlcvLoader::load_file(&data_paths[0])
    }
    .map_err(|e| format!("{e}"))?;

    if load_result.quality.error_count() > 0 {
        return Err(format!(
            "data quality errors in {}: {} error(s) must be fixed before backtest",
            data_paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            load_result.quality.error_count()
        ));
    }
    if load_result.candles.is_empty() {
        return Ok(None);
    }

    let entry_tf =
        Timeframe::from_str(&cfg.entry_timeframe).map_err(|e| format!("entry_timeframe: {e}"))?;
    let confirmation_tf = Timeframe::from_str(&cfg.confirmation_timeframe)
        .map_err(|e| format!("confirmation_timeframe: {e}"))?;
    let screening_tf = Timeframe::from_str(&cfg.screening_timeframe)
        .map_err(|e| format!("screening_timeframe: {e}"))?;
    let regime_tf =
        Timeframe::from_str(&cfg.regime_timeframe).map_err(|e| format!("regime_timeframe: {e}"))?;
    let store = CandleStore::build(
        load_result.candles,
        entry_tf,
        confirmation_tf,
        screening_tf,
        regime_tf,
    )
    .map_err(|e| format!("{e}"))?;
    if store.entry_candles.is_empty() {
        return Ok(None);
    }

    let strategy_runtime = build_strategy_runtime(&cfg.strategy_id).map_err(|e| format!("{e}"))?;
    let symbol_obj = Symbol::new(symbol).map_err(|e| format!("invalid symbol '{symbol}': {e}"))?;
    let result = BacktestEngine::run(BacktestRunInput {
        symbol: symbol_obj,
        store,
        timeframes: TimeframeRoles {
            entry: entry_tf,
            confirmation: confirmation_tf,
            screening: screening_tf,
            regime: regime_tf,
        },
        backtest: BacktestConfig {
            initial_equity: cfg.initial_equity,
            reports_dir: cfg.reports_dir.clone(),
            conservative_intrabar: cfg.conservative_intrabar,
            max_bars_held: cfg.max_bars_held,
            entry_geometry_mode: crate::backtest::EntryGeometryMode::parse(
                &cfg.entry_geometry_mode,
            )
            .map_err(|e| format!("{e}"))?,
        },
        risk: cfg.risk_config(),
        cost: cfg.cost_model_config(),
        strategy: strategy_runtime.strategy.as_ref(),
        min_confidence: cfg.min_confidence,
        entry_lookback_bars: cfg.entry_lookback_bars,
        cooldown_bars: cfg.cooldown_bars_for_strategy(&cfg.strategy_id) as usize,
    })
    .map_err(|e| format!("{e}"))?;

    // Build Phase 7 report data.
    let diagnostics =
        DiagnosticEngine::build(&result.trades, &result.risk_rejections, &result.signal_flow);
    let attribution = AttributionEngine::build(&result.trades);
    let audit = ReportAuditor::audit_trades(&result.trades);
    let manifest = ManifestWriter::build(
        &cfg.reports_dir,
        &result.trades,
        &result.equity_curve,
        &attribution,
        result.risk_rejections.len(),
        &diagnostics,
    );

    // Write Phase 6 base reports.
    ReportWriter::write_all(
        &cfg.reports_dir,
        &result.summary,
        &result.trades,
        &result.equity_curve,
        &result.risk_rejections,
        &result.signal_flow,
    )
    .map_err(|e| format!("{e}"))?;

    // Write Phase 7 attribution, audit, manifest.
    AttributionWriter::write_all(&cfg.reports_dir, &attribution, &audit, &manifest)
        .map_err(|e| format!("{e}"))?;

    // Write diagnostic reports.
    DiagnosticWriter::write_all_with_trades(&cfg.reports_dir, &diagnostics, &result.trades)
        .map_err(|e| format!("{e}"))?;

    let attr_s: &AttributionSummary = &attribution.summary;
    let dist = diagnostics.trade_distribution;
    let s = &result.summary;
    let flow = &result.signal_flow;

    let audit_errors: Vec<(String, String)> = audit
        .issues
        .iter()
        .filter(|i| i.severity == AuditSeverity::Error)
        .map(|i| (i.code.clone(), i.message.clone()))
        .collect();

    Ok(Some(CompletedResearchRun {
        total_trades: s.total_trades,
        win_rate: s.win_rate,
        net_pnl: s.net_pnl,
        gross_pnl: s.gross_pnl,
        total_fee: s.total_fee,
        total_slippage: s.total_slippage,
        profit_factor: s.profit_factor,
        expectancy: s.expectancy,
        max_drawdown: s.max_drawdown,
        max_consecutive_losses: s.max_consecutive_losses,
        signals_generated: flow.signals_generated,
        signals_preapproved: flow.signals_preapproved,
        signals_rejected_initial_risk: flow.signals_rejected_initial_risk,
        signals_rejected_actual_entry: flow.signals_rejected_actual_entry,
        trades_opened: flow.trades_opened,
        trades_closed: flow.trades_closed,
        risk_rejections: flow.risk_rejections,
        rejections_max_drawdown: flow.rejections_max_drawdown,
        rejections_daily_loss: flow.rejections_daily_loss,
        rejections_reward_risk: flow.rejections_reward_risk,
        rejections_expected_net_edge: flow.rejections_expected_net_edge,
        rejections_other: flow.rejections_other,
        trade_distribution: dist,
        attr_unique_signal_ids: attr_s.unique_signal_ids,
        attr_avg_expected_edge_bps: attr_s.avg_expected_edge_bps,
        attr_avg_actual_edge_bps: attr_s.avg_actual_edge_bps,
        attr_edge_realization_bps: attr_s.edge_realization_bps,
        audit_passed: audit.passed,
        audit_error_count: audit.error_count,
        audit_warning_count: audit.warning_count,
        audit_errors,
    }))
}

// ── Verbose single-mode symbol runner ─────────────────────────────────────────

/// Run a symbol in single mode with full verbose CLI output.
/// Preserves existing single-mode behavior exactly.
fn run_symbol_verbose(cfg: &ResearchConfig, symbol: &str) {
    let data_paths = cfg.historical_paths_for(symbol);

    if data_paths.iter().any(|path| !path.exists()) {
        out::subsection(&format!("Symbol: {symbol}"));
        println!("Missing historical data.");
        println!("Expected files:");
        for (idx, path) in data_paths.iter().enumerate() {
            out::numbered(idx + 1, path.display());
        }
        println!("How to fix:");
        out::bullet("configure [historical_files], or");
        out::bullet("place fallback CSV at data_dir/<SYMBOL>.csv");
        println!(
            "Source data currently must be {} OHLCV with columns timestamp,open,high,low,close,volume",
            cfg.source_timeframe
        );
        out::blank();
        return;
    }

    let data_quality_ok = print_data_quality(cfg, symbol, &data_paths);
    if !data_quality_ok {
        println!("  Skipping backtest — fix data quality errors first.");
        println!();
        return;
    }

    out::subsection("Backtest Replay");
    match run_symbol_strategy(cfg, symbol) {
        Err(e) => {
            println!("  Backtest error: {e}");
            println!();
        }
        Ok(None) => {
            println!("  No data returned from backtest.");
            println!();
        }
        Ok(Some(run)) => {
            let pf_str = if run.profit_factor.is_infinite() {
                "inf".to_string()
            } else {
                format!("{:.4}", run.profit_factor)
            };

            out::subsection("Backtest Summary");
            out::key_value("Total trades", out::format_int(run.total_trades));
            out::key_value("Win rate", format!("{:.2}%", run.win_rate));
            out::key_value("Net PnL", out::format_f64(run.net_pnl, 2));
            out::key_value("Gross PnL", out::format_f64(run.gross_pnl, 2));
            out::key_value("Total fees", out::format_f64(run.total_fee, 2));
            out::key_value("Total slippage", out::format_f64(run.total_slippage, 2));
            out::key_value("Profit factor", pf_str);
            out::key_value("Max drawdown", format!("{:.2}%", run.max_drawdown));
            out::key_value(
                "Max consecutive losses",
                out::format_int(run.max_consecutive_losses),
            );
            out::blank();

            out::subsection("Signal Flow");
            out::key_value("Signals generated", out::format_int(run.signals_generated));
            out::key_value(
                "Signals preapproved",
                out::format_int(run.signals_preapproved),
            );
            out::key_value(
                "Rejected initial risk",
                out::format_int(run.signals_rejected_initial_risk),
            );
            out::key_value(
                "Rejected actual entry",
                out::format_int(run.signals_rejected_actual_entry),
            );
            out::key_value("Trades opened", out::format_int(run.trades_opened));
            out::key_value("Trades closed", out::format_int(run.trades_closed));
            out::key_value("Risk rejection rows", out::format_int(run.risk_rejections));
            out::blank();

            out::subsection("Rejection Breakdown");
            out::key_value("Max drawdown", out::format_int(run.rejections_max_drawdown));
            out::key_value("Daily loss", out::format_int(run.rejections_daily_loss));
            out::key_value("Reward/risk", out::format_int(run.rejections_reward_risk));
            out::key_value(
                "Expected net edge",
                out::format_int(run.rejections_expected_net_edge),
            );
            out::key_value("Other", out::format_int(run.rejections_other));
            out::blank();

            out::subsection("Audit");
            out::key_value("Passed", if run.audit_passed { "true" } else { "false" });
            out::key_value("Errors", out::format_int(run.audit_error_count));
            out::key_value("Warnings", out::format_int(run.audit_warning_count));
            if !run.audit_passed {
                println!("Audit errors (see audit_report.json):");
                for (code, msg) in &run.audit_errors {
                    out::bullet(format!("[ERROR] {code} - {msg}"));
                }
            }
            out::blank();

            out::subsection("Attribution");
            out::key_value(
                "Unique signals",
                out::format_int(run.attr_unique_signal_ids),
            );
            out::key_value(
                "Avg expected edge",
                format!("{:.2} bps", run.attr_avg_expected_edge_bps),
            );
            out::key_value(
                "Avg actual edge",
                format!("{:.2} bps", run.attr_avg_actual_edge_bps),
            );
            out::key_value(
                "Edge realization",
                format!("{:.2} bps", run.attr_edge_realization_bps),
            );
            out::blank();

            print_reports_written(&cfg.reports_dir);

            let d = &run.trade_distribution;
            out::subsection("Diagnostics");
            out::key_value("Avg total cost", format!("{:.2} bps", d.avg_total_cost_bps));
            out::key_value(
                "Avg edge realization",
                format!("{:.2} bps", d.avg_edge_realization_bps),
            );
            if !d.dominant_rejection_reason.is_empty() {
                out::key_value(
                    "Dominant rejection",
                    format!(
                        "{} ({})",
                        d.dominant_rejection_reason,
                        out::format_int(d.dominant_rejection_count)
                    ),
                );
            }
            out::blank();
        }
    }
}

const BASE_REPORT_FILES: &[&str] = &[
    "backtest_summary.json",
    "trades.csv",
    "equity_curve.csv",
    "risk_rejections.csv",
    "signal_flow_summary.json",
];

const ATTRIBUTION_REPORT_FILES: &[&str] = &[
    "attribution_summary.json",
    "attribution_by_regime.csv",
    "attribution_by_exit_reason.csv",
    "attribution_by_side.csv",
    "attribution_by_filter.csv",
    "attribution_by_strategy.csv",
    "audit_report.json",
    "report_manifest.json",
];

const DIAGNOSTIC_REPORT_FILES: &[&str] = &[
    "signal_diagnostics.csv",
    "rejection_by_stage_reason.csv",
    "monthly_summary.csv",
    "cost_edge_distribution.csv",
    "trade_distribution_summary.json",
];

fn print_reports_written(reports_dir: &str) {
    out::subsection("Reports Written");
    out::key_value("Directory", reports_dir);
    println!("Base:");
    for file in BASE_REPORT_FILES {
        out::bullet(file);
    }
    println!("Attribution:");
    for file in ATTRIBUTION_REPORT_FILES {
        out::bullet(file);
    }
    println!("Diagnostics:");
    for file in DIAGNOSTIC_REPORT_FILES {
        out::bullet(file);
    }
    out::blank();
}

// ── Data quality printer ──────────────────────────────────────────────────────

/// Print data quality for the symbol.  Returns `true` if no errors.
fn print_data_quality(
    cfg: &ResearchConfig,
    symbol: &str,
    data_paths: &[std::path::PathBuf],
) -> bool {
    use crate::market::CandleStore;

    let load_result = match if data_paths.len() > 1 {
        OhlcvLoader::load_files(data_paths)
    } else {
        OhlcvLoader::load_file(&data_paths[0])
    } {
        Ok(r) => r,
        Err(e) => {
            println!("  Error loading {symbol}: {e}");
            return false;
        }
    };

    let quality = &load_result.quality;
    let entry_tf = match crate::core::Timeframe::from_str(&cfg.entry_timeframe) {
        Ok(tf) => tf,
        Err(e) => {
            println!("  Invalid entry_timeframe: {e}");
            return false;
        }
    };
    let confirmation_tf = match crate::core::Timeframe::from_str(&cfg.confirmation_timeframe) {
        Ok(tf) => tf,
        Err(e) => {
            println!("  Invalid confirmation_timeframe: {e}");
            return false;
        }
    };
    let screening_tf = match crate::core::Timeframe::from_str(&cfg.screening_timeframe) {
        Ok(tf) => tf,
        Err(e) => {
            println!("  Invalid screening_timeframe: {e}");
            return false;
        }
    };
    let regime_tf = match crate::core::Timeframe::from_str(&cfg.regime_timeframe) {
        Ok(tf) => tf,
        Err(e) => {
            println!("  Invalid regime_timeframe: {e}");
            return false;
        }
    };
    let store = match CandleStore::build(
        load_result.candles,
        entry_tf,
        confirmation_tf,
        screening_tf,
        regime_tf,
    ) {
        Ok(s) => s,
        Err(e) => {
            println!("  Error building candle store: {e}");
            return false;
        }
    };

    let dup_count = quality
        .issues
        .iter()
        .filter(|i| i.kind == DataQualityIssueKind::DuplicateTimestamp)
        .count();

    out::subsection(&format!("Symbol: {symbol}"));
    println!("Data files:");
    for (idx, path) in data_paths.iter().enumerate() {
        out::numbered(idx + 1, path.display());
    }
    out::blank();
    out::subsection("Data Summary");
    out::key_value(
        &format!("Raw {} candles", cfg.source_timeframe),
        out::format_int(store.raw_1m.len()),
    );
    out::key_value(
        &format!("Entry {} candles", store.entry_tf),
        out::format_int(store.entry_len()),
    );
    out::key_value(
        &format!("Confirm {} candles", store.confirmation_tf),
        out::format_int(store.confirmation_len()),
    );
    out::key_value(
        &format!("Screen {} candles", store.screening_tf),
        out::format_int(store.screening_len()),
    );
    out::key_value(
        &format!("Regime {} candles", store.regime_tf),
        out::format_int(store.regime_len()),
    );
    out::key_value(
        "Data quality errors",
        out::format_int(quality.error_count()),
    );
    out::key_value("Duplicate timestamps", out::format_int(dup_count));
    out::key_value("Missing gaps", out::format_int(quality.missing_gaps.len()));

    if quality.error_count() > 0 {
        println!();
        println!("  Data quality errors:");
        for issue in quality.issues.iter().filter(|i| i.kind.is_error()) {
            match issue.row {
                Some(row) => println!("    [{}] row {row}: {}", issue.kind, issue.message),
                None => println!("    [{}] {}", issue.kind, issue.message),
            }
        }
        return false;
    }

    if !quality.missing_gaps.is_empty() {
        println!();
        println!("  Missing configured source-timeframe gaps (warnings):");
        for gap in &quality.missing_gaps {
            println!(
                "    {} missing candle(s) after ts={}  (expected ts={})",
                gap.missing_count, gap.from_timestamp, gap.expected_next_timestamp
            );
        }
    }

    println!();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_groups_use_filenames_without_directory_prefixes() {
        for file in BASE_REPORT_FILES
            .iter()
            .chain(ATTRIBUTION_REPORT_FILES.iter())
            .chain(DIAGNOSTIC_REPORT_FILES.iter())
        {
            assert!(!file.contains('/'));
            assert!(!file.is_empty());
        }
    }
}
