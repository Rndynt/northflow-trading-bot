# Strategy Sample Cleanup Report

## Summary

The active strategy layer has been cleaned up to contain exactly one production strategy: `BasicSampleStrategy` with the stable strategy ID `basic_sample_strategy`.

This cleanup is intentionally not a trading-edge or profitability change. The retained strategy is a deterministic sample/reference implementation for future strategy development.

## Strategy files removed

Removed stale production strategy modules:

- `src/strategy/screened_vwap_scalp.rs`
- `src/strategy/liquidity_sweep_reclaim.rs`
- `src/strategy/regime.rs`

Previously referenced strategy IDs that are no longer accepted:

- `screened_vwap_scalp`
- `screened_vwap_scalp_v2`
- `ema_trend_pullback_v1`
- `vwap_reclaim_short_v1`
- `vwap_reclaim_short_v2`
- `mean_revert_v1`
- `liquidity_sweep_reclaim_v1`

## New sample strategy

Added `src/strategy/basic_sample.rs` with `BasicSampleStrategy`.

The sample strategy:

- implements the existing `Strategy` trait,
- reads only `StrategyContext` and `MultiTimeframeInput`,
- emits only `Option<Signal>`,
- uses deterministic ATR-based stop-loss/take-profit geometry,
- uses deterministic `SIG-BT-XXXXXXXX` signal IDs from `ctx.signal_index`,
- uses configured timeframe roles from `StrategyContext`,
- does not place orders,
- does not size positions,
- does not mutate account state,
- does not call exchanges, APIs, system time, or randomness.

## Final accepted strategy ID

The only accepted production strategy ID is:

```text
basic_sample_strategy
```

## Registry cleanup summary

`src/strategy/registry.rs` now resolves only `basic_sample_strategy`.

Old strategy IDs are rejected loudly with a config error and are not retained as aliases or compatibility mappings.

## Config cleanup summary

`config/research.toml` now points to `basic_sample_strategy` for both `strategy_id` and `strategies`.

The Rust config model was simplified to remove old strategy-specific config structs, parsing, validation, and helpers for the removed strategies. The sample strategy currently has no strategy-specific configuration.

## Tests added or updated

Tests now cover:

- successful registry resolution for `basic_sample_strategy`,
- rejection of old strategy IDs by the registry,
- config acceptance of `basic_sample_strategy`,
- config rejection of old strategy IDs,
- sample strategy ID stability,
- long sample signal emission,
- short sample signal emission,
- no signal for invalid ATR or no setup,
- valid emitted signal geometry,
- deterministic signal IDs,
- configured timeframe roles carried into emitted signals.

Existing engine/report tests were updated to use `basic_sample_strategy` or generic unknown IDs where the test is about engine/report behavior rather than old strategy behavior.

## Commands run and results

- `cargo fmt --check` — passed.
- `cargo test` — passed; 419 tests passed.
- `cargo run -- research --config config/research.toml` — passed; config loaded, strategy registry validation passed, and research mode ran with `basic_sample_strategy`.
- Final repository search for removed strategy IDs and stale strategy config prefixes under active code/config/docs — passed with remaining old names limited to this cleanup report and the historical prompt.

## Remaining limitations

- `BasicSampleStrategy` is a reference/sample implementation only and should not be treated as a trading edge.
- The backtest engine remains historical-simulation only; paper and live trading remain disabled.
- Compiler warnings remain in `src/research/mod.rs` for pre-existing unused reporting fields/helper functions; they do not block tests or the research command.
