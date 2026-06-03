# Northflow Phase 7 Build Prompt

You are working on this repository:

https://github.com/Rndynt/northflow-crypto-trading-bot

Your task is to implement Phase 7: Reports and Attribution.

Read these files first:

- AGENTS.md
- docs/ROADMAP.md
- README.md
- config/research.toml
- src/main.rs
- src/lib.rs
- src/research/mod.rs
- src/report/mod.rs
- src/backtest/mod.rs
- src/backtest/engine.rs
- src/backtest/report.rs
- src/backtest/metrics.rs
- src/core/trade.rs
- src/core/signal.rs
- src/core/side.rs
- src/core/symbol.rs

Do not ignore repository documentation.

## Project mission

Northflow is a deterministic research-first crypto trading engine.

Northflow is not:

- a dashboard
- a React app
- a Telegram bot
- an AI trading agent
- a live trading system
- a paper trading loop
- a parameter optimizer
- a portfolio optimizer

The current goal is to make every backtest trade explainable and auditable.

Phase 7 is the final research-core phase before any later paper/live/advisor phases.

Do not implement live trading.

Do not implement paper trading.

Do not call exchange APIs.

Do not call LLMs.

Do not create dashboard, Telegram, or notification systems.

## Current phase

Implement:

Phase 7 - Reports and Attribution

Roadmap requirement:

Every trade must be explainable.

Required trades.csv fields:

- trade_id
- signal_id
- symbol
- strategy_id
- regime
- side
- entry_time
- exit_time
- entry_price
- exit_price
- stop_loss
- take_profit
- qty
- gross_pnl
- fee
- slippage
- net_pnl
- reward_risk
- bars_held
- exit_reason
- entry_reason
- filters_passed
- filters_failed
- expected_edge_bps
- actual_edge_bps

Required summary JSON fields:

- total_trades
- win_rate
- net_pnl
- gross_pnl
- total_fee
- total_slippage
- profit_factor
- expectancy
- avg_win
- avg_loss
- max_drawdown
- max_consecutive_losses
- avg_trade_duration

Phase 6 already writes basic report files from src/backtest/report.rs.

Phase 7 must turn those reports into a stricter attribution layer:

- validate report completeness
- validate traceability
- enrich attribution summaries
- make reports deterministic and audit-friendly
- keep all outputs local files
- do not add external dependencies unless absolutely necessary

## Important boundary

Phase 7 may write report and attribution files.

Phase 7 must not:

- place real orders
- run paper trading
- run live trading
- call exchange APIs
- call LLMs
- optimize parameters
- mutate external account state
- create a dashboard
- create Telegram integration
- give financial advice
- claim future profitability

Backtest results remain historical simulation only.

## Current report situation

There is already a Phase 6 report writer in:

src/backtest/report.rs

There is also a placeholder:

src/report/mod.rs

Phase 7 should not duplicate two independent report systems.

Preferred direction:

- Keep src/backtest/report.rs as the low-level writer that writes backtest_summary.json, trades.csv, and equity_curve.csv.
- Implement src/report/ as the Phase 7 attribution and validation layer.
- The Phase 7 layer can call or wrap the existing backtest report writer.
- Avoid duplicating CSV writing logic unnecessarily.

## Required active structure

Create or repair this structure:

- src/report/mod.rs
- src/report/attribution.rs
- src/report/audit.rs
- src/report/manifest.rs
- src/report/validation.rs

Optional if useful:

- src/report/json.rs
- src/report/csv.rs

Only add optional files if they keep code clean.

Update:

- src/lib.rs if needed
- src/research/mod.rs
- README.md
- src/main.rs

only as needed.

## Required exports

Update src/report/mod.rs to export:

pub mod attribution;
pub mod audit;
pub mod manifest;
pub mod validation;

pub use attribution::*;
pub use audit::*;
pub use manifest::*;
pub use validation::*;

Use explicit exports if preferred.

## Phase 7 output files

After running:

cargo run -- research --config config/research.toml

with valid historical CSV data, the reports directory should include at least:

- reports/backtest_summary.json
- reports/trades.csv
- reports/equity_curve.csv
- reports/attribution_summary.json
- reports/attribution_by_regime.csv
- reports/attribution_by_exit_reason.csv
- reports/attribution_by_side.csv
- reports/attribution_by_filter.csv
- reports/audit_report.json
- reports/report_manifest.json

If no trades exist, still write valid empty attribution files with headers and zero counts.

If no CSV exists, keep the friendly missing-data message and do not panic.

## Required attribution summaries

Implement src/report/attribution.rs.

Create deterministic types.

Recommended types:

pub struct AttributionSummary {
    pub total_trades: usize,
    pub total_signals_with_trades: usize,
    pub unique_signal_ids: usize,
    pub unique_trade_ids: usize,
    pub avg_expected_edge_bps: f64,
    pub avg_actual_edge_bps: f64,
    pub edge_realization_bps: f64,
    pub positive_expected_edge_trades: usize,
    pub positive_actual_edge_trades: usize,
    pub filters_passed_count: usize,
    pub filters_failed_count: usize,
}

pub struct AttributionBucket {
    pub key: String,
    pub trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub net_pnl: f64,
    pub gross_pnl: f64,
    pub total_fee: f64,
    pub total_slippage: f64,
    pub avg_net_pnl: f64,
    pub avg_expected_edge_bps: f64,
    pub avg_actual_edge_bps: f64,
    pub avg_bars_held: f64,
}

pub struct AttributionReport {
    pub summary: AttributionSummary,
    pub by_regime: Vec<AttributionBucket>,
    pub by_exit_reason: Vec<AttributionBucket>,
    pub by_side: Vec<AttributionBucket>,
    pub by_filter: Vec<AttributionBucket>,
}

pub struct AttributionEngine;

impl AttributionEngine {
    pub fn build(trades: &[Trade]) -> AttributionReport;
}

Rules:

- All calculations must be deterministic.
- Sort buckets by key ascending for stable output.
- For no trades, return empty buckets and zero summary values.
- wins = net_pnl > 0.
- losses = net_pnl <= 0.
- win_rate = wins / trades * 100, or 0 if bucket empty.
- avg_net_pnl = net_pnl / trades, or 0.
- avg_expected_edge_bps = average expected_edge_bps, or 0.
- avg_actual_edge_bps = average actual_edge_bps, or 0.
- edge_realization_bps = avg_actual_edge_bps - avg_expected_edge_bps.
- by_filter should count both passed and failed filters.

Filter bucket keys:

For passed filters:

passed:<filter_name>

For failed filters:

failed:<filter_name>

Example:

passed:atr_valid
failed:volume_below_threshold

If a trade has no filters, do not create an empty filter bucket.

## Required audit validation

Implement src/report/audit.rs and src/report/validation.rs.

Purpose:

Ensure every trade is explainable and traceable.

Recommended types:

pub struct AuditIssue {
    pub severity: AuditSeverity,
    pub code: String,
    pub message: String,
    pub trade_id: Option<String>,
    pub signal_id: Option<String>,
}

pub enum AuditSeverity {
    Info,
    Warning,
    Error,
}

pub struct AuditReport {
    pub passed: bool,
    pub total_trades: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub issues: Vec<AuditIssue>,
}

pub struct ReportAuditor;

impl ReportAuditor {
    pub fn audit_trades(trades: &[Trade]) -> AuditReport;
}

Audit rules:

A trade should produce an Error if:

- trade_id is empty
- signal_id is empty
- symbol is empty
- strategy_id is empty
- regime is empty
- entry_time <= 0
- exit_time <= 0
- exit_time < entry_time
- entry_price <= 0 or not finite
- exit_price <= 0 or not finite
- stop_loss <= 0 or not finite
- take_profit <= 0 or not finite
- quantity <= 0 or not finite
- gross_pnl is not finite
- fee is not finite or fee < 0
- slippage is not finite or slippage < 0
- net_pnl is not finite
- reward_risk is not finite or reward_risk <= 0
- actual_edge_bps is not finite
- trade_id is duplicated
- signal_id is duplicated among closed trades if max_open_positions is currently 1
- trade_id does not contain signal_id
- position_id does not contain signal_id
- entry_reason is empty
- exit_reason string is empty

A trade should produce a Warning if:

- filters_passed is empty
- filters_failed is empty
- expected_edge_bps is not finite
- expected_edge_bps <= 0
- bars_held == 0

Notes:

- `filters_failed` may legitimately be empty for a passed signal. Still warn, not error.
- For Phase 7, do not block report writing on warnings.
- If any Error exists, AuditReport.passed = false.
- If only warnings exist, AuditReport.passed = true.

Stable issue codes:

Use stable snake_case codes, such as:

empty_trade_id
empty_signal_id
duplicate_trade_id
duplicate_signal_id
invalid_entry_price
invalid_exit_price
invalid_quantity
invalid_fee
invalid_slippage
invalid_reward_risk
non_finite_actual_edge
trade_id_missing_signal_id
position_id_missing_signal_id
empty_entry_reason
empty_filters_passed
empty_filters_failed

Do not use random issue IDs.

## Required manifest

Implement src/report/manifest.rs.

Purpose:

Write a deterministic manifest describing generated report files.

Recommended types:

pub struct ReportFileEntry {
    pub path: String,
    pub kind: String,
    pub rows: usize,
}

pub struct ReportManifest {
    pub phase: String,
    pub generated_by: String,
    pub files: Vec<ReportFileEntry>,
}

