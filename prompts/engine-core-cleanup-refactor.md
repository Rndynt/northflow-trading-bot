# Northflow Trading Bot — Engine Core Cleanup & Refactor Prompt

## Role

You are an implementation agent working on `Rndynt/northflow-trading-bot`, a Rust-based deterministic crypto trading research/backtest project.

Your task is to refactor and validate the **engine core**. This is not a strategy research task. This is not a profitability task. This is a cleanup, correctness, determinism, and architecture task.

The final result must make the engine reliable enough to be used later for strategy experiments.

---

## Primary Objective

Refactor the project from a strategy-coupled research prototype into a cleaner deterministic research engine where:

1. Preset-based config still works.
2. Internal config is split by concern.
3. `BacktestEngine` no longer knows concrete strategies.
4. Cost/slippage accounting is unambiguous and tested.
5. The current single-position model is made explicit.
6. Data quality and dropped higher-timeframe buckets are visible.
7. Engine lifecycle tests prove the backtest engine works without depending on real strategies.

---

## Non-Negotiable Constraints

Do not violate these rules.

1. Do **not** add a new trading strategy.
2. Do **not** tune existing strategies.
3. Do **not** optimize for profitability.
4. Do **not** delete existing indicators.
5. Do **not** delete existing strategies unless absolutely required to make the code compile, and if so, document why.
6. Do **not** enable `paper` mode.
7. Do **not** enable `live` mode.
8. Do **not** add exchange calls, network calls, API calls, LLM decision calls, or real order placement.
9. Do **not** hide failing tests.
10. Do **not** silently ignore accounting ambiguity.
11. Do **not** implement preset fragments/composition yet unless explicitly necessary. Keep one complete preset file per run.
12. Do **not** convert this into a general trading framework. Keep the scope tight.

---

## Existing Areas To Inspect First

Before editing, inspect these files and understand the current flow:

```text
Cargo.toml
src/main.rs
src/lib.rs
src/config/mod.rs
src/research/mod.rs
src/backtest/mod.rs
src/backtest/engine.rs
src/backtest/fill_model.rs
src/backtest/geometry.rs
src/backtest/metrics.rs
src/backtest/risk_trace.rs
src/risk/mod.rs
src/risk/guard.rs
src/risk/cost_model.rs
src/risk/position_sizing.rs
src/market/mod.rs
src/market/ohlcv_loader.rs
src/market/candle_store.rs
src/market/timeframe_builder.rs
src/strategy/mod.rs
src/strategy/traits.rs
src/report/*
config/research.toml
```

Current known architectural problems:

- `BacktestEngine` imports concrete strategy implementations directly.
- `BacktestEngine` builds an internal concrete strategy enum.
- `BacktestEngine::run` takes the global `ResearchConfig`, which mixes unrelated concerns.
- `ResearchConfig` contains pairs, historical data paths, report paths, strategy runner config, risk config, cost config, backtest config, and many strategy-specific fields.
- Current config parsing is manual line-by-line parsing and does not respect TOML sections properly.
- The user-facing config file is effectively a preset and must remain usable as a preset.
- Fill prices already include adverse slippage, but trade accounting also subtracts slippage as a nominal cost. This likely double-counts slippage.
- The engine is functionally single-position because it stores one `Option<OpenSimPosition>`, but config exposes `max_open_positions`.
- Higher-timeframe aggregation drops incomplete or overfilled buckets silently.
- Current integration tests do not strongly prove the full lifecycle: signal → pending entry → fill → open position → exit → trade → equity update.

---

## Desired Architecture

The external workflow should remain simple:

```bash
cargo run -- research --config config/research.toml
```

Internally, the architecture should become:

```text
CLI
  ↓
Research preset loader
  ↓
Typed config sections
  ↓
Research orchestrator
  ↓
Strategy registry/factory
  ↓
BacktestEngine
  ↓
BacktestResult ledger
  ↓
Report writers / diagnostics / attribution
```

`BacktestEngine` must become a clean engine component. It should not parse TOML, choose strategy IDs, know concrete strategies, write reports, or know preset file paths.

