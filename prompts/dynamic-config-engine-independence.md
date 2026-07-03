# Northflow Trading Bot — Dynamic Config & Engine Independence Prompt

## Role

You are an implementation agent working on `Rndynt/northflow-trading-bot`, a Rust deterministic crypto trading research/backtest project.

Your task is to fix the remaining architecture and runtime issues after the engine cleanup, sample-only strategy cleanup, and market regime restore.

This task is about correctness, dynamic configuration, engine boundaries, and preventing misleading runtime behavior.

This is not a strategy research task. This is not a profitability task.

---

## Primary Objective

Make the research runtime actually dynamic and independent by fixing all known hardcoding and boundary issues:

1. Config parser must read the same sections used by `config/research.toml`.
2. Single research mode must actually run the backtest again.
3. `BacktestEngine` must be decoupled from `ResearchConfig`, file loading, and strategy registry.
4. Source timeframe must be explicit instead of silently assumed.
5. Runtime/CLI output must not hardcode `1m/5m/15m` labels when config is dynamic.
6. Strategy ID constant must be centralized.
7. Existing sample-only strategy policy must remain intact.

---

## Non-Negotiable Constraints

Do not violate these rules.

1. Do **not** restore old strategies.
2. Do **not** add a new production strategy.
3. Do **not** accept old strategy IDs again.
4. Do **not** tune strategy logic for profitability.
5. Do **not** change indicator formulas unless required to fix compile errors.
6. Do **not** enable `paper` mode.
7. Do **not** enable `live` mode.
8. Do **not** add exchange/network/API/LLM calls.
9. Do **not** hide failing tests.
10. Do **not** remove data-quality validation.
11. Do **not** make source timeframe dynamic by silently accepting arbitrary data without validation.
12. Do **not** keep misleading hardcoded text in CLI output.
13. Do **not** let `cargo run -- research --config ...` pass without actually executing the intended single-mode research run.

---

## Current Known Issues

These are the issues you must fix.

### Issue 1 — Config section mismatch

Current `config/research.toml` uses sections like:

```toml
[pairs]
symbols = ["BTCUSDT"]
entry_timeframe = "1m"
screening_timeframe = "15m"
confirmation_timeframe = "5m"

[strategy]
strategy_id = "basic_sample_strategy"
min_confidence = 90

[backtest]
data_dir = "data/historical"
reports_dir = "reports/basic_sample_btc_entry1m_2020_2025"
strategy_run_mode = "single"
strategies = ["basic_sample_strategy"]
```

But the parser currently reads some values from other sections or top-level fields:

- `symbols` from top-level, not `[pairs]`.
- timeframes from `[timeframes]` or top-level, not `[pairs]`.
- `data_dir` from `[data]` or top-level, not `[backtest]`.
- `reports_dir` from `[reports]` or top-level, not `[backtest]`.
- `min_confidence` from `[backtest]`, not `[strategy]`.

This means the preset can look dynamic while the runtime silently falls back to defaults.

### Issue 2 — Single mode no longer runs backtest

`run_single_strategy` currently prints status and returns `Ok(())` without running symbols/backtests.

This is unacceptable. `cargo run -- research --config config/research.toml` must execute the selected strategy over the configured symbol(s), or clearly report missing data after attempting the run path.

### Issue 3 — BacktestEngine is still coupled to global config and strategy registry

`BacktestEngine::run` currently receives `&ResearchConfig`, loads historical files, parses timeframe strings, builds the `CandleStore`, builds the strategy runtime through the registry, and then runs replay.

The engine should not do these orchestration responsibilities.

### Issue 4 — Source timeframe is silently hardcoded to 1m

The data pipeline assumes 1m source data. That may be acceptable for now, but it must be explicit in config and validation.

If the project is still 1m-source-only, make that clear:

```toml
[data]
source_timeframe = "1m"
```

or:

```toml
[backtest]
source_timeframe = "1m"
```

Preferred location: `[data]`.

### Issue 5 — CLI/research output hardcodes `1m/5m/15m`

Some output text says `1m`, `5m`, or `15m` even though config supports different role values. Output must render actual configured values.

### Issue 6 — Strategy ID constant is duplicated

`basic_sample_strategy` appears as literals in several places. Centralize it into one public constant used by strategy implementation, registry, and config validation.

---

# Desired Final Architecture

External command should remain:

```bash
cargo run -- research --config config/research.toml
```

Internal responsibilities should be:

