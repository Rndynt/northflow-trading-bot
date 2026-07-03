# Northflow Trading Bot — Sample-Only Strategy Cleanup Prompt

## Role

You are an implementation agent working on `Rndynt/northflow-trading-bot`, a Rust deterministic trading research/backtest project.

Your task is to clean up the strategy layer so the codebase contains **exactly one production strategy**: a basic sample strategy used as an implementation example.

This is not a trading edge task. This is not a strategy research task. This is not a profitability task.

The goal is to make the codebase clean, understandable, and ready for future strategy development by keeping only one simple reference strategy.

---

## Primary Objective

Remove old experimental strategy implementations, old strategy aliases, and stale strategy config fields. Replace them with one clear sample/basic strategy that demonstrates how a strategy should implement the `Strategy` trait and emit a `Signal`.

After this cleanup:

1. There must be only one production strategy module.
2. There must be only one production strategy ID.
3. Strategy registry must only resolve that one strategy ID.
4. Config preset must reference only that strategy ID.
5. Old strategy names must not remain as accepted aliases.
6. Backtest engine must remain strategy-agnostic.
7. Tests must prove old strategy IDs are rejected.
8. Tests must prove the sample strategy works as a basic example.

---

## Non-Negotiable Constraints

Do not violate these rules.

1. Do **not** add multiple strategies.
2. Do **not** keep old strategy IDs as compatibility aliases.
3. Do **not** map old strategy IDs to the new sample strategy.
4. Do **not** tune for profitability.
5. Do **not** claim the sample strategy has an edge.
6. Do **not** delete indicators.
7. Do **not** alter the indicator engine unless required to compile.
8. Do **not** alter backtest accounting unless tests reveal a direct compile/runtime issue from this cleanup.
9. Do **not** re-couple `BacktestEngine` to concrete strategies.
10. Do **not** enable paper trading.
11. Do **not** enable live trading.
12. Do **not** add exchange/network/API calls.
13. Do **not** hide failing tests.
14. Do **not** keep dead files just because they are unused.

---

## Current Problem To Fix

The strategy layer still contains old strategy references and aliases.

Examples of current leftovers to inspect:

```text
src/strategy/mod.rs
src/strategy/registry.rs
src/strategy/liquidity_sweep_reclaim.rs
src/strategy/screened_vwap_scalp.rs
config/research.toml
src/config/*
src/research/*
reports or docs mentioning old strategy IDs
tests referencing old strategy IDs
```

Current known bad pattern:

```rust
"screened_vwap_scalp" | "screened_vwap_scalp_v2" => Box::new(ScreenedVwapScalp::default()),
"liquidity_sweep_reclaim_v1" => Box::new(LiquiditySweepReclaimV1::default()),
"ema_trend_pullback_v1"
| "vwap_reclaim_short_v1"
| "vwap_reclaim_short_v2"
| "mean_revert_v1" => Box::new(ScreenedVwapScalp::default()),
```

This is not acceptable. Old strategy IDs must not silently run a different strategy.

---

# Desired Final Strategy Design

## Production Strategy Name

Use exactly one production strategy:

```text
BasicSampleStrategy
```

## Production Strategy ID

Use exactly one stable ID:

```text
basic_sample_strategy
```

## File Layout

Preferred layout:

```text
src/strategy/
  mod.rs
  basic_sample.rs
  registry.rs
  traits.rs
  regime.rs       # keep only if still used by the sample or tests
```

Remove production modules for old strategies:

```text
src/strategy/screened_vwap_scalp.rs
src/strategy/screened_vwap_scalp_v2.rs
src/strategy/ema_trend_pullback.rs
src/strategy/vwap_reclaim_short.rs
src/strategy/vwap_reclaim_short_v2.rs
src/strategy/mean_revert.rs
src/strategy/liquidity_sweep_reclaim.rs
```

Only keep files that are still legitimately used after cleanup. If a file no longer has production usage, delete it.

---

# Strategy Behavior Requirements

The sample strategy should be intentionally simple and educational.

It must:

1. Implement the existing `Strategy` trait.
2. Be deterministic.
3. Use only the provided `StrategyContext` and `MultiTimeframeInput`.
4. Emit `Option<Signal>` only.
5. Never place orders.
6. Never calculate position size.
7. Never mutate account state.
8. Never call external APIs.
9. Never use randomness.
10. Never use system time.

## Suggested Simple Logic

Keep the logic basic and readable.

Recommended behavior:

### Long setup

Emit a long signal when all are true:

```text
entry close > entry VWAP
entry EMA 8 > entry EMA 21
confirmation close > confirmation VWAP
screening close > screening EMA 50
ATR is finite and > 0
```

### Short setup

Emit a short signal when all are true:

```text
entry close < entry VWAP
entry EMA 8 < entry EMA 21
confirmation close < confirmation VWAP
screening close < screening EMA 50
ATR is finite and > 0
```

If indicator field names differ, adapt to the existing `IndicatorSnapshot` fields. Do not create new indicators.

## Geometry

Use ATR-based geometry:

```text
Long:
  entry = entry_candle.close
  stop_loss = entry - ATR
  take_profit = entry + ATR * 1.5

Short:
  entry = entry_candle.close
  stop_loss = entry + ATR
  take_profit = entry - ATR * 1.5
```

If ATR is zero, negative, NaN, or infinite, return `Ok(None)`.

## Confidence

Use a fixed confidence that passes normal default config:

```text
confidence = max(ctx.min_confidence, 70)
```

Clamp to `100` if needed.

## Signal ID

Signal ID must remain deterministic:

```text
SIG-BT-00000001
SIG-BT-00000002
...
```

Use `ctx.signal_index` exactly as the existing strategy pattern expects.

## Signal Fields

Populate `Signal` fields clearly:

```text
strategy_id = "basic_sample_strategy"
regime = "sample_bullish" / "sample_bearish"
entry_reason = short plain-English description
filters_passed = list of passed sample filters
filters_failed = empty if signal is emitted
expected_reward_bps = computed from geometry if practical
estimated_cost_bps = ctx.estimated_cost_bps
expected_net_edge_bps = expected_reward_bps - ctx.estimated_cost_bps
```

For `Ok(None)`, no signal is emitted and no fake signal should be created.

---

# Phase S0 — Baseline Inspection

## Goal

Understand the current strategy references before modifying anything.

## Required Steps

1. Search for old strategy IDs:

```text
screened_vwap_scalp
screened_vwap_scalp_v2
ema_trend_pullback_v1
vwap_reclaim_short_v1
vwap_reclaim_short_v2
mean_revert_v1
liquidity_sweep_reclaim_v1
```

2. Inspect:

```text
src/strategy/mod.rs
src/strategy/registry.rs
src/config/*
src/research/*
config/research.toml
tests
README.md
docs
prompts
```

3. Record current leftovers in:

```text
docs/strategy-sample-cleanup-report.md
```

## Acceptance Criteria

- Existing old strategy references are identified.
- No code behavior changed yet.

---

# Phase S1 — Add BasicSampleStrategy

## Goal

Create the one production sample strategy.

## Required Steps

1. Create:

```text
src/strategy/basic_sample.rs
```

2. Implement:

```rust
pub struct BasicSampleStrategy;
```

3. Add:

```rust
impl Default for BasicSampleStrategy
```

4. Implement the existing `Strategy` trait.

5. Ensure:

```rust
fn strategy_id(&self) -> &'static str {
    "basic_sample_strategy"
}
```

6. Implement deterministic long/short sample logic as described above.

7. Add unit tests for:

- `strategy_id()` returns `basic_sample_strategy`.
- emits long signal on clear long sample input.
- emits short signal on clear short sample input.
- returns `Ok(None)` when ATR is invalid or no setup exists.
- emitted signal has valid geometry.
- emitted signal has deterministic ID.
- emitted signal uses configured timeframe roles from context.

## Acceptance Criteria

- `BasicSampleStrategy` compiles.
- Strategy emits only `Signal`.
- No risk sizing or order placement is inside the strategy.
- Tests pass.

---

# Phase S2 — Clean Strategy Module Exports

## Goal

Make `src/strategy` expose only the sample strategy plus shared traits/types.

## Required Changes