---

# Phase P0 — Baseline Inventory And Safety Check

## Goal

Establish the current baseline before refactoring.

## Required Steps

1. Run:

```bash
cargo fmt --check
cargo test
```

2. Inspect the current engine flow:

```text
main.rs → ResearchConfig::load → run_research → run_symbol_strategy → BacktestEngine::run
```

3. Create or update:

```text
docs/engine-cleanup-notes.md
```

Include:

- Current CLI entrypoint.
- Current config entrypoint.
- Current strategy selection flow.
- Current backtest replay flow.
- Current cost/slippage accounting behavior.
- Current test coverage gaps.
- Any failing baseline tests.

## Acceptance Criteria

- Baseline result is documented.
- No functional trading logic changed in this phase.
- No indicators removed.
- No strategies added.

---

# Phase P1 — Project Identity Cleanup

## Goal

Align package metadata with the current repository name to avoid stale identity from the previous repo.

## Required Steps

1. Inspect `Cargo.toml`.
2. Update stale repository metadata.
3. Preferred package identity:

```toml
[package]
name = "northflow-trading-bot"
```

4. Preferred library identity:

```toml
[lib]
name = "northflow_trading_bot"
```

5. Keep binary name:

```toml
[[bin]]
name = "northflow"
```

6. If the library crate name changes, update imports in `src/main.rs` and tests.

Example:

```rust
use northflow_crypto_trading_bot::{config::ResearchConfig, research::run_research};
```

should become something like:

```rust
use northflow_trading_bot::{config::ResearchPreset, research::run_research};
```

Only update names that actually exist after your refactor.

## Acceptance Criteria

- `cargo fmt` passes.
- `cargo test` passes.
- `cargo run -- research --config config/research.toml` still invokes the binary.
- No strategy or indicator behavior changed.

---

# Phase P2 — Preserve Presets While Splitting Config Internally

## Goal

Keep the single preset file workflow, but split internal config into typed sections.

The user should still be able to use one preset file like:

```text
config/research.toml
```

But internally the engine should not receive one giant global config object.

## Required New Config Model

Create typed config sections. Suggested layout:

```text
src/config/
  mod.rs
  preset.rs
  mode.rs
  data.rs
  timeframe.rs
  strategy.rs
  backtest.rs
  report.rs
```

You may adjust file names if the codebase demands it, but keep the separation of concerns.

Suggested top-level struct:

```rust
pub struct ResearchPreset {
    pub preset: PresetMeta,
    pub mode: ModeConfig,
    pub data: DataConfig,
    pub timeframes: TimeframeConfig,
    pub strategy_run: StrategyRunConfig,
    pub strategy: StrategyPresetConfig,
    pub risk: RiskConfig,
    pub cost: CostModelConfig,
    pub backtest: EngineBacktestConfig,
    pub report: ReportConfig,
}
```

Suggested supporting structs:

```rust
pub struct PresetMeta {
    pub name: String,
    pub description: Option<String>,
}

pub struct ModeConfig {
    pub run_mode: String,
    pub dry_run: bool,
}

pub struct DataConfig {
    pub symbols: Vec<String>,
    pub data_dir: String,
    pub historical_files: HashMap<String, Vec<PathBuf>>,
}

pub struct TimeframeConfig {
    pub entry_timeframe: Timeframe,
    pub confirmation_timeframe: Timeframe,
    pub screening_timeframe: Timeframe,
}

pub struct StrategyRunConfig {
    pub strategy_id: String,
    pub strategy_run_mode: StrategyRunMode,
    pub strategies: Vec<String>,
}

pub struct EngineBacktestConfig {
    pub conservative_intrabar: bool,
    pub max_bars_held: u32,
    pub entry_geometry_mode: EntryGeometryMode,
    pub entry_lookback_bars: usize,
}

pub struct ReportConfig {
    pub reports_dir: String,
}
```

Use existing `RiskConfig` and `CostModelConfig` if appropriate.

## TOML Parsing Requirement

Replace the manual line-by-line parser with real TOML parsing.

