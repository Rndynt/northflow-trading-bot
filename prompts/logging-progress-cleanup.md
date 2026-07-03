# Northflow Trading Bot — Logging & Progress Output Cleanup Prompt

## Role

You are an implementation agent working on `Rndynt/northflow-trading-bot`, a Rust deterministic crypto trading research/backtest project.

Your task is to clean up and organize the CLI/runtime output for research runs.

This is a presentation, logging, and developer-experience cleanup task. It must not alter trading logic, strategy behavior, accounting, risk logic, indicator formulas, data loading semantics, or backtest results.

---

## Primary Objective

Make `cargo run -- research --config config/research.toml` output clean, readable, organized, and terminal-friendly.

Current runtime output is too noisy and difficult to read, especially on mobile/Termux. Backtest progress currently prints one line every 50,000 candles, causing massive vertical spam for multi-year 1m datasets.

Replace noisy line-by-line progress with a compact progress bar or single-line progress indicator, and reorganize all logs into clear sections.

---

## Non-Negotiable Constraints

Do not violate these rules.

1. Do **not** change strategy logic.
2. Do **not** change signal logic.
3. Do **not** change indicator formulas.
4. Do **not** change risk sizing or risk guards.
5. Do **not** change fill simulation logic.
6. Do **not** change accounting logic.
7. Do **not** change backtest results.
8. Do **not** enable paper mode.
9. Do **not** enable live mode.
10. Do **not** add exchange/network/API/LLM calls.
11. Do **not** introduce heavy logging dependencies unless clearly justified.
12. Do **not** make logs dependent on colors only; output must remain readable without ANSI color support.
13. Do **not** spam millions of progress lines.
14. Do **not** hide warnings/errors.
15. Do **not** remove important metrics from the final summary.
16. Do **not** change report file names or report contents unless only adding a harmless log-related metadata field.

---

## Current Problem

The current CLI output is functional but messy:

- Section headers are inconsistent.
- Runtime capability messages are mixed with actual run output.
- Historical file paths are printed as a long wrapped line, difficult to read on mobile terminals.
- Backtest progress prints repeated lines like:

```text
Backtest progress: 50000/3156480 entry candles (1.6%)
Backtest progress: 100000/3156480 entry candles (3.2%)
...
```

- For 3,156,480 entry candles this produces many lines.
- Final summary is readable but could be grouped better.
- Some numbers are not aligned consistently.
- Logs do not clearly separate these phases:
  - config load
  - runtime plan
  - data loading
  - data quality
  - backtest replay
  - result summary
  - signal flow
  - rejections
  - reports written
  - diagnostics

---

# Desired Output Style

Use clear, compact, organized sections.

Example target output:

```text
Northflow Research
==================
Mode        : research
Strategy    : basic_sample_strategy
Symbols     : BTCUSDT
Source TF   : 1m
Entry TF    : 1m
Confirm TF  : 5m
Screen TF   : 15m
Reports Dir : reports/basic_sample_btc_entry1m_2020_2025

Runtime Guardrails
------------------
Paper trading : disabled
Live trading  : disabled
Exchange calls: disabled

Symbol: BTCUSDT
---------------
Data files:
  1. data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv
  2. data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv
  3. data/historical/BTCUSDT/1m/BTCUSDT-1m-2022.csv
  4. data/historical/BTCUSDT/1m/BTCUSDT-1m-2023.csv
  5. data/historical/BTCUSDT/1m/BTCUSDT-1m-2024.csv
  6. data/historical/BTCUSDT/1m/BTCUSDT-1m-2025.csv

Data Summary
------------
Raw 1m candles        : 3,156,480
Entry 1m candles      : 3,156,480
Confirm 5m candles    :   631,296
Screen 15m candles    :   210,432
Data quality errors   : 0
Duplicate timestamps  : 0
Missing gaps          : 0

Backtest Replay
---------------
[##############################] 100.0% 3,156,480/3,156,480 candles | trades: 24,096 | elapsed: 00:42

Backtest Summary
----------------
Total trades           : 24,096
Win rate               : 34.41%
Net PnL                : -5,000.00
Gross PnL              : -969.06
Total fees             : 2,464.77
Total slippage         : 1,232.39
Profit factor          : 0.2679
Max drawdown           : 100.00%
Max consecutive losses : 20
Final equity           : 0.00

Signal Flow
-----------
Signals generated      : 861,338
Signals preapproved    : 24,107
Rejected initial risk  : 837,231
Rejected actual entry  : 11
Trades opened          : 24,096
Trades closed          : 24,096
Risk rejection rows    : 949,838

Rejection Breakdown
-------------------
Max drawdown           : 0
Daily loss             : 232,436
Reward/risk            : 0
Expected net edge      : 717,402
Other                  : 0

Reports Written
---------------
Base:
  - backtest_summary.json
  - trades.csv
  - equity_curve.csv
  - risk_rejections.csv
  - signal_flow_summary.json
Attribution:
  - attribution_summary.json
  - attribution_by_regime.csv
  - attribution_by_exit_reason.csv
  - attribution_by_side.csv
  - attribution_by_filter.csv
  - attribution_by_strategy.csv
  - audit_report.json
  - report_manifest.json
Diagnostics:
  - signal_diagnostics.csv
  - rejection_by_stage_reason.csv
  - monthly_summary.csv
  - cost_edge_distribution.csv
  - trade_distribution_summary.json

Diagnostics
-----------
Avg total cost bps     : 12.00
Avg edge realization   : -33.81 bps
Dominant rejection     : expected_net_edge_not_positive (717,402)
```

