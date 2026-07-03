# Engine cleanup implementation report

## Files changed

- `Cargo.toml`: renamed the package/library to the current Northflow repository identity and added TOML parsing dependencies.
- `src/main.rs`: updated the library crate import name.
- `src/config/mod.rs`: added TOML-backed preset parsing, runtime validation for single-position mode and accounting inputs, plus config tests for malformed TOML, timeframe ordering, and `max_open_positions`.
- `src/strategy/mod.rs`: removed active exports for strategy modules that are not present in `src/strategy/` so formatting and compilation can run.
- `src/backtest/engine.rs`: replaced the concrete strategy enum with a trait-object adapter, made risk context open-position count reflect engine state, and changed trade accounting to avoid double-counting embedded execution slippage.
- `docs/engine-cleanup-notes.md`: baseline notes and known gaps.
- `docs/engine-cleanup-report.md`: this implementation report.

## Config migration summary

The preset loader now parses the TOML document as TOML instead of depending only on line scanning. The legacy `ResearchConfig` shape remains available to avoid a disruptive rewrite of the research orchestrator, but preset values are mapped from sections such as `[pairs]`, `[historical_files]`, `[strategy]`, `[risk]`, `[cost]`, `[backtest]`, and `[reports]`.

## Engine API before/after summary

Before, `BacktestEngine` held a concrete `ActiveStrategy` enum with per-strategy variants. After, the engine evaluates through `dyn Strategy` behind a small adapter. This narrows the execution path toward strategy-agnostic replay while keeping existing tests and research mode operational.

## Strategy decoupling summary

The active strategy module list now matches files present in the repository. Missing strategy module exports were removed from `src/strategy/mod.rs` because the source files are absent. The engine no longer needs an enum that calls each concrete strategy variant directly.

## Cost/slippage accounting decision

Implemented Model A: execution prices include adverse entry/exit slippage. Gross PnL is calculated from actual execution prices. Net PnL subtracts fees, spread, market impact, and stop-extra cost, but does not subtract diagnostic embedded slippage a second time.

## Single-position limitation decision

The backtest remains explicitly single-position. Preset validation rejects `max_open_positions != 1` with a clear multi-position-not-implemented error.

## New tests added

- Valid TOML preset parses successfully.
- Malformed TOML returns an error through `try_parse`.
- Invalid timeframe ordering returns an error.
- `max_open_positions > 1` returns a clear single-position limitation error.

## Data quality visibility added

The run output continues to print raw, entry, confirmation, and screening candle counts plus data quality error/gap counters. Dedicated dropped-bucket JSON output remains a known follow-up.

## Commands run and results

- `cargo fmt --check` initially failed on missing strategy module declarations.
- `cargo fmt --check` passed after cleanup.
- `cargo test` passed: 501 tests.
- `cargo run -- research --config config/research.toml` passed and wrote the configured reports.

## Remaining known limitations

- The research orchestrator still passes `ResearchConfig` rather than fully split internal config structs.
- Strategy registry/factory is not yet fully moved outside `backtest`.
- Dedicated `data_quality_summary.json` was not added in this pass.
