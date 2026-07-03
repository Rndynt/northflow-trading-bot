# Northflow Trading Bot — Forecast ML Module Implementation Prompt

## Reference Roadmap

Before implementing anything, read this roadmap first:

```text
docs/forecast-ml-roadmap.md
```

That roadmap is the source of truth for the architecture, module boundaries, metrics, reports, and phase order.

This prompt turns the roadmap into an actionable implementation task.

---

## Role

You are an implementation agent working on `Rndynt/northflow-trading-bot`, a Rust deterministic crypto trading research/backtest project.

Your task is to implement the first production-quality foundation of the independent forecast ML research module.

The module must support Ridge Regression and Random Forest research workflows, but implementation should be phased correctly. Do not jump straight into strategy integration.

---

## Core Objective

Build an independent `forecast` module that can:

1. load a forecast config,
2. load historical OHLCV,
3. generate feature rows without lookahead,
4. generate forward labels,
5. create chronological walk-forward windows with embargo,
6. train/evaluate Ridge Regression,
7. train/evaluate Random Forest,
8. write forecast research reports,
9. keep all ML/forecast logic independent from the existing strategy and backtest engine.

---

## Non-Negotiable Constraints

Do not violate these rules.

1. Do **not** modify strategy logic.
2. Do **not** add a new production trading strategy.
3. Do **not** make ML emit production `Signal` objects.
4. Do **not** make `BacktestEngine` depend on `forecast` or ML code.
5. Do **not** make `strategy` depend on `forecast` or ML code.
6. Do **not** change risk sizing.
7. Do **not** change fill simulation.
8. Do **not** change accounting.
9. Do **not** change indicator formulas unless required for read-only access.
10. Do **not** enable paper trading.
11. Do **not** enable live trading.
12. Do **not** add exchange/network/API/LLM calls.
13. Do **not** use random train/test split.
14. Do **not** train on future data.
15. Do **not** standardize using test data.
16. Do **not** write generated forecast reports into git.
17. Do **not** claim profitability.
18. Do **not** skip tests because full historical data is unavailable.
19. Do **not** require Python.
20. Do **not** add heavy dependencies without documenting why.

---

## Required Phase Order

Follow the phase order in `docs/forecast-ml-roadmap.md`.

For the first implementation pass, complete these phases if practical:

```text
F0 — Forecast module skeleton
F1 — Dataset builder
F2 — Walk-forward splitter
F3 — Ridge Regression baseline
F4 — Random Forest model
F5 — Model comparison and recommendation
```

If F3-F5 are too large for one pass, complete F0-F2 first and document the remaining work clearly. However, do not leave partially implemented model stubs that look complete but do not perform real out-of-sample evaluation.

---

# Phase F0 — Forecast Module Skeleton

## Goal

Create the independent forecast module and CLI command.

## Required Files

Create:

```text
src/forecast/mod.rs
src/forecast/config.rs
src/forecast/runner.rs
config/forecast.toml
```

Update:

```text
src/lib.rs
src/main.rs
```

## CLI Requirement

Add command:

```bash
cargo run -- forecast --config config/forecast.toml
```

Keep existing command unchanged:

```bash
cargo run -- research --config config/research.toml
```

`forecast` command must not call `run_research`.

## Config Requirement

Create `ForecastConfig` independent from `ResearchConfig`.

Do not reuse `ResearchConfig` directly.

Suggested struct:

```rust
pub struct ForecastConfig {
    pub symbols: Vec<String>,
    pub source_timeframe: String,
    pub entry_timeframe: String,
    pub forecast_horizon: String,
    pub data_dir: String,
    pub historical_files: HashMap<String, Vec<PathBuf>>,
    pub enabled_features: Vec<String>,
    pub label_target: String,
    pub horizon_bars: usize,
    pub cost: CostModelConfig,
    pub enabled_models: Vec<String>,
    pub ridge: RidgeConfig,
    pub random_forest: RandomForestConfig,
    pub walk_forward: WalkForwardConfig,
    pub reports_dir: String,
}
```

Validation:

- `run_mode = "forecast"`.
- symbols non-empty.
- `source_timeframe = "1m"` for now.
- `horizon_bars > 0`.
- enabled features must be known.
- label target must be known.
- models must be known: `ridge`, `random_forest`.
- cost bps finite and non-negative.
- walk-forward months positive.
- reports dir non-empty.

## CLI Output

Use the same clean section style as research logging.

Initial forecast command should print:

```text
Northflow Forecast Research
===========================
Run Plan
--------
Mode             : forecast
Symbols          : BTCUSDT
Source TF        : 1m
Entry TF         : 1m
Forecast Horizon : 15m
Models           : ridge, random_forest
Reports Dir      : reports/forecast/btcusdt_1m_h15
```

## Acceptance Criteria

- `cargo run -- forecast --config config/forecast.toml` parses config and prints run plan.
- Existing `research` command still works.
- No strategy/backtest behavior changes.
- Tests cover valid and invalid forecast config.