Exact wording may vary, but the structure must become organized and readable.

---

# Phase L0 — Baseline Inspection

## Goal

Identify all current direct CLI/log output.

## Required Files To Inspect

```text
src/main.rs
src/research/mod.rs
src/backtest/engine.rs
src/config/mod.rs
src/report/*
src/backtest/report.rs
```

Search for:

```text
println!
eprintln!
Backtest progress
Backtest complete
reports written
Signal flow
Diagnostics
Historical data
```

## Required Baseline Notes

Create or update:

```text
docs/logging-progress-cleanup-report.md
```

Add a baseline section listing:

1. Where CLI output is currently produced.
2. Which logs are high-level research orchestration logs.
3. Which logs are engine progress logs.
4. Which logs are final result summaries.
5. Which logs are misleading or too noisy.

## Acceptance Criteria

- Baseline logging sources are documented.
- No behavior changed in this phase.

---

# Phase L1 — Add Lightweight CLI Output Helpers

## Goal

Avoid scattered ad-hoc `println!` formatting.

## Required Design

Add a small internal output helper module. Preferred location:

```text
src/research/output.rs
```

or:

```text
src/cli/output.rs
```

If `src/cli` does not exist, `src/research/output.rs` is acceptable.

Do not over-engineer. Keep it simple.

## Suggested Helpers

```rust
pub fn section(title: &str)
pub fn subsection(title: &str)
pub fn key_value(label: &str, value: impl std::fmt::Display)
pub fn bullet(value: impl std::fmt::Display)
pub fn numbered(index: usize, value: impl std::fmt::Display)
pub fn blank()
pub fn format_int(value: usize) -> String
pub fn format_f64(value: f64, decimals: usize) -> String
```

Use plain ASCII or Unicode consistently. Since this is often run in Termux/mobile, prefer simple ASCII-safe separators unless Unicode already works well.

Recommended simple style:

```text
Section Title
-------------
Label : value
```

Avoid overly wide banners.

## Acceptance Criteria

- Output formatting helpers exist.
- Research output uses helpers where practical.
- No behavior or calculations changed.
- `cargo fmt --check` passes.
- `cargo test` passes.

---

# Phase L2 — Replace Backtest Progress Spam With Progress Bar

## Goal

Replace repeated progress lines with a compact progress indicator.

## Current Behavior To Replace

In `src/backtest/engine.rs`, progress currently prints every fixed number of candles.

Replace line spam with one of these approaches:

### Preferred approach — Single-line progress bar

Print a carriage-return updated progress line:

```text
[##########--------------------]  33.4% 1,054,321/3,156,480 candles | trades: 8,123 | elapsed: 00:14
```

Use `\r` and flush stdout.