Recommended implementation:

- Add `serde` and `toml` crates if they are not present.
- Use serde-deserializable raw config structs.
- Convert raw config into validated runtime config.
- Keep validation errors clear and explicit.

Do not keep global key scanning as the primary parser.

## Required Preset Shape

Migrate `config/research.toml` toward this shape:

```toml
[preset]
name = "svs2_btc_1m_2020_2025"
description = "BTCUSDT 1m research preset"

[mode]
run_mode = "research"
dry_run = true

[pairs]
symbols = ["BTCUSDT"]
entry_timeframe = "1m"
confirmation_timeframe = "5m"
screening_timeframe = "15m"

[historical_files]
BTCUSDT = [
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2022.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2023.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2024.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2025.csv",
]

[strategy]
strategy_id = "basic_sample_strategy"
strategy_run_mode = "single"
strategies = ["basic_sample_strategy"]

[strategy.basic_sample_strategy]
require_strict_confirmation = true
require_ema_ribbon_alignment = true
allow_neutral_confirmation = false
min_expected_reward_bps = 24.0
min_expected_net_edge_bps = 10.0
min_atr_bps = 12.0
max_atr_bps = 90.0
tp_atr_multiple = 1.8
sl_atr_multiple = 1.0
min_volume_ratio = 1.40
vwap_distance_atr_min = 0.0
vwap_distance_atr_max = 0.45
cooldown_bars = 12
enable_long = true
enable_short = true

[risk]
initial_equity_usd = 5000.0
risk_per_trade_pct = 0.15
max_open_positions = 1
max_leverage = 3.0
min_reward_risk = 1.3
max_daily_loss_pct = 3.0
max_drawdown_pct = 100.0

[cost]
taker_fee_bps = 4.0
slippage_bps = 2.0
spread_bps = 1.0
market_impact_bps = 1.0
stop_slippage_bps = 5.0

[backtest]
conservative_intrabar = true
max_bars_held = 18
entry_geometry_mode = "reanchor_to_actual_entry"
entry_lookback_bars = 120

[reports]
reports_dir = "reports/svs2_btc_entry1m_momentum_quality_2020_2025"
```

## Validation Rules

Implement validation for:

- `run_mode` must be `research` for now.
- `paper` and `live` must remain disabled.
- `symbols` must not be empty.
- `entry_timeframe < confirmation_timeframe < screening_timeframe`.
- `risk.max_open_positions` must be exactly `1` for now, unless Phase P5 implements true multi-position support. Do not pretend multi-position works.
- Cost bps values must be finite and non-negative.
- Risk percentages and leverage must be finite and positive.
- `strategy_run_mode` must be one of `single`, `comparison`, or `multi`, but `multi` must still return a clear not-implemented error.
- `reports_dir` must not be empty.

## Acceptance Criteria

- `config/research.toml` remains a complete preset file.
- Preset loading uses real TOML parsing.
- Config is split internally by concern.
- At least these tests exist:
  - valid preset parses successfully.
  - malformed TOML returns error.
  - invalid timeframe ordering returns error.
  - `max_open_positions > 1` returns a clear error until true multi-position support exists.
- `cargo fmt` passes.
- `cargo test` passes.

---

# Phase P3 — Decouple BacktestEngine From Concrete Strategies

## Goal

Make `BacktestEngine` strategy-agnostic.

## Required Refactor

Remove this pattern from `src/backtest/engine.rs`:

- Direct imports of concrete strategies.
- Internal `ActiveStrategy` enum.
- `match cfg.strategy_id.as_str()` inside the engine.
- Strategy-specific config access from inside the engine.

## Required New Strategy Registry

Create a registry/factory outside `backtest`, preferably:

```text
src/research/strategy_registry.rs
```

or:

```text
src/strategy/registry.rs
```

This registry may know concrete strategies.

Suggested runtime object:

```rust
pub struct StrategyRuntime {
    pub strategy_id: String,
    pub strategy: Box<dyn Strategy>,
    pub signal_cooldown_bars: usize,
}
```