---

# Phase F1 — Dataset Builder

## Goal

Build forecast dataset rows from historical OHLCV and existing indicators.

## Required Files

Create:

```text
src/forecast/features.rs
src/forecast/labels.rs
src/forecast/dataset.rs
src/forecast/reports.rs
```

## Dataset Structs

Implement something equivalent to:

```rust
pub struct ForecastDataset {
    pub symbol: String,
    pub feature_names: Vec<String>,
    pub rows: Vec<ForecastRow>,
    pub summary: DatasetSummary,
}

pub struct ForecastRow {
    pub timestamp: i64,
    pub close: f64,
    pub features: Vec<f64>,
    pub future_return_bps: f64,
    pub future_return_after_cost_bps: f64,
}
```

## Required Features

Implement initial features from roadmap:

```text
return_1m
return_5m
return_15m
atr_bps
volume_ratio
vwap_distance_bps
ema_8_21_spread_bps
range_position
hour_of_day
day_of_week
```

Every feature must use only current/past data.

No future candle access is allowed in features.

## Required Labels

Implement:

```text
future_return_bps
future_return_after_cost_bps
future_direction_after_cost
```

`future_return_after_cost_bps` must subtract estimated round-trip cost:

```text
taker_fee_bps * 2
+ slippage_bps * 2
+ spread_bps
+ market_impact_bps
```

Do not include stop slippage in plain forward-return labels.

## Row Skipping

Skip rows where:

- any enabled feature is missing,
- any feature is NaN or infinite,
- label horizon goes past dataset end,
- close is invalid,
- label is NaN or infinite.

Count skip reasons in `DatasetSummary`.

## Reports

Write:

```text
dataset_summary.json
feature_summary.csv
label_summary.json
```

## Acceptance Criteria

- Dataset builder works with synthetic candles.
- No-lookahead feature tests exist.
- Exact label horizon tests exist.
- Skip reason counters exist.
- Reports are deterministic.

---

# Phase F2 — Walk-Forward Splitter

## Goal

Create chronological walk-forward train/test windows with embargo.

## Required File

Create:

```text
src/forecast/split.rs
```

## Required Behavior

Do not random split.

Given dataset rows sorted by timestamp, create windows:

```text
train period
embargo period
test period
```

Rules:

- train rows must occur before test rows.
- embargo must separate train and test.
- test rows must not overlap train labels.
- windows with insufficient rows should be skipped and counted.

## Reports

Write:

```text
walk_forward_windows.csv
```

Fields:

```text
window_id
train_start
train_end
test_start
test_end
train_rows
test_rows
embargo_bars
```

## Acceptance Criteria

- Tests cover chronological ordering.
- Tests cover embargo boundary.
- No random split exists.

---

# Phase F3 — Ridge Regression Baseline

## Goal

Implement Ridge Regression out-of-sample baseline.

## Required File

Create:

```text
src/forecast/models/ridge.rs
```

## Implementation Rules

- Train only on train rows.
- Standardize features using train-only mean/std.
- Apply train standardization to test rows.
- Predict test rows.
- Evaluate only on test rows.
- Do not train on full dataset before evaluation.

## Model Output

For each test row:

```text
timestamp
actual_bps
predicted_bps
```

## Required Metrics

Implement in:

```text
src/forecast/metrics.rs
src/forecast/evaluation.rs
```

Metrics:

```text
MAE
RMSE
correlation
directional_accuracy
avg_predicted_bps
avg_actual_bps
avg_actual_after_cost_bps
prediction_bucket_summary
```

## Prediction Buckets

Use 10 deciles.

Each decile:

```text
bucket_id
min_prediction_bps
max_prediction_bps
row_count
avg_prediction_bps
avg_actual_bps
avg_actual_after_cost_bps
hit_rate_after_cost
```

## Reports

Write:

```text
ridge_summary.json
ridge_prediction_buckets.csv
ridge_walk_forward.csv
```

## Acceptance Criteria

- Ridge predictions are out-of-sample.
- Standardization does not use test rows.
- Metrics are deterministic.
- Prediction bucket report exists.

---

# Phase F4 — Random Forest Model

## Goal

Implement Random Forest forecast evaluation.

## Required File

Create:

```text
src/forecast/models/random_forest.rs
```

## Dependency Decision

Before adding a Rust ML crate, inspect existing dependencies.

If adding a crate, document in the implementation report:

- why it was chosen,
- compile impact,
- license compatibility,
- deterministic configuration,
- limitations.

Do not add Python.

Do not call external services.

## Initial Mode

Prefer Random Forest regression for first implementation if it is simpler and deterministic.

Target:

```text
future_return_after_cost_bps
```

## Required Metrics

Same regression metrics as Ridge:

```text
MAE
RMSE
correlation
directional_accuracy
prediction_bucket_summary
```