At completion, print a newline once.

### Fallback approach — Coarse milestone progress

If single-line progress is unreliable or too complex, print only milestone lines at:

```text
0%, 10%, 20%, 30%, 40%, 50%, 60%, 70%, 80%, 90%, 100%
```

Do not print every 50,000 candles.

## Required Runtime Config

Keep it simple. Add an internal progress config if needed:

```rust
pub struct ProgressConfig {
    pub enabled: bool,
    pub width: usize,
    pub min_update_interval_ms: u64,
}
```

But do not require config file support unless easy.

Default:

```text
enabled = true
width = 30
min_update_interval_ms = 250
```

## Important Requirements

1. Progress should show percentage.
2. Progress should show current/total candles.
3. Progress should show number of trades opened or closed if available.
4. Progress should show elapsed time if practical.
5. Progress should not print thousands of lines.
6. Progress should not break final summary formatting.
7. Progress should still work when stdout is not a TTY, but avoid uncontrolled carriage-return logs.

For non-TTY, either:

- print coarse milestones only, or
- keep single-line updates disabled.

Do not add a dependency just to detect TTY unless Rust std or an existing crate makes it simple. It is acceptable to always use coarse milestones if that is more robust.

## Suggested Implementation Without External Dependency

Create helper:

```rust
struct BacktestProgress {
    total: usize,
    width: usize,
    last_percent_bucket: usize,
    started_at: std::time::Instant,
}
```

Methods:

```rust
impl BacktestProgress {
    fn new(total: usize) -> Self
    fn tick(&mut self, current: usize, trades: usize)
    fn finish(&mut self, trades: usize)
}
```

For line-based milestone mode:

```rust
let bucket = percent.floor() as usize / 10;
if bucket > self.last_percent_bucket { print milestone }
```

For progress bar mode:

```rust
let filled = (percent / 100.0 * width as f64).round() as usize;
let bar = format!("[{}{}]", "#".repeat(filled), "-".repeat(width - filled));
print!("\r{} {:>5.1}% {}/{} candles | trades: {} | elapsed: {}", ...);
std::io::stdout().flush().ok();
```

## Acceptance Criteria

- No `Backtest progress: 50000/...` spam remains.
- Progress output is compact.
- Final newline is printed before final summary.
- `cargo test` passes.

---

# Phase L3 — Keep Engine Pure Enough While Handling Progress

## Goal

Do not make logging cleanup ruin the engine boundary.

The engine currently accepts prepared input and runs replay. Adding progress printing directly inside the engine is acceptable only if kept minimal, but better is a progress callback or reporter interface.

## Preferred Design

Add optional progress callback to `BacktestRunInput`:

```rust
pub struct BacktestProgressSnapshot {
    pub current: usize,
    pub total: usize,
    pub trades_opened: usize,
    pub trades_closed: usize,
}

pub type BacktestProgressCallback<'a> = Option<&'a mut dyn FnMut(BacktestProgressSnapshot)>;
```

Then the research orchestrator owns rendering.

If that becomes too invasive, keep progress rendering in engine for now but document it as a temporary CLI concern. Do not let this block the cleanup.

## Simpler Acceptable Design

Add a `progress_enabled: bool` or `progress: ProgressMode` field to `BacktestRunInput`, with default enabled from research. Keep all progress output in one helper, not scattered `println!`.

## Acceptance Criteria

- Progress implementation is centralized.
- Backtest loop does not contain ugly formatting logic inline.
- No scattered progress `println!` calls remain.
- Engine tests still pass.

---

# Phase L4 — Organize Research Header Output

## Goal

Make the initial research output compact and useful.

## Required Header Sections

Replace the current long banner and scattered messages with organized sections.

### Header

```text
Northflow Research
==================
```

### Run Plan

```text
Run Plan
--------
Mode        : research
Strategy    : basic_sample_strategy
Symbols     : BTCUSDT
Source TF   : 1m
Entry TF    : 1m
Confirm TF  : 5m
Screen TF   : 15m
Reports Dir : reports/basic_sample_btc_entry1m_2020_2025
```

### Runtime Guardrails