Suggested function:

```rust
pub fn build_strategy_runtime(
    preset: &ResearchPreset,
    strategy_id: &str,
) -> Result<StrategyRuntime, NorthflowError>
```

The registry should support all existing strategies that currently compile.

## Required Engine API

Refactor engine input toward this model:

```rust
pub struct BacktestRunInput<'a> {
    pub symbol: &'a str,
    pub candles: Vec<Candle>,
    pub timeframes: TimeframeConfig,
    pub backtest: EngineBacktestConfig,
    pub risk: RiskConfig,
    pub cost: CostModelConfig,
    pub strategy: &'a dyn Strategy,
    pub signal_cooldown_bars: usize,
}
```

Exact types may vary, but the engine must receive a `Strategy` trait object or generic strategy parameter. It must not choose concrete strategies itself.

## Required Research Orchestrator Changes

The research module should:

1. Load and validate preset.
2. Resolve selected strategies from `StrategyRunConfig`.
3. Build strategy runtime through the registry.
4. Load historical data.
5. Call `BacktestEngine` with data and runtime dependencies.
6. Write reports outside the engine.

## Acceptance Criteria

- `src/backtest/engine.rs` does not import concrete strategy modules.
- `src/backtest/engine.rs` does not match on strategy IDs.
- Strategy selection lives outside `backtest`.
- Existing strategies still work through the registry.
- `cargo fmt` passes.
- `cargo test` passes.

---

# Phase P4 — Fix And Test Cost / Slippage Accounting

## Goal

Resolve the likely double-counting of slippage.

## Current Problem

The fill model applies adverse slippage to execution prices:

- Long entry price is above open.
- Short entry price is below open.
- Long exit price is below base exit price.
- Short exit price is above base exit price.

Then `build_trade` calculates gross PnL from those already-slipped prices and also subtracts nominal slippage from `net_pnl`.

This likely double-counts slippage.

## Required Decision

Choose exactly one accounting model and implement it consistently.

### Preferred Model A — Execution prices include slippage

Use adverse execution prices for entry and exit.

Then:

```text
gross_pnl = pnl based on actual execution prices
net_pnl = gross_pnl - fees - spread_cost - market_impact_cost - stop_extra_cost
```

Do not subtract entry/exit slippage again if the execution prices already include it.

In this model, slippage can still be reported separately as diagnostic information, but it must not be subtracted twice.

### Alternative Model B — Raw prices plus explicit slippage cost

Use raw entry/exit prices for PnL.

Then:

```text
net_pnl = raw_pnl - fees - slippage_cost - spread_cost - market_impact_cost - stop_extra_cost
```

If choosing this model, the fill model must not store adverse prices as execution prices.

## Strong Recommendation

Use Model A because the existing fill model already produces adverse execution prices.

## Required Tests

Add deterministic unit tests with exact numbers.

Example test scenario:

```text
Long trade
entry raw open = 100.00
entry slippage = 10 bps → execution entry = 100.10
exit TP base = 110.00
exit slippage = 10 bps → execution exit = 109.89
qty = 1.0
fee = 0
spread = 0
impact = 0
stop extra = 0

Expected gross/net PnL = 109.89 - 100.10 = 9.79
```

The final `net_pnl` must be `9.79`, not `9.58` or another double-counted value.

Also test:

- Short trade with adverse entry and adverse exit.
- Fee-only trade.
- Spread-only trade.
- Market-impact-only trade.
- Stop-loss trade with stop extra cost.

## Required Reporting Semantics

Clarify fields:

- `fee` = actual fee cost.
- `slippage` = diagnostic slippage impact already embedded in execution price if using Model A.
- `net_pnl` must not subtract diagnostic slippage twice.

If needed, rename internal fields or add comments to avoid future confusion.

## Acceptance Criteria

- Cost/slippage model is documented in code comments.
- Exact arithmetic tests prove no double-counting.
- Existing fill model tests still pass or are updated to the chosen model.
- Reported `net_pnl` is consistent with chosen model.
- `cargo fmt` passes.
- `cargo test` passes.