```text
CLI
  - read command args
  - load ResearchConfig / ResearchPreset
  - call run_research

Research Orchestrator
  - validate config
  - resolve selected strategies
  - build strategy runtime from registry
  - load historical data
  - build CandleStore
  - create BacktestRunInput
  - call BacktestEngine
  - write reports

BacktestEngine
  - receive prepared candles/store/config/strategy
  - replay candles deterministically
  - evaluate provided Strategy trait object
  - risk assess
  - simulate fills
  - produce BacktestResult
  - no file IO
  - no strategy registry lookup
  - no TOML/config parsing
  - no report writing
```

---

# Phase D0 — Baseline Inspection

## Goal

Confirm the current state before editing.

## Required Steps

Inspect these files:

```text
config/research.toml
src/main.rs
src/config/mod.rs
src/research/mod.rs
src/backtest/engine.rs
src/backtest/mod.rs
src/strategy/basic_sample.rs
src/strategy/registry.rs
src/strategy/mod.rs
src/market/candle_store.rs
src/market/timeframe_builder.rs
src/core/timeframe.rs
```

Create or update:

```text
docs/dynamic-config-engine-independence-report.md
```

Add a baseline section documenting:

- Existing config section mismatch.
- Single mode behavior before fix.
- Current `BacktestEngine` dependencies.
- Current source timeframe assumption.
- Existing hardcoded runtime output text.

## Acceptance Criteria

- Baseline issues are documented.
- No behavior changed yet in this phase.

---

# Phase D1 — Fix Config Section Parsing

## Goal

Make `ResearchConfig` parse the actual config file structure correctly.

## Required Decision

Keep `config/research.toml` as a single complete preset file.

You may either:

### Option A — Minimal safe fix

Keep current `ResearchConfig` struct but fix `apply_toml_value` to read the correct sections.

### Option B — Better typed refactor

Split config into typed raw structs with serde and convert into runtime `ResearchConfig`.

Option B is better long-term, but Option A is acceptable if completed with tests and no ambiguity.

## Required Parsing Rules

The parser must read:

### `[mode]`

```toml
run_mode = "research"
dry_run = true
```

Validation:

- `run_mode` must be `research` for now.
- `dry_run` may exist but must not enable paper/live.

### `[pairs]`

```toml
symbols = ["BTCUSDT"]
entry_timeframe = "1m"
confirmation_timeframe = "5m"
screening_timeframe = "15m"
```

Validation:

- `symbols` must not be empty.
- All symbols must be non-empty strings.
- `entry_timeframe < confirmation_timeframe < screening_timeframe`.

### `[data]` — new preferred section

Add this section to `config/research.toml`:

```toml
[data]
source_timeframe = "1m"
data_dir = "data/historical"
```

Validation:

- `source_timeframe` must exist.
- For now, `source_timeframe` must be `1m`.
- If user sets any other value, return a clear error:

```text
source_timeframe currently supports only '1m' because higher timeframes are built from 1m candles
```

Backward compatibility:

- If `data_dir` still exists under `[backtest]`, either support it temporarily with a deprecation comment/test, or move it to `[data]` and update tests accordingly.
- Prefer final config to use `[data]`.

### `[historical_files]`

Keep this map:

```toml
[historical_files]
BTCUSDT = ["..."]
```

Validation:

- If a symbol has explicit files, preserve file order.
- Do not silently sort files unless documented.
- If no files are configured for a symbol, fallback to `data_dir/<SYMBOL>.csv` is acceptable.

### `[strategy]`

```toml
strategy_id = "basic_sample_strategy"
min_confidence = 90
```

Validation:

- `strategy_id` must be `basic_sample_strategy`.
- `min_confidence` must be `0..=100`.

### `[risk]`

Existing risk fields remain.

Validation:

- numeric values must be finite and valid.
- `max_open_positions` must be exactly `1` for now.

### `[cost]`

Existing cost fields remain.

Validation:

- bps values must be finite and non-negative.

### `[backtest]`

```toml
reports_dir = "reports/basic_sample_btc_entry1m_2020_2025"
conservative_intrabar = true
max_bars_held = 18
entry_geometry_mode = "reanchor_to_actual_entry"
entry_lookback_bars = 120
strategy_run_mode = "single"
strategies = ["basic_sample_strategy"]
```

Validation:

- `reports_dir` must not be empty.
- `max_bars_held` must be > 0.
- `entry_geometry_mode` must be valid.
- `strategy_run_mode` must be `single`, `comparison`, or `multi`.
- `multi` remains not implemented.
- All strategy IDs in `strategies` must be `basic_sample_strategy`.

## Required Tests

Add/update tests for:

1. `[pairs].symbols` is parsed.
2. `[pairs].entry_timeframe` is parsed.
3. `[pairs].confirmation_timeframe` is parsed.
4. `[pairs].screening_timeframe` is parsed.
5. `[data].source_timeframe = "1m"` is accepted.
6. `[data].source_timeframe = "5m"` is rejected.
7. `[data].data_dir` is parsed.
8. `[backtest].reports_dir` is parsed.
9. `[strategy].min_confidence` is parsed.
10. invalid `min_confidence > 100` is rejected.
11. invalid/missing symbols are rejected.
12. old strategy IDs remain rejected.
13. duplicate strategy IDs in list are rejected.
14. `strategy_run_mode = "multi"` returns clear not-implemented error.
15. malformed TOML returns error.

## Acceptance Criteria

- `config/research.toml` values are actually used.
- No silent fallback hides valid configured values.
- Tests prove section parsing works.
- `cargo fmt --check` passes.
- `cargo test` passes.

---

# Phase D2 — Update `config/research.toml`

## Goal

Make the preset structure match the parser and future expectations.

## Required Final Shape

Update `config/research.toml` to this shape or equivalent:

```toml
[mode]
run_mode = "research"
dry_run = true

[pairs]
symbols = ["BTCUSDT"]
entry_timeframe = "1m"
confirmation_timeframe = "5m"
screening_timeframe = "15m"

[data]
source_timeframe = "1m"
data_dir = "data/historical"

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
min_confidence = 90

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
reports_dir = "reports/basic_sample_btc_entry1m_2020_2025"
conservative_intrabar = true
max_bars_held = 18
entry_geometry_mode = "reanchor_to_actual_entry"
entry_lookback_bars = 120
strategy_run_mode = "single"
strategies = ["basic_sample_strategy"]
```

## Acceptance Criteria

- The preset has no stale old strategy fields.
- The preset has explicit `source_timeframe`.
- The preset parses and validates.
- Runtime uses the configured values.

---

# Phase D3 — Restore Single Mode Execution

## Goal

`strategy_run_mode = "single"` must actually run the configured symbols.

## Required Fix

`run_single_strategy` must:

1. Resolve selected strategy ID.
2. Build a per-run config if needed.
3. Print generic strategy info.
4. Loop over `cfg.symbols`.
5. Call the actual symbol runner.
6. Write reports through the existing report path.
7. Return errors only when actual errors occur.

Suggested shape:

```rust
fn run_single_strategy(cfg: &ResearchConfig) -> Result<(), String> {
    let strategies = cfg.selected_strategies().map_err(|e| format!("{e}"))?;
    let strategy_id = strategies
        .into_iter()
        .next()
        .unwrap_or_else(|| cfg.strategy_id.clone());

    let run_cfg = cfg.with_strategy_for_run(&strategy_id, cfg.reports_dir.clone());

    print_single_run_header(&run_cfg, &strategy_id);

    for symbol in &run_cfg.symbols {
        run_symbol_verbose(&run_cfg, symbol);
    }

    print_runtime_capabilities(&run_cfg, &strategy_id);
    Ok(())
}
```

Do not just print and return.

## Required Tests

Add tests where possible. If direct testing of CLI is hard, test the orchestration function or add a small integration test with a temporary CSV.

At minimum:

1. Single mode calls the symbol runner when data exists.
2. Single mode reports missing CSV clearly when data is missing.
3. Research command does not succeed by only printing readiness text.

## Acceptance Criteria

- `cargo run -- research --config config/research.toml` attempts actual configured symbol execution.
- Missing data is reported through the symbol path, not skipped by a no-op single mode.
- Existing report writing still works when data exists.
- `cargo test` passes.

---

# Phase D4 — Decouple BacktestEngine From ResearchConfig And Strategy Registry

## Goal

Make `BacktestEngine` a clean replay engine.

## Required New Types

Introduce a prepared input type. Suggested:

```rust
pub struct BacktestRunInput<'a> {
    pub symbol: Symbol,
    pub store: CandleStore,
    pub timeframes: TimeframeRoles,
    pub backtest: BacktestConfig,
    pub risk: RiskConfig,
    pub cost: CostModelConfig,
    pub strategy: &'a dyn Strategy,
    pub min_confidence: u8,
    pub entry_lookback_bars: usize,
    pub cooldown_bars: usize,
}
```

Suggested timeframe roles type:

```rust
pub struct TimeframeRoles {
    pub entry: Timeframe,
    pub confirmation: Timeframe,
    pub screening: Timeframe,
}
```

Or use an existing config struct if already available.

## Required Engine API

Refactor engine toward:

```rust
impl BacktestEngine {
    pub fn run(input: BacktestRunInput<'_>) -> Result<BacktestResult, NorthflowError> {
        ...
    }
}
```

