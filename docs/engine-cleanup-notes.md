# Engine cleanup notes

## Baseline inventory

- CLI entrypoint: `src/main.rs` dispatches `northflow research --config ...`, loads `ResearchConfig`, and calls `research::run_research`.
- Config entrypoint: `src/config/mod.rs` owns preset loading and runtime validation.
- Strategy selection flow: research single/comparison mode selects strategy ids from config, while the engine still uses a small trait-object adapter for the currently compiled strategies.
- Backtest replay flow: `BacktestEngine::run` loads data, builds timeframe candles, precomputes no-lookahead indicator snapshots, evaluates signals, re-risks actual entry fills, simulates exits, updates equity, and returns a `BacktestResult` ledger.
- Cost/slippage behavior after cleanup: execution prices include adverse slippage, while `Trade.slippage` is diagnostic and is not subtracted from `net_pnl` a second time.
- Single-position limitation: the deterministic replay engine stores one `Option<OpenSimPosition>`; config validation rejects `max_open_positions != 1` for real preset loads.
- Baseline safety check: initial `cargo fmt --check` failed because `src/strategy/mod.rs` referenced strategy modules not present in the active source tree. Active module exports were aligned with files that exist under `src/strategy/`.

## Remaining gaps

- A fuller registry/factory split should replace the temporary in-engine trait-object adapter for all strategy ids once missing strategy modules are restored or deliberately removed from config validation.
- Timeframe dropped-bucket counts are still covered by builder tests, but a dedicated JSON data-quality report was not fully wired into report output in this pass.