---

# Phase P5 — Make Single-Position Engine Explicit

## Goal

Avoid pretending the engine supports multiple open positions when it currently stores only one open position.

## Required Decision

For this refactor, choose single-position mode explicitly.

Do **not** implement true multi-position support unless absolutely necessary. That would be a separate large refactor.

## Required Changes

1. Validate config:

```text
max_open_positions must be 1
```

2. If user sets `max_open_positions > 1`, return a clear config error:

```text
multi-position backtest is not implemented; set max_open_positions = 1
```

3. Inside `BacktestEngine`, stop passing misleading `open_positions: 0` blindly if possible.

Use a helper:

```rust
let open_positions = if open_position.is_some() { 1 } else { 0 };
```

Even if the engine only evaluates when no position is open, context should represent reality.

4. Document that the current engine is a single-position deterministic replay engine.

## Acceptance Criteria

- Config validation rejects `max_open_positions > 1`.
- Risk context open position count reflects actual engine state.
- Documentation states single-position limitation.
- `cargo fmt` passes.
- `cargo test` passes.

---

# Phase P6 — Add Deterministic Engine Lifecycle Tests

## Goal

Prove the engine lifecycle without relying on real trading strategy behavior.

Real strategies are allowed to remain, but engine tests should use a deterministic test strategy/stub.

## Required Test Strategy

Create a test-only strategy under `#[cfg(test)]`, for example:

```rust
struct EmitsSignalAtIndex {
    emit_at: u64,
    side: Side,
    entry_price: f64,
    stop_loss: f64,
    take_profit: f64,
}
```

It should implement the existing `Strategy` trait and emit exactly one signal when `ctx.signal_index` or a controlled test counter reaches the desired moment.

Do not add this as a production strategy. Keep it test-only.

## Required Engine Tests

Add integration-style unit tests for:

1. **No signal means no trade**
   - Engine returns result.
   - Trades length is zero.
   - Equity curve has initial point.

2. **Signal enters on next candle open**
   - Signal generated at candle `N`.
   - Entry occurs at candle `N+1` open.
   - Entry timestamp matches `N+1` candle timestamp.

3. **Long take-profit exit**
   - Force long signal.
   - Next candles touch TP.
   - Trade exit reason is `TakeProfit`.

4. **Long stop-loss exit**
   - Force long signal.
   - Next candles touch SL.
   - Trade exit reason is `StopLoss`.

5. **Short take-profit exit**
   - Force short signal.
   - Candle low touches TP.
   - Trade exit reason is `TakeProfit`.

6. **Short stop-loss exit**
   - Force short signal.
   - Candle high touches SL.
   - Trade exit reason is `StopLoss`.

7. **Same-candle SL and TP uses conservative stop-first rule**
   - Candle touches both.
   - Exit reason must be `StopLoss`.

8. **Time exit**
   - No SL/TP touched.
   - `max_bars_held` reached.
   - Exit reason is `TimeExit`.

9. **End-of-backtest exit**
   - Open position remains at last candle.
   - Engine closes it with `EndOfBacktest`.

10. **Equity arithmetic exactness**
    - Use zero fees/costs first.
    - Expected final equity must match exact PnL.
    - Then add fee/spread/impact tests.

11. **No-lookahead boundary**
    - Build synthetic 1m candles.
    - Ensure 5m/15m snapshots are not available before the higher timeframe candle is closed.
    - Test boundary around 5m close and 15m close.

12. **Entry lookback excludes current candle**
    - Keep existing test or strengthen it.

## Acceptance Criteria

- Tests do not depend on real strategy profitability.
- Tests are deterministic.
- Tests verify exact trade lifecycle.
- Tests verify exact accounting.
- `cargo fmt` passes.
- `cargo test` passes.

---

# Phase P7 — Data Quality And Higher-Timeframe Visibility

## Goal

Make dropped higher-timeframe buckets and data quality limitations visible in reports/diagnostics.

## Current Issue

`TimeframeBuilder` drops incomplete or overfilled buckets silently. This is deterministic, but bad for research transparency.