If feature importance is supported, write it.

If not supported, document `feature_importance_unavailable` in the report.

## Reports

Write:

```text
random_forest_summary.json
random_forest_prediction_buckets.csv
random_forest_walk_forward.csv
random_forest_feature_importance.csv
```

## Acceptance Criteria

- Random Forest uses train rows only.
- Evaluation uses test rows only.
- Results are compared to Ridge.
- No strategy integration exists.

---

# Phase F5 — Model Comparison

## Goal

Compare Ridge and Random Forest conservatively.

## Required File

Implement comparison logic in:

```text
src/forecast/evaluation.rs
```

or a separate:

```text
src/forecast/comparison.rs
```

## Report

Write:

```text
model_comparison.json
forecast_run_manifest.json
```

## Recommendation Values

Use one of:

```text
no_predictive_signal_detected
weak_signal_needs_more_validation
candidate_for_backtest_filter_phase
reject_due_to_instability
reject_due_to_cost_adjusted_decay
```

Recommendation must not claim profitability.

## Acceptance Criteria

- comparison report references both models,
- recommendation is conservative,
- output explains the decision basis.

---

# Required Report Layout

Reports must be written under:

```text
reports/forecast/<run_name>/
```

`reports/` is already ignored by git and must stay ignored.

Required files after full implementation:

```text
dataset_summary.json
feature_summary.csv
label_summary.json
walk_forward_windows.csv
ridge_summary.json
ridge_prediction_buckets.csv
ridge_walk_forward.csv
random_forest_summary.json
random_forest_prediction_buckets.csv
random_forest_walk_forward.csv
random_forest_feature_importance.csv
model_comparison.json
forecast_run_manifest.json
```

If a phase is intentionally incomplete, do not create fake complete reports. Create an implementation report documenting what is missing.

---

# Required Documentation

Create or update:

```text
docs/forecast-ml-implementation-report.md
```

Include:

1. roadmap reference,
2. phases completed,
3. module boundary summary,
4. config schema summary,
5. feature list implemented,
6. label list implemented,
7. walk-forward policy,
8. Ridge implementation details,
9. Random Forest implementation details,
10. metrics implemented,
11. report files written,
12. dependency decisions,
13. commands run and results,
14. remaining limitations,
15. explicit confirmation that strategy/backtest/risk/accounting logic was not changed.

---

# Required Tests

Add tests for:

```text
ForecastConfig valid parse
ForecastConfig invalid source_timeframe rejected
ForecastConfig invalid model rejected
ForecastConfig unknown feature rejected
feature computation exactness
feature computation no-lookahead
label computation exact horizon
cost-adjusted label calculation
invalid row skipping
walk-forward chronological ordering
embargo boundary
metric calculations
prediction bucket generation
ridge standardization train-only behavior
ridge prediction smoke test
random forest smoke test if implemented
report writer smoke test
```

Tests must use small synthetic datasets.

Do not require the full BTCUSDT 2020-2025 dataset for tests.

---

# Final Search Requirements

Search for unwanted coupling:

```text
forecast::
BacktestEngine
BasicSampleStrategy
build_strategy_runtime
Signal
paper
live
```

Expected:

- `forecast` may appear in `main.rs`, `lib.rs`, and `src/forecast/*`.
- `BacktestEngine` must not import forecast.
- `strategy` must not import forecast.
- forecast module must not emit production `Signal`.
- paper/live remain disabled.

---

# Final Validation Commands

Run:

```bash
cargo fmt --check
cargo test
cargo run -- forecast --config config/forecast.toml
```

Also run existing research command if data exists:

```bash
cargo run -- research --config config/research.toml
```

If historical data is missing in the implementation environment, the forecast command must fail or report missing files clearly without panicking.

---

# Success Definition

This task is successful when:

1. `docs/forecast-ml-roadmap.md` is followed.
2. `forecast` command exists.
3. `ForecastConfig` is independent from `ResearchConfig`.
4. forecast module can generate deterministic datasets.
5. labels are cost-aware where configured.
6. walk-forward splitting is chronological and embargo-aware.
7. Ridge Regression is evaluated out-of-sample.
8. Random Forest is evaluated out-of-sample or explicitly documented as pending if not implemented.
9. metrics and prediction buckets are written.
10. model comparison is conservative.
11. reports are generated under ignored `reports/forecast/...`.
12. no strategy/backtest/risk/accounting behavior changes.
13. `cargo fmt --check` passes.
14. `cargo test` passes.
15. `docs/forecast-ml-implementation-report.md` documents the work.

---

## Out Of Scope

Do not implement these in this task:

- ML-backed production trading strategy.
- ForecastScorer integration into backtest.
- Model persistence/artifacts for live use.
- Paper trading.
- Live trading.
- Exchange integration.
- Hyperparameter optimization.
- Neural networks.
- Python pipeline.
- Dashboard/UI.
- Database persistence.