pub struct ManifestWriter;

impl ManifestWriter {
    pub fn build(reports_dir: &str, trades: &[Trade], equity_curve: &[EquityPoint]) -> ReportManifest;
    pub fn write(reports_dir: &str, manifest: &ReportManifest) -> Result<(), NorthflowError>;
}

Rules:

- Do not use system time.
- Do not use random IDs.
- Do not include machine-specific absolute paths.
- Use relative paths like reports/trades.csv.
- Sort file entries by path ascending.
- Include row counts:
  - trades.csv rows = trades.len()
  - equity_curve.csv rows = equity_curve.len()
  - summary json rows = 1
  - attribution json rows = 1
  - audit json rows = 1
  - attribution bucket csv rows = bucket count

Recommended manifest JSON fields:

phase
generated_by
files

Example:

{
  "phase": "phase_7_reports_and_attribution",
  "generated_by": "northflow_research",
  "files": [
    {"path":"reports/backtest_summary.json","kind":"summary","rows":1},
    {"path":"reports/trades.csv","kind":"trades","rows":12}
  ]
}

Manual JSON string formatting is acceptable.

## Required Phase 7 writer

Implement a Phase 7 writer function.

Recommended type:

pub struct AttributionWriter;

impl AttributionWriter {
    pub fn write_all(
        reports_dir: &str,
        attribution: &AttributionReport,
        audit: &AuditReport,
        manifest: &ReportManifest,
    ) -> Result<(), NorthflowError>;
}

It must write:

- attribution_summary.json
- attribution_by_regime.csv
- attribution_by_exit_reason.csv
- attribution_by_side.csv
- attribution_by_filter.csv
- audit_report.json
- report_manifest.json

CSV escaping:

- Escape commas, quotes, and newlines.
- Quote fields when needed.
- Keep headers stable.

Attribution CSV headers should be stable:

key,trades,wins,losses,win_rate,net_pnl,gross_pnl,total_fee,total_slippage,avg_net_pnl,avg_expected_edge_bps,avg_actual_edge_bps,avg_bars_held

Audit JSON must include:

passed
total_trades
error_count
warning_count
issues

Each issue should include:

severity
code
message
trade_id
signal_id

Use null or empty string for missing trade_id/signal_id. Pick one and keep it consistent.

## Integration with research command

Update src/research/mod.rs.

After BacktestEngine::run returns Some(result) and after base reports are written:

1. Build AttributionReport from result.trades.
2. Build AuditReport from result.trades.
3. Build ReportManifest.
4. Write Phase 7 attribution files.
5. Print a clear Phase 7 summary:
   - audit passed true/false
   - audit errors
   - audit warnings
   - attribution report file paths
6. Do not panic if there are zero trades.
7. If audit has errors:
   - print a warning
   - still write audit_report.json and attribution files
   - return Ok from research unless file writing fails
8. If file writing fails:
   - return Err or print a clear warning consistently with existing research behavior.

Do not rerun backtest in the report layer.

Do not change strategy/risk/backtest logic except where required for report integration.

## README update

Update README.md:

- Current phase is Phase 7 - Reports and Attribution.
- Phase 1 through Phase 6 are complete.
- Phase 7 is implemented.
- Explain report files:
  - backtest_summary.json
  - trades.csv
  - equity_curve.csv
  - attribution_summary.json
  - attribution_by_regime.csv
  - attribution_by_exit_reason.csv
  - attribution_by_side.csv
  - attribution_by_filter.csv
  - audit_report.json
  - report_manifest.json
- Explain that every trade traces:
  signal_id -> position_id -> trade_id
- Explain audit validation:
  - errors mean broken attribution or invalid trade fields
  - warnings mean incomplete but non-fatal explainability
- Paper and live modes remain disabled.
- Later phases may add paper/live/advisor, but AI must not decide entries, SL/TP, or position size.

Do not mark paper/live/advisor as implemented.

Do not claim profitability.

## CLI help update

Update src/main.rs help text to Phase 7.

Expected help wording:

Phase 7: reports and attribution ready.
         Backtest output: simulated Trade records only.
         Reports include summary, trades, equity curve, attribution, audit, and manifest.
         No live orders, no paper trading, no exchange calls.
         Place 1m CSV data in data/historical/<SYMBOL>.csv
         Columns: timestamp,open,high,low,close,volume
         Alternative timestamp column: open_time

Keep paper/live disabled.

## Tests required

Add comprehensive tests.

### Attribution tests

- attribution_empty_trades_has_zero_summary
- attribution_counts_unique_signal_ids
- attribution_counts_unique_trade_ids
- attribution_calculates_avg_expected_edge
- attribution_calculates_avg_actual_edge
- attribution_calculates_edge_realization
- attribution_groups_by_regime
- attribution_groups_by_exit_reason
- attribution_groups_by_side
- attribution_groups_by_passed_filter
- attribution_groups_by_failed_filter
- attribution_buckets_are_sorted_by_key