Update `src/strategy/mod.rs` to something like:

```rust
//! Strategy layer.
//!
//! Production strategy set is intentionally minimal.
//! The only active production strategy is `basic_sample_strategy`, used as a
//! reference implementation for future strategy development.
//!
//! Strategies emit `Signal` only. They must not place orders, size positions,
//! call exchanges, mutate account state, or write reports.

pub mod basic_sample;
pub mod registry;
pub mod traits;

pub use basic_sample::BasicSampleStrategy;
pub use traits::{MultiTimeframeInput, Strategy, StrategyContext};
```

Keep `regime.rs` only if it is still used. If not used, remove it.

Remove module declarations and exports for old strategies.

## Acceptance Criteria

- `src/strategy/mod.rs` no longer mentions old production strategies.
- Only `BasicSampleStrategy`, `registry`, and `traits` are exported, plus `regime` only if genuinely required.
- `cargo test` passes.

---

# Phase S3 — Clean Strategy Registry

## Goal

Registry must resolve only the one sample strategy.

## Required Changes

Update `src/strategy/registry.rs` so it only accepts:

```text
basic_sample_strategy
```

Example:

```rust
use crate::core::NorthflowError;
use crate::strategy::{BasicSampleStrategy, Strategy};

pub struct StrategyRuntime {
    pub strategy_id: String,
    pub strategy: Box<dyn Strategy>,
}

pub fn build_strategy_runtime(strategy_id: &str) -> Result<StrategyRuntime, NorthflowError> {
    let strategy: Box<dyn Strategy> = match strategy_id {
        "basic_sample_strategy" => Box::new(BasicSampleStrategy::default()),
        other => {
            return Err(NorthflowError::ConfigError(format!(
                "unknown strategy_id: '{other}'. Available strategy: 'basic_sample_strategy'"
            )));
        }
    };

    Ok(StrategyRuntime {
        strategy_id: strategy_id.to_string(),
        strategy,
    })
}
```

## Tests Required

Add tests for:

- `basic_sample_strategy` resolves successfully.
- `screened_vwap_scalp` is rejected.
- `screened_vwap_scalp_v2` is rejected.
- `ema_trend_pullback_v1` is rejected.
- `vwap_reclaim_short_v1` is rejected.
- `vwap_reclaim_short_v2` is rejected.
- `mean_revert_v1` is rejected.
- `liquidity_sweep_reclaim_v1` is rejected.

## Acceptance Criteria

- Registry has no aliases.
- Old strategy IDs fail loudly.
- Backtest engine remains decoupled and receives only `dyn Strategy` or equivalent.
- `cargo test` passes.

---

# Phase S4 — Delete Old Strategy Implementations

## Goal

Remove stale production strategy files.

## Required Steps

1. Delete old strategy files that are no longer referenced:

```text
src/strategy/screened_vwap_scalp.rs
src/strategy/screened_vwap_scalp_v2.rs
src/strategy/ema_trend_pullback.rs
src/strategy/vwap_reclaim_short.rs
src/strategy/vwap_reclaim_short_v2.rs
src/strategy/mean_revert.rs
src/strategy/liquidity_sweep_reclaim.rs
```

2. Remove tests that only validate old strategy behavior.

3. If old tests are actually engine tests disguised as strategy tests, rewrite them to use:

```text
BasicSampleStrategy
```

or a test-only stub strategy.

4. Search the entire repo again and remove stale imports/exports.

## Acceptance Criteria

- Old strategy source files are gone or explicitly documented if one must remain temporarily.
- No production code imports old strategies.
- No registry mapping references old IDs.
- `cargo test` passes.

---

# Phase S5 — Clean Config Preset

## Goal

Make config reference only the sample strategy.

## Required Changes

Update `config/research.toml` to use:

```toml
[strategy]
strategy_id = "basic_sample_strategy"
strategy_run_mode = "single"
strategies = ["basic_sample_strategy"]
```

Remove old strategy-specific fields such as:

```text
v2_require_strict_confirmation
v2_require_ema_ribbon_alignment
v2_allow_neutral_confirmation
v2_min_expected_reward_bps
v2_min_expected_net_edge_bps
v2_min_atr_bps
v2_max_atr_bps
v2_tp_atr_multiple
v2_sl_atr_multiple
v2_min_volume_ratio
v2_vwap_distance_atr_min
v2_vwap_distance_atr_max
v2_cooldown_bars
v2_enable_long
v2_enable_short
```

The sample strategy should not require strategy-specific config yet.

Keep risk, cost, timeframe, historical files, and backtest config.

## Acceptance Criteria

- `config/research.toml` has no old strategy-specific fields.
- Config validation accepts `basic_sample_strategy`.
- Config validation rejects old strategy IDs.
- `cargo test` passes.

---

# Phase S6 — Clean Config Types And Validation

## Goal

Remove old strategy-specific config from Rust config structs.

## Required Steps

1. Inspect all config structs and parsing/deserialization code.

2. Remove fields for old strategies, including but not limited to:

```text
v2_*
etp_*
vrs_*
vrs2_*
mean_revert*
liquidity_sweep*
```

3. Remove helper methods that only build old strategy configs, such as:

```text
v2_config()
etp_config()
vrs_config()
vrs2_config()
cooldown_bars_for_strategy()
```

unless still needed generically. Do not keep dead compatibility helpers.

4. If cooldown still exists, make it generic in strategy runtime or backtest config. For the sample strategy, default can be `0`.

5. Make validation list only:

```text
basic_sample_strategy
```

## Acceptance Criteria

- Config model has no old strategy-specific fields.
- Unknown strategy IDs are rejected.
- `basic_sample_strategy` is the only accepted production strategy.
- `cargo test` passes.

---

# Phase S7 — Clean Research Output Text And Docs

## Goal

Remove stale output messages and docs that imply old strategies are active.

## Required Steps

1. Inspect CLI output in `src/research/*`.

2. Remove special-case printing for old strategies.

3. Replace with generic strategy runtime printing:

```text
Strategy:
  strategy_id = basic_sample_strategy
  type = sample/reference implementation
```

4. Update README/docs if they mention old active strategies.

5. Keep old prompts if useful as historical instructions, but do not let README/current docs claim old strategies are active.

## Acceptance Criteria

- Current docs do not list old strategies as active.
- CLI output does not print old strategy-specific config.
- `cargo test` passes.

---

# Phase S8 — Final Repository Search And Enforcement

## Goal

Ensure cleanup is complete.

## Required Search Terms

Search the repository for:

```text
screened_vwap_scalp
screened_vwap_scalp_v2
ema_trend_pullback
ema_trend_pullback_v1
vwap_reclaim_short
vwap_reclaim_short_v1
vwap_reclaim_short_v2
mean_revert
mean_revert_v1
liquidity_sweep_reclaim
liquidity_sweep_reclaim_v1
v2_
etp_
vrs_
vrs2_
```

## Allowed Remaining References

Old strategy names may remain only in:

```text
prompts/strategy-sample-only-cleanup.md
docs/strategy-sample-cleanup-report.md
```

They must not remain in active production code, active config, active tests, README active strategy list, or registry aliases.

## Acceptance Criteria

- Search confirms no old strategy names in production code/config/tests.
- Only allowed documentation/report references remain.
- `cargo fmt --check` passes.
- `cargo test` passes.

---

# Final Validation Commands

Run:

```bash
cargo fmt --check
cargo test
cargo run -- research --config config/research.toml
```

If historical data files are unavailable, `cargo run` may stop with a clear missing-data message. That is acceptable only if config loading and strategy registry validation pass first.

---

# Required Final Report

Create or update:

```text
docs/strategy-sample-cleanup-report.md
```

Include:

1. Summary of strategy files removed.
2. Summary of the new sample strategy.
3. Final accepted strategy ID.
4. Registry cleanup summary.
5. Config cleanup summary.
6. Tests added/updated.
7. Commands run and results.
8. Remaining limitations.

---

# Success Definition

This task is successful when the project contains one and only one production strategy named `BasicSampleStrategy` with ID `basic_sample_strategy`, all old strategies and aliases are removed from active code/config/tests, the backtest engine remains strategy-agnostic, the preset still runs through the registry, and the full test suite passes.