```text
Runtime Guardrails
------------------
Paper trading : disabled
Live trading  : disabled
Exchange calls: disabled
```

### Engine Capabilities

```text
Engine
------
Strategy output : Signal only
Risk output     : RiskAssessment only
Backtest model  : conservative intrabar fill
Lookahead       : disabled across configured higher timeframes
Signal IDs      : deterministic SIG-BT-XXXXXXXX
```

Do not overprint repeated information.

## Acceptance Criteria

- Header is compact.
- Values come from config.
- No hardcoded 1m/5m/15m text unless those are config values.
- No duplicate repeated readiness blocks.

---

# Phase L5 — Organize Symbol/Data Output

## Goal

Make per-symbol data loading output readable.

## Required Output

For each symbol:

```text
Symbol: BTCUSDT
---------------
Data files:
  1. path/to/file-2020.csv
  2. path/to/file-2021.csv
  3. path/to/file-2022.csv
```

Do not print all file paths in one long comma-separated wrapped line.

Then:

```text
Data Summary
------------
Raw 1m candles        : 3,156,480
Entry 1m candles      : 3,156,480
Confirm 5m candles    :   631,296
Screen 15m candles    :   210,432
Data quality errors   : 0
Duplicate timestamps  : 0
Missing gaps          : 0
```

Use thousands separators for large integers.

## Missing Data Output

If missing:

```text
Symbol: BTCUSDT
---------------
Missing historical data.
Expected files:
  1. data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv
  2. data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv

How to fix:
  - configure [historical_files], or
  - place fallback CSV at data_dir/<SYMBOL>.csv
```

## Acceptance Criteria

- File paths are printed one per line.
- Large numbers use thousands separators.
- Missing data message is clear and organized.
- No long unreadable comma-separated source line remains.

---

# Phase L6 — Organize Final Summary Output

## Goal

Make final results readable and grouped.

## Required Sections

### Backtest Summary

```text
Backtest Summary
----------------
Total trades           : 24,096
Win rate               : 34.41%
Net PnL                : -5,000.00
Gross PnL              : -969.06
Total fees             : 2,464.77
Total slippage         : 1,232.39
Profit factor          : 0.2679
Max drawdown           : 100.00%
Max consecutive losses : 20
```

### Signal Flow

```text
Signal Flow
-----------
Signals generated      : 861,338
Signals preapproved    : 24,107
Rejected initial risk  : 837,231
Rejected actual entry  : 11
Trades opened          : 24,096
Trades closed          : 24,096
Risk rejection rows    : 949,838
```

### Rejection Breakdown

```text
Rejection Breakdown
-------------------
Max drawdown           : 0
Daily loss             : 232,436
Reward/risk            : 0
Expected net edge      : 717,402
Other                  : 0
```

### Audit

```text
Audit
-----
Passed   : true
Errors   : 0
Warnings : 0
```

### Attribution

```text
Attribution
-----------
Unique signals         : 24,096
Avg expected edge      : 16.44 bps
Avg actual edge        : -17.37 bps
Edge realization       : -33.81 bps
```

### Diagnostics

```text
Diagnostics
-----------
Avg total cost         : 12.00 bps
Avg edge realization   : -33.81 bps
Dominant rejection     : expected_net_edge_not_positive (717,402)
```

## Acceptance Criteria

- Final summary is grouped into clear sections.
- Numeric formatting is consistent.
- All current important metrics remain visible.
- No calculations change.

---

# Phase L7 — Organize Reports Written Output

## Goal

Make report output readable without printing too much clutter.

## Required Output

Instead of long repeated blocks, use grouped report lists.

```text
Reports Written
---------------
Directory: reports/basic_sample_btc_entry1m_2020_2025

Base:
  - backtest_summary.json
  - trades.csv
  - equity_curve.csv
  - risk_rejections.csv
  - signal_flow_summary.json

Attribution:
  - attribution_summary.json
  - attribution_by_regime.csv
  - attribution_by_exit_reason.csv
  - attribution_by_side.csv
  - attribution_by_filter.csv
  - attribution_by_strategy.csv
  - audit_report.json
  - report_manifest.json

Diagnostics:
  - signal_diagnostics.csv
  - rejection_by_stage_reason.csv
  - monthly_summary.csv
  - cost_edge_distribution.csv
  - trade_distribution_summary.json
```