### Audit tests

- audit_passes_empty_trade_list
- audit_rejects_empty_trade_id
- audit_rejects_empty_signal_id
- audit_rejects_duplicate_trade_id
- audit_rejects_duplicate_signal_id_for_single_position_model
- audit_rejects_invalid_entry_price
- audit_rejects_invalid_exit_price
- audit_rejects_invalid_quantity
- audit_rejects_negative_fee
- audit_rejects_negative_slippage
- audit_rejects_non_finite_net_pnl
- audit_rejects_trade_id_missing_signal_id
- audit_rejects_position_id_missing_signal_id
- audit_warns_empty_filters_passed
- audit_warns_empty_filters_failed
- audit_warns_non_positive_expected_edge

### Writer tests

- attribution_writer_writes_summary_json
- attribution_writer_writes_regime_csv
- attribution_writer_writes_exit_reason_csv
- attribution_writer_writes_side_csv
- attribution_writer_writes_filter_csv
- attribution_writer_writes_audit_json
- attribution_writer_writes_manifest_json
- attribution_csv_headers_are_stable
- attribution_csv_escape_handles_commas_and_quotes

### Manifest tests

- manifest_contains_required_files
- manifest_uses_relative_paths
- manifest_sorts_files_by_path
- manifest_has_stable_phase_name

### Research integration tests

Add only if existing testing structure allows it without brittle large fixtures.

Minimum acceptable: unit-test the Phase 7 report builder/writer directly.

Do not create huge CSV fixtures unless necessary.

## Existing behavior must remain

All existing Phase 1 through Phase 6 tests must continue passing.

Backtest must still write:

- backtest_summary.json
- trades.csv
- equity_curve.csv

Strategy still emits Signal only.

Risk still emits RiskAssessment only.

Backtest still creates simulated Trade only.

Paper and live must remain disabled.

## Strictly forbidden in Phase 7

Do not create:

- React app
- TypeScript app
- dashboard
- web UI
- Telegram integration
- LLM trading decision
- manager agent
- learning agent
- survival agent
- orchestrator
- live exchange order placement
- paper trading loop
- strategy optimizer
- portfolio optimizer
- 100x leverage logic
- synthetic candles
- interpolated candles
- exchange API integration
- websocket feed
- database requirement

Do not implement:

- live trading
- paper trading
- exchange adapters
- parameter optimization
- AI signal generation
- adaptive strategy tuning
- external broker integration
- notification systems
- AI advisor
- automatic strategy changes

AI may later summarize reports, but AI must not decide entries, directly change SL/TP, or directly size positions.

## Required commands

Run:

cargo fmt
cargo build
cargo test
cargo run -- research --config config/research.toml
cargo run -- help

If valid CSV data exists, research must generate the full Phase 7 report set.

If no CSV data exists, research must not panic and must print the friendly missing-data message.

Do not leave failing tests.

Do not leave TODO stubs in active Phase 7 behavior.

## Expected final result

At the end of Phase 7, the repository should have:

- Phase 1 core still intact
- Phase 2 market data still intact
- Phase 3 indicators still intact
- Phase 4 strategy still intact
- Phase 5 risk model still intact
- Phase 6 backtest engine still intact
- src/report/attribution.rs
- src/report/audit.rs
- src/report/manifest.rs
- src/report/validation.rs
- every trade auditable by signal_id
- attribution summary generated
- attribution bucket CSV files generated
- audit report generated
- report manifest generated
- README updated to Phase 7
- CLI help updated to Phase 7
- paper mode disabled
- live mode disabled
- no exchange API
- no LLM trading decisions
- cargo fmt passing
- cargo build passing
- cargo test passing
- cargo run -- research --config config/research.toml working
- cargo run -- help working

## Suggested implementation order

1. Read AGENTS.md and docs/ROADMAP.md.
2. Review src/backtest/report.rs.
3. Review src/core/trade.rs.
4. Replace src/report/mod.rs placeholder with Phase 7 module exports.
5. Implement src/report/attribution.rs.
6. Implement src/report/audit.rs.
7. Implement src/report/manifest.rs.
8. Implement src/report/validation.rs.
9. Add writer functions for Phase 7 attribution files.
10. Integrate Phase 7 writer into src/research/mod.rs.
11. Update README to Phase 7.
12. Update src/main.rs help text to Phase 7.
13. Add attribution tests.
14. Add audit tests.
15. Add manifest tests.
16. Add writer tests.
17. Run cargo fmt.
18. Run cargo build.
19. Run cargo test.
20. Run cargo run -- research --config config/research.toml.
21. Run cargo run -- help.

## Commit message suggestion

phase7: implement reports and attribution
