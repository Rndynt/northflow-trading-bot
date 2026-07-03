# Logging & Progress Output Cleanup Report

## 1. Baseline logging sources

Baseline inspection found direct CLI output in these places:

- `src/main.rs`: top-level error printing and help text.
- `src/research/mod.rs`: research orchestration banners, timeframe/config plan, strategy/risk/engine readiness messages, per-symbol data loading and data quality output, backtest start/error messages, final summary, signal flow, audit, attribution, diagnostics, and report-written messages.
- `src/backtest/engine.rs`: replay progress and final completion messages from inside the engine loop.
- `src/config/mod.rs`: inspected for config behavior; no runtime presentation output was changed.
- `src/report/*` and `src/backtest/report.rs`: report writers produce files; no direct runtime presentation output required cleanup there.

## 2. High-level orchestration logs

High-level research orchestration logs were previously scattered across the start of `run_research`, `run_single_strategy`, and comparison-mode setup in `src/research/mod.rs`. They included mixed timeframe descriptions, runtime guardrails, strategy readiness, risk readiness, and backtest engine readiness.

## 3. Engine progress logs

The backtest engine previously printed one progress line every 50,000 entry candles and printed a final `Backtest complete` line. This made multi-year 1m datasets noisy in narrow/mobile terminals.

## 4. Final result summaries

Final result output was produced in `run_symbol_verbose` in `src/research/mod.rs`. It included trade summary metrics, signal flow counters, rejection counts, audit status, attribution summary, report paths, and diagnostics, but the sections were inconsistently grouped and mixed full report paths with repeated marker lines.

## 5. Misleading or noisy logs

The noisy or misleading items found were:

- Repeated old fixed-interval progress line spam.
- Long comma-separated historical source file paths.
- Repeated report-written blocks with full directory prefixes for each file.
- Help text that only mentioned a single fallback path and did not mention `[historical_files]`.
- Runtime readiness text that duplicated information already present in the run plan.

## 6. Progress output design chosen

The implemented design uses a lightweight coarse progress-bar milestone reporter in `src/backtest/engine.rs`. It prints compact progress lines at 0%, 10%, 20%, ..., 100% with percentage, current/total candles, trade count, and elapsed time.

This avoids uncontrolled carriage-return behavior for non-TTY output while still avoiding thousands of progress lines. Progress formatting is centralized in a small `BacktestProgress` helper and no longer appears inline in the replay loop.

## 7. Research header cleanup summary

Research-mode output now starts with clear sections:

- `Northflow Research`
- `Run Plan`
- `Runtime Guardrails`
- `Engine`

Values come from `ResearchConfig` rather than hardcoded timeframe text.

## 8. Symbol/data output cleanup summary

Per-symbol output now prints historical data files one per line with numbering, followed by a `Data Summary` section. Large integer counts use thousands separators. Missing-data output is organized into missing data, expected files, and how-to-fix guidance.

## 9. Final summary cleanup summary

Final output is grouped into:

- `Backtest Summary`
- `Signal Flow`
- `Rejection Breakdown`
- `Audit`
- `Attribution`
- `Reports Written`
- `Diagnostics`

Existing metrics remain visible; this change is presentation-only and does not alter trading, strategy, risk, accounting, fill simulation, or report-writing calculations.

## 10. Reports written output cleanup summary

Report output is grouped by report type and prints the report directory once. Individual report filenames are listed without repeated directory prefixes.

## 11. Help text update summary

`northflow help` now explains both `[historical_files]` preset configuration and the `data_dir/<SYMBOL>.csv` fallback. It still states that source data must currently be 1m OHLCV and lists accepted timestamp columns.

## 12. Tests added/updated

Added tests for:

- `format_int(3156480) == "3,156,480"`.
- Progress formatting for 0.0%, 50.0%, and 100.0%.
- Output helpers accepting empty labels/values without panic.
- Report grouping constants containing filenames without repeated directory prefixes.

Existing config, engine, strategy, risk, indicator, market, and report tests remain in the normal `cargo test` suite.

## 13. Commands run and results

- `cargo fmt --check` — passed after formatting.
- `cargo test` — passed.
- `cargo run -- research --config config/research.toml` — passed with organized output and generated the configured reports.
- Final search verified that active source no longer contains the old fixed-interval progress spam text, hardcoded old help-only path, or repeated report block marker strings.

## 14. Remaining limitations and future logging improvements

- Progress currently uses coarse 10% milestone lines for robust non-TTY behavior rather than a carriage-return single-line progress bar.
- A future verbosity mode (`quiet`, `normal`, `verbose`) can make data-quality warning detail and comparison-mode output more configurable.
- Comparison mode still uses simpler per-run messages than single mode; it can be further organized without changing backtest behavior.