or:

```rust
pub fn run_prepared(input: BacktestRunInput<'_>) -> Result<BacktestResult, NorthflowError>
```

If you need to preserve the old API temporarily, keep it as a wrapper outside the engine or mark it as deprecated. Preferred: remove the old API once research orchestrator is migrated.

## BacktestEngine Must Not Do These

After refactor, `src/backtest/engine.rs` must not:

- import `ResearchConfig`.
- import `build_strategy_runtime`.
- call `OhlcvLoader`.
- check file existence.
- parse timeframe strings from config.
- write reports.
- know strategy IDs.
- construct concrete strategies.

## Research Orchestrator Responsibility

Move these responsibilities to `src/research/mod.rs` or helper modules:

1. Resolve strategy runtime through registry.
2. Build strategy trait object.
3. Load historical files through `OhlcvLoader`.
4. Validate data quality.
5. Parse timeframe roles.
6. Build `CandleStore`.
7. Build `BacktestRunInput`.
8. Call `BacktestEngine::run(input)`.
9. Write reports.

## Required Tests

Add tests to enforce boundary:

1. Engine can run with a test-only stub strategy without registry.
2. Engine can run with a prepared `CandleStore` without file IO.
3. Engine lifecycle tests still pass.
4. Unknown strategy ID is rejected in config/registry, not inside engine.

## Acceptance Criteria

- `src/backtest/engine.rs` has no dependency on `ResearchConfig`.
- `src/backtest/engine.rs` has no dependency on strategy registry.
- `src/backtest/engine.rs` has no dependency on `OhlcvLoader`.
- Engine receives a `dyn Strategy` or equivalent from outside.
- Research orchestrator owns file loading and strategy resolution.
- `cargo test` passes.

---

# Phase D5 — Make Source Timeframe Explicit

## Goal

Stop silently assuming source data is 1m.

## Required Behavior

Add `source_timeframe` to config runtime.

For now:

```text
source_timeframe must be 1m
```

The pipeline may still build all higher timeframes from 1m candles, but the constraint must be explicit and validated.

## Required Code Changes

1. Add `source_timeframe` field to `ResearchConfig` or equivalent data config.
2. Parse it from `[data].source_timeframe`.
3. Validate it equals `1m` for now.
4. Use it in user-facing messages.
5. Document this limitation.

## Optional Better Design

Add a typed data config:

```rust
pub struct DataConfig {
    pub source_timeframe: Timeframe,
    pub data_dir: String,
    pub historical_files: HashMap<String, Vec<PathBuf>>,
}
```

## Acceptance Criteria

- Source timeframe is explicit in config.
- Non-1m source timeframe fails clearly.
- CLI/docs explain higher timeframe candles are built from 1m source data.
- Tests cover accepted/rejected source timeframe.

---

# Phase D6 — Remove Hardcoded Runtime Output Text

## Goal

Runtime output must reflect actual configured values.

## Required Fixes

Replace output like:

```text
entry_timeframe = "{}"  (1m → entry & execution)
screening_timeframe = "{}" (15m → regime bias)
confirmation_timeframe = "{}"  (5m → confirmation)
no lookahead across 5m / 15m candles
```

with dynamic output:

```text
entry_timeframe        = "{entry}"        → entry & execution
confirmation_timeframe = "{confirmation}" → intermediate confirmation
screening_timeframe    = "{screening}"    → market regime / bias
source_timeframe       = "{source}"       → raw OHLCV source
no lookahead across configured confirmation/screening timeframes
```

Also update help text if it says only `data/historical/<SYMBOL>.csv` while the preset uses explicit yearly paths. Make it generic:

```text
Historical data:
  Configure [historical_files] in the preset, or place fallback CSV at data_dir/<SYMBOL>.csv.
  Source data currently must be 1m OHLCV.
```

## Acceptance Criteria

- No runtime output claims fixed 1m/5m/15m roles unless those are actual configured values.
- Help text matches config behavior.
- `cargo test` passes.

---

# Phase D7 — Centralize Strategy ID Constant

## Goal

Avoid duplicating `basic_sample_strategy` as string literals across layers.

## Required Design

Create a single source of truth, for example:

```rust
pub const BASIC_SAMPLE_STRATEGY_ID: &str = "basic_sample_strategy";
```

Preferred location:

```text
src/strategy/basic_sample.rs
```

or:

```text
src/strategy/registry.rs
```

Better general location:

```text
src/strategy/ids.rs
```

Suggested:

```rust
// src/strategy/ids.rs
pub const BASIC_SAMPLE_STRATEGY_ID: &str = "basic_sample_strategy";
```

Then use it in:

- `BasicSampleStrategy::strategy_id()`.
- `build_strategy_runtime`.
- config validation.
- tests.
- config helper messages.

Avoid defining the same constant separately in `config`.

## Acceptance Criteria

- One source constant exists.
- Strategy, registry, and config validation use it.
- Tests still prove old strategy IDs are rejected.
- `cargo test` passes.

---

# Phase D8 — Add Dynamic-Timeframe Tests

## Goal

Prove the engine/research path honors configured timeframe roles.

## Required Tests

Add tests for non-default role combinations where possible:

1. `entry=5m`, `confirmation=15m`, `screening=1h` parses and validates.
2. `entry=1m`, `confirmation=5m`, `screening=15m` parses and validates.
3. `entry=5m`, `confirmation=5m`, `screening=1h` is rejected.
4. `entry=15m`, `confirmation=5m`, `screening=1h` is rejected.
5. `CandleStore::build` supports configured roles from config.
6. No-lookahead tests use timeframe values rather than hardcoded 5m/15m constants.

## Acceptance Criteria

- Timeframe roles are actually dynamic.
- Tests fail if parser ignores `[pairs]` values.
- Tests fail if engine hardcodes default role values.
- `cargo test` passes.

---

# Phase D9 — Documentation And Final Report

## Goal

Document the completed dynamic config and engine independence cleanup.

## Required Report

Create or update:

```text
docs/dynamic-config-engine-independence-report.md
```

Include:

1. Config parser fixes.
2. Final config section schema.
3. Source timeframe policy.
4. Single-mode execution fix.
5. Engine boundary before/after.
6. Strategy registry boundary.
7. Dynamic output cleanup.
8. Strategy ID constant location.
9. Tests added/updated.
10. Commands run and results.
11. Remaining limitations.

## Remaining Limitations To Document

At minimum document:

- Source data currently supports only `1m`.
- Engine is still single-position by policy.
- Only one production strategy exists intentionally.
- Paper/live remain disabled.

---

# Final Search Requirements

Search for misleading hardcoded text and stale strategy references.

## Search terms

```text
1m  → entry
5m  → confirmation
15m → regime
no lookahead across 5m / 15m
source timeframe assumes
basic_sample_strategy
screened_vwap_scalp
screened_vwap_scalp_v2
ema_trend_pullback
vwap_reclaim_short
mean_revert
liquidity_sweep_reclaim
ResearchConfig
build_strategy_runtime
OhlcvLoader
```

## Expected Results

- Old strategy names may remain only in prompt/report history or rejection tests.
- `basic_sample_strategy` should be centralized through the shared constant, except in `config/research.toml` and documentation.
- `build_strategy_runtime` must not appear in `src/backtest/engine.rs`.
- `OhlcvLoader` must not appear in `src/backtest/engine.rs`.
- `ResearchConfig` must not appear in `src/backtest/engine.rs`.
- Hardcoded output text claiming fixed `1m/5m/15m` must be removed or made dynamic.

---

# Final Validation Commands

Run:

```bash
cargo fmt --check
cargo test
cargo run -- research --config config/research.toml
```

If historical data files are missing in the environment, the research command may stop with a clear missing-data message. That is acceptable only if:

1. config parsing succeeds,
2. strategy registry validation succeeds,
3. single-mode execution attempts to process configured symbols,
4. the missing-data message names the expected path(s).

If historical data exists, reports must be produced under the configured `reports_dir`.

---

# Success Definition

This task is successful when:

1. `config/research.toml` is parsed according to its actual sections.
2. Changing `[pairs]` timeframes changes runtime behavior.
3. Changing `[data].data_dir` changes fallback data path behavior.
4. Changing `[backtest].reports_dir` changes report output location.
5. Changing `[strategy].min_confidence` changes strategy context.
6. Single mode actually runs configured symbols.
7. `BacktestEngine` no longer depends on `ResearchConfig`, `OhlcvLoader`, or strategy registry.
8. Source timeframe is explicit and validated as `1m` for now.
9. Runtime output uses configured timeframe values.
10. Strategy ID is centralized.
11. Only `basic_sample_strategy` is accepted.
12. Old strategy IDs remain rejected.
13. `cargo fmt --check` passes.
14. `cargo test` passes.
15. The final documentation report exists.

---

# Out Of Scope

Do not implement these in this task:

- New strategies.
- Strategy tuning.
- Profitability optimization.
- Paper trading.
- Live trading.
- Exchange integration.
- Multi-position engine.
- Non-1m source data support beyond explicit validation and clear rejection.
- UI/dashboard.
- Database persistence.
- Preset inheritance/fragments.