## Required Refactor

Add a build report for timeframe aggregation.

Suggested struct:

```rust
pub struct TimeframeBuildReport {
    pub timeframe: Timeframe,
    pub source_candles: usize,
    pub output_candles: usize,
    pub dropped_incomplete_buckets: usize,
    pub dropped_overfilled_buckets: usize,
}
```

Suggested return type:

```rust
pub struct TimeframeBuildResult {
    pub candles: Vec<Candle>,
    pub report: TimeframeBuildReport,
}
```

If changing the existing API is too invasive, add a new function:

```rust
TimeframeBuilder::build_with_report(...)
```

and keep `build(...)` as a wrapper.

## CandleStore Requirement

`CandleStore` should retain build reports for:

- entry timeframe if aggregated.
- confirmation timeframe.
- screening timeframe.

Suggested fields:

```rust
pub entry_build_report: Option<TimeframeBuildReport>,
pub confirmation_build_report: TimeframeBuildReport,
pub screening_build_report: TimeframeBuildReport,
```

## Report Requirement

Surface these values in one or more of:

```text
report_manifest.json
data_quality_summary.json
backtest_summary.json
CLI output
```

Do not overcomplicate. At minimum, write a deterministic JSON summary in the reports directory.

Suggested file:

```text
reports/data_quality_summary.json
```

Include:

```json
{
  "symbol": "BTCUSDT",
  "raw_1m_candles": 0,
  "entry_candles": 0,
  "confirmation_candles": 0,
  "screening_candles": 0,
  "missing_1m_gaps": 0,
  "data_quality_errors": 0,
  "data_quality_warnings": 0,
  "dropped_entry_buckets": 0,
  "dropped_confirmation_buckets": 0,
  "dropped_screening_buckets": 0
}
```

## Acceptance Criteria

- Dropped HTF buckets are no longer invisible.
- Data quality summary is deterministic.
- Existing data validation still rejects actual data errors.
- Missing aligned gaps remain visible as warnings unless the existing policy says otherwise.
- `cargo fmt` passes.
- `cargo test` passes.

---

# Final Validation Checklist

After all phases are complete, run:

```bash
cargo fmt --check
cargo test
cargo run -- research --config config/research.toml
```

If the dataset is not present in the environment, `cargo run` may report missing historical CSV files. That is acceptable only if the error is clear and expected.

Also inspect that these statements are true:

- `BacktestEngine` does not import concrete strategies.
- `BacktestEngine` does not parse config files.
- `BacktestEngine` does not write report files.
- `BacktestEngine` does not match on strategy IDs.
- `BacktestEngine` can be tested with a stub strategy.
- Preset file still drives research runs.
- Cost/slippage accounting has exact arithmetic tests.
- Single-position limitation is explicit and validated.
- Higher-timeframe dropped buckets are visible.
- Existing indicators still exist.
- Existing strategies still compile through registry.
- `paper` and `live` remain disabled.

---

# Expected Deliverables

At the end of this refactor, provide a concise implementation report containing:

1. Files changed.
2. Config migration summary.
3. Engine API before/after summary.
4. Strategy decoupling summary.
5. Cost/slippage accounting decision.
6. Single-position limitation decision.
7. New tests added.
8. Data quality visibility added.
9. Commands run and results.
10. Any remaining known limitations.

Suggested report path:

```text
docs/engine-cleanup-report.md
```

---

# Out Of Scope

Do not implement these in this task:

- New strategies.
- Indicator tuning.
- Parameter optimization.
- Walk-forward optimization improvements.
- Paper trading.
- Live trading.
- Exchange integration.
- Portfolio/multi-position engine.
- Preset fragments or inheritance.
- Database persistence.
- UI/dashboard.
- AI advisor decisions.

---

# Success Definition

This task is successful when the project has a clean deterministic research engine that can run from a preset, execute strategy signals through a strategy trait, simulate fills and risk with correct accounting, produce auditable ledgers/reports, and pass deterministic engine lifecycle tests without depending on any real strategy being profitable.