Do not repeat the full directory prefix for every file unless user enables verbose mode later.

## Acceptance Criteria

- Reports are grouped.
- Directory printed once.
- Filenames listed clearly.
- Existing file paths/reports remain unchanged.

---

# Phase L8 — Update Main Help Text

## Goal

Make `northflow help` match current config behavior.

## Current Issue

Help text still says:

```text
Place 1m CSV data in data/historical/<SYMBOL>.csv
```

This is incomplete because the preset now supports `[historical_files]` and `data_dir` fallback.

## Required Help Text

Update to something like:

```text
Historical data:
  Configure [historical_files] in the preset, or place fallback CSV at data_dir/<SYMBOL>.csv.
  Source data currently must be 1m OHLCV.
  Columns: timestamp,open,high,low,close,volume
  Alternative timestamp column: open_time
```

## Acceptance Criteria

- Help text matches actual config behavior.
- No misleading hardcoded fallback path remains as the only option.

---

# Phase L9 — Optional Verbosity Flag Preparation

## Goal

Keep scope small, but avoid painting the project into a corner.

Do not implement a full logging framework. But it is acceptable to prepare simple verbosity support if easy.

Possible future modes:

```text
quiet
normal
verbose
```

For now, default should be `normal`.

If implementing now is too much, document it as future work in the report.

---

# Tests Required

Add/update tests where practical.

At minimum:

1. `format_int(3156480) == "3,156,480"`.
2. progress formatter renders `0.0%`, `50.0%`, and `100.0%` correctly.
3. report file grouping renders filenames without repeated full directory prefixes if formatter is testable.
4. output helper does not panic on empty labels/values.
5. existing config tests still pass.
6. existing engine tests still pass.
7. existing strategy tests still pass.

Do not snapshot-test huge full CLI outputs unless the project already uses snapshot testing.

---

# Final Search Requirements

Search for:

```text
Backtest progress:
no lookahead across 5m / 15m
Place 1m CSV data in data/historical/<SYMBOL>.csv
Base backtest reports written.
Phase 7 attribution reports written.
Diagnostic reports written.
```

Expected:

- `Backtest progress:` old spam text should be gone.
- hardcoded `no lookahead across 5m / 15m` should be gone.
- old help text should be gone.
- repeated report block text should be replaced with grouped report output.

---

# Final Validation Commands

Run:

```bash
cargo fmt --check
cargo test
cargo run -- research --config config/research.toml
```

If historical data exists, the run should produce a clean organized output and report files.

If historical data is missing, the missing-data output should still be organized and helpful.

---

# Required Final Report

Create or update:

```text
docs/logging-progress-cleanup-report.md
```

Include:

1. Logging sources found during baseline.
2. Progress output design chosen.
3. Whether progress uses single-line bar or coarse milestones.
4. Research header cleanup summary.
5. Symbol/data output cleanup summary.
6. Final summary cleanup summary.
7. Reports written output cleanup summary.
8. Help text update summary.
9. Tests added/updated.
10. Commands run and results.
11. Remaining limitations or future logging improvements.

---

# Success Definition

This task is successful when:

1. Research CLI output is organized into clear sections.
2. Historical data files are printed one per line.
3. Large integer counts are formatted with thousands separators.
4. Backtest progress no longer spams one line every 50,000 candles.
5. Progress is shown as a compact progress bar or coarse milestone output.
6. Final summary is grouped and aligned.
7. Reports written output is grouped by report type.
8. Help text matches config behavior.
9. No trading/backtest/accounting/strategy result logic changes.
10. `cargo fmt --check` passes.
11. `cargo test` passes.
12. `docs/logging-progress-cleanup-report.md` documents the change.

---

# Out Of Scope

Do not implement these in this task:

- New strategies.
- Strategy tuning.
- Profitability optimization.
- Accounting changes.
- Risk model changes.
- Indicator changes.
- Paper trading.
- Live trading.
- Exchange integration.
- External logging/observability stack.
- UI/dashboard.
- Database persistence.
