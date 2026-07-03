# Northflow Trading Bot — Forecast ML Research Roadmap

## Purpose

This roadmap defines how Northflow should add a forecasting-focused machine learning module using Ridge Regression and Random Forest without contaminating the existing deterministic backtest engine, strategy layer, risk engine, or reporting pipeline.

The goal is not to add an AI trading strategy immediately.

The goal is to build an independent research module that can answer a narrower and more important question first:

> Do the current market features contain stable, cost-adjusted predictive signal across time, regime, and walk-forward validation?

Only after that question is answered should model outputs be considered as filters or scorers for trading signals.

---

## Current Project Context

Northflow currently has a deterministic research/backtest foundation:

- `research` CLI mode.
- dynamic preset config.
- explicit timeframe roles.
- `source_timeframe = "1m"` policy.
- sample-only production strategy: `basic_sample_strategy`.
- strategy registry accepting only the sample strategy.
- market regime classification as a generic market component.
- strategy-agnostic `BacktestEngine` accepting prepared input.
- risk, fill, cost, and report modules separated from strategy logic.
- generated reports ignored through `.gitignore`.

The ML/forecasting module must preserve those boundaries.

---

## Core Principle

The ML module must be a **forecast research module**, not a direct trading strategy.

Correct boundary:

```text
historical OHLCV
  -> feature builder
  -> label builder
  -> dataset builder
  -> walk-forward splitter
  -> model trainer
  -> forecast evaluator
  -> forecast reports
```

Incorrect boundary:

```text
ML model
  -> direct buy/sell signal
  -> backtest engine
```

The first implementation must not emit production `Signal` objects and must not call the risk engine or execution modules.

---

## Naming Decision

Use module name:

```text
forecast
```

Preferred over:

```text
ml
```

Reason:

- The business purpose is forecasting and forecast evaluation.
- Ridge Regression and Random Forest are implementation choices.
- Future forecasting methods should fit the same module without renaming.

Suggested structure:

```text
src/forecast/
  mod.rs
  config.rs
  features.rs
  labels.rs
  dataset.rs
  split.rs
  metrics.rs
  evaluation.rs
  reports.rs
  runner.rs
  models/
    mod.rs
    ridge.rs
    random_forest.rs
```

---

## Non-Negotiable Boundaries

The forecast module may depend on:

```text
core::Candle
core::Timeframe
market::OhlcvLoader
market::CandleStore
market::MarketRegime
indicators::IndicatorEngine
risk::CostModelConfig only for cost-adjusted labels/metrics
```

The forecast module must not depend on:

```text
strategy::BasicSampleStrategy
strategy::registry
backtest::BacktestEngine
execution::*
journal::*
advisor::*
live/paper trading code
```

The backtest engine must not depend on:

```text
forecast::*
```

The strategy layer must not depend on:

```text
forecast::*
```

until a future explicit integration phase.

---

## Command Design

Add a new command:

```bash
cargo run -- forecast --config config/forecast.toml
```

Keep existing command unchanged:

```bash
cargo run -- research --config config/research.toml
```

The `forecast` command should:

1. load forecast config,
2. load historical OHLCV,
3. build indicator-derived features,
4. build forward-looking labels,
5. build walk-forward windows,
6. train Ridge and Random Forest models,
7. evaluate forecast quality,
8. write forecast reports,
9. exit without producing trading orders or production signals.

---

## Config Design

Create:

```text
config/forecast.toml
```

Initial shape:

```toml
[mode]
run_mode = "forecast"
dry_run = true

[pairs]
symbols = ["BTCUSDT"]
source_timeframe = "1m"
entry_timeframe = "1m"
forecast_horizon = "15m"

[data]
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

[features]
enabled = [
  "return_1m",
  "return_5m",
  "return_15m",
  "atr_bps",
  "volume_ratio",
  "vwap_distance_bps",
  "ema_8_21_spread_bps",
  "range_position",
  "hour_of_day",
  "day_of_week",
]

[label]
target = "future_return_bps"
horizon_bars = 15
cost_adjusted = true

[cost]
taker_fee_bps = 4.0
slippage_bps = 2.0
spread_bps = 1.0
market_impact_bps = 1.0
stop_slippage_bps = 5.0

[models]
enabled = ["ridge", "random_forest"]

[models.ridge]
alpha = 1.0
standardize = true

[models.random_forest]
trees = 100
max_depth = 8
min_samples_leaf = 50
feature_subsample_ratio = 0.7

[walk_forward]
train_months = 12
test_months = 3
step_months = 3
embargo_bars = 15

[reports]
reports_dir = "reports/forecast/btcusdt_1m_h15"
```

Validation rules:

- `run_mode` must be `forecast`.
- `dry_run` must not enable paper/live.
- `symbols` must not be empty.
- `source_timeframe` must be `1m` for now.
- `entry_timeframe` must be supported by `Timeframe`.
- `forecast_horizon` must be supported or convertible to bars.
- `horizon_bars > 0`.
- `enabled` features must be known.
- `label.target` must be one of supported labels.
- `cost` bps values must be finite and non-negative.
- `walk_forward.train_months > 0`.
- `walk_forward.test_months > 0`.
- `walk_forward.step_months > 0`.
- `walk_forward.embargo_bars >= horizon_bars` is preferred.
- `reports_dir` must not be empty.

---

## Feature Engineering Roadmap

Features must be computed using only data available at or before the current candle timestamp.

No feature may use future candles.

Initial feature set:

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

### Feature definitions

#### `return_1m`

```text
(close[t] - close[t-1]) / close[t-1] * 10_000
```

#### `return_5m`

```text
(close[t] - close[t-5]) / close[t-5] * 10_000
```

#### `return_15m`

```text
(close[t] - close[t-15]) / close[t-15] * 10_000
```

#### `atr_bps`

```text
ATR_14 / close[t] * 10_000
```

Use existing indicator snapshot if available.

#### `volume_ratio`

```text
volume[t] / rolling_avg_volume[t]
```

The rolling average must not include future candles.

#### `vwap_distance_bps`

```text
(close[t] - vwap[t]) / close[t] * 10_000
```

#### `ema_8_21_spread_bps`

```text
(ema_8[t] - ema_21[t]) / close[t] * 10_000
```

#### `range_position`

```text
(close[t] - low[t]) / (high[t] - low[t])
```

If `high == low`, return neutral value or skip row.

#### `hour_of_day`

Extracted from timestamp.

Encode as numeric value initially. Later can use sine/cosine encoding.

#### `day_of_week`

Extracted from timestamp.

Encode as numeric value initially. Later can use one-hot or cyclical encoding.

---

## Label Engineering Roadmap

Initial supported labels:

```text
future_return_bps
future_return_after_cost_bps
future_direction_after_cost
hit_tp_before_sl
mfe_bps
mae_bps
```

Start with:

```text
future_return_bps
future_return_after_cost_bps
```

### `future_return_bps`

For horizon `H` bars:

```text
(close[t + H] - close[t]) / close[t] * 10_000
```

### `future_return_after_cost_bps`

```text
future_return_bps - estimated_round_trip_cost_bps
```

Use configured cost model assumptions:

```text
taker_fee_bps * 2
+ slippage_bps * 2
+ spread_bps
+ market_impact_bps
```

Do not include stop slippage for plain forward-return label unless the label is explicitly stop-loss based.

### `future_direction_after_cost`

Classification target:

```text
1 if future_return_after_cost_bps > 0
0 otherwise
```

This is useful for Random Forest classification later.

### `hit_tp_before_sl`

Future optional label. Requires simulated path over horizon and deterministic conservative rule if both TP and SL are touched.

Not required in first implementation.

### `mfe_bps` and `mae_bps`

Future optional labels:

```text
MFE = max favorable excursion over horizon
MAE = max adverse excursion over horizon
```

Not required in first implementation.

---

## Dataset Design

Dataset row should contain:

```text
symbol
timestamp
close
feature values
label value(s)
metadata
```

Suggested Rust structs:

```rust
pub struct ForecastDataset {
    pub symbol: String,
    pub feature_names: Vec<String>,
    pub rows: Vec<ForecastRow>,
}

pub struct ForecastRow {
    pub timestamp: i64,
    pub close: f64,
    pub features: Vec<f64>,
    pub future_return_bps: f64,
    pub future_return_after_cost_bps: f64,
}
```

Rows must be skipped when:

- required feature is missing,
- feature is non-finite,
- label horizon exceeds dataset end,
- close price is invalid,
- label is non-finite.

Dataset report must include:

```text
raw_candles
candidate_rows
usable_rows
skipped_missing_feature
skipped_invalid_feature
skipped_missing_label
skipped_invalid_label
feature_count
label_target
horizon_bars
```

---

## Walk-Forward Design

Do not use random split.

Use chronological walk-forward windows.

Initial policy:

```text
train_months = 12
test_months = 3
step_months = 3
embargo_bars = horizon_bars
```

Window structure:

```text
train_start
train_end
embargo_start
embargo_end
test_start
test_end
```

The embargo area prevents label overlap leakage between train and test.

Example:

```text
train: 2020-01-01 to 2020-12-31
embargo: next H bars
test: 2021-01-01 to 2021-03-31
```

Reports must include each window:

```text
window_id
train_start
train_end
test_start
test_end
train_rows
test_rows
model
metrics
```

---

## Model 1 — Ridge Regression

Ridge Regression is the baseline model.

Purpose:

- determine whether features contain linear predictive signal,
- create a simple benchmark,
- expose feature/label sanity problems early,
- provide a low-overfit baseline.

Output target:

```text
future_return_after_cost_bps
```

Required behavior:

- standardize features using train-only mean/std,
- apply same transform to test,
- train Ridge only on training rows,
- predict test rows,
- never train on future rows,
- never standardize using test rows.

Minimum metrics:

```text
MAE
RMSE
correlation(predicted, actual)
directional_accuracy
avg_predicted_bps
avg_actual_bps
prediction_bucket_summary
```

Ridge report files:

```text
ridge_summary.json
ridge_prediction_buckets.csv
ridge_walk_forward.csv
```

---

## Model 2 — Random Forest

Random Forest is the nonlinear model.

Purpose:

- evaluate nonlinear interactions,
- capture threshold behavior,
- compare against Ridge baseline,
- produce feature importance for research only.

Initial target options:

### Option A — Regression

Target:

```text
future_return_after_cost_bps
```

Metrics:

```text
MAE
RMSE
correlation
directional_accuracy
prediction_buckets
feature_importance
```

### Option B — Classification

Target:

```text
future_direction_after_cost
```

Metrics:

```text
accuracy
precision
recall
AUC if available
Brier score if probability output is available
probability_bucket_summary
feature_importance
```

Recommended initial implementation:

- start with Random Forest regression if simpler in the chosen Rust implementation,
- add classification later if probability output is clean and reliable.

Random Forest report files:

```text
random_forest_summary.json
random_forest_prediction_buckets.csv
random_forest_walk_forward.csv
random_forest_feature_importance.csv
```

---

## Metric Tiers

### Tier 1 — required from first model implementation

```text
sample_count
train_rows
test_rows
MAE
RMSE
correlation
directional_accuracy
avg_predicted_bps
avg_actual_bps
avg_actual_after_cost_bps
prediction_bucket_summary
```

### Tier 2 — required before any strategy integration

```text
walk_forward_summary
prediction_decile_report
hit_rate_by_bucket
avg_return_by_bucket
feature_importance
coverage
turnover_estimate
cost_adjusted_edge
period_stability
```

### Tier 3 — later advanced metrics

```text
calibration_curve
Brier score
AUC
precision
recall
regime attribution
monthly breakdown
stability score
purged validation diagnostics
```

---

## Prediction Bucket / Decile Analysis

This is one of the most important parts.

For regression predictions, sort test predictions and split into buckets.

Initial bucket count:

```text
10 deciles
```

Each bucket report should include:

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

Core question:

> Do higher prediction buckets produce higher realized returns after cost?

If prediction buckets are not monotonic or at least directionally useful, the model is probably not useful as a trading scorer.

---

## Model Comparison

Create a final comparison report:

```text
model_comparison.json
```

Fields:

```text
symbol
label_target
horizon_bars
feature_set
models_compared
best_model_by_rmse
best_model_by_correlation
best_model_by_top_decile_return
best_model_by_stability
ridge_metrics
random_forest_metrics
recommendation
```

Recommendation must be conservative:

- `no_predictive_signal_detected`,
- `weak_signal_needs_more_validation`,
- `candidate_for_backtest_filter_phase`,
- `reject_due_to_instability`,
- `reject_due_to_cost_adjusted_decay`.

Do not claim profitability.

---

## Report Output Layout

Create reports under:

```text
reports/forecast/<run_name>/
```

Initial output files:

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

Generated reports must remain ignored by git through `reports/` in `.gitignore`.

---

## CLI Output Design

Forecast command output should follow the same clean logging style as research mode.

Example:

```text
Northflow Forecast Research
===========================
Run Plan
--------
Mode              : forecast
Symbols           : BTCUSDT
Source TF         : 1m
Entry TF          : 1m
Forecast Horizon  : 15m
Models            : ridge, random_forest
Reports Dir       : reports/forecast/btcusdt_1m_h15

Dataset
-------
Raw candles        : 3,156,480
Usable rows        : 3,142,000
Feature count      : 10
Label              : future_return_after_cost_bps
Horizon bars       : 15

Walk Forward
------------
Windows            : 18
Train months       : 12
Test months        : 3
Embargo bars       : 15

Model: Ridge
------------
MAE                : 8.42 bps
RMSE               : 14.77 bps
Correlation        : 0.0312
Directional acc    : 51.02%
Top decile avg     : 2.14 bps

Model: Random Forest
--------------------
MAE                : 8.11 bps
RMSE               : 14.22 bps
Correlation        : 0.0441
Directional acc    : 51.66%
Top decile avg     : 3.82 bps

Recommendation
--------------
weak_signal_needs_more_validation
```

---

## Phase Roadmap

## Phase F0 — Forecast module skeleton

Goal:

- create module boundaries,
- add config file,
- add CLI command,
- no model training yet.

Deliverables:

```text
src/forecast/mod.rs
src/forecast/config.rs
src/forecast/runner.rs
config/forecast.toml
docs/forecast-ml-roadmap.md
```

Acceptance:

- `cargo run -- forecast --config config/forecast.toml` loads config and prints run plan.
- no trading/backtest logic changes.

---

## Phase F1 — Dataset builder

Goal:

- load historical OHLCV,
- compute selected features,
- compute forward labels,
- write dataset summary.

Deliverables:

```text
src/forecast/features.rs
src/forecast/labels.rs
src/forecast/dataset.rs
src/forecast/reports.rs
```

Reports:

```text
dataset_summary.json
feature_summary.csv
label_summary.json
```

Acceptance:

- all features use only past/current data,
- all labels use future data only in target construction,
- skipped rows are counted,
- no model training yet.

---

## Phase F2 — Walk-forward splitter

Goal:

- create chronological walk-forward windows,
- apply embargo,
- prevent train/test label overlap leakage.

Deliverables:

```text
src/forecast/split.rs
walk_forward_windows.csv
```

Acceptance:

- no random split,
- train rows always occur before test rows,
- embargo is applied,
- tests cover window boundaries.

---

## Phase F3 — Ridge Regression baseline

Goal:

- train Ridge Regression per walk-forward window,
- evaluate on test rows,
- write summary and bucket reports.

Deliverables:

```text
src/forecast/models/ridge.rs
src/forecast/evaluation.rs
ridge_summary.json
ridge_prediction_buckets.csv
ridge_walk_forward.csv
```

Acceptance:

- standardization uses train rows only,
- predictions are out-of-sample,
- metrics are computed on test rows only,
- bucket analysis exists.

---

## Phase F4 — Random Forest model

Goal:

- train Random Forest per walk-forward window,
- evaluate nonlinear forecast performance,
- write feature importance and bucket reports.

Deliverables:

```text
src/forecast/models/random_forest.rs
random_forest_summary.json
random_forest_prediction_buckets.csv
random_forest_walk_forward.csv
random_forest_feature_importance.csv
```

Acceptance:

- model trains only on train rows,
- test metrics are out-of-sample,
- feature importance is research-only,
- RF compared against Ridge baseline.

---

## Phase F5 — Model comparison and recommendation

Goal:

- compare Ridge vs Random Forest,
- evaluate ranking power and stability,
- generate conservative recommendation.

Deliverables:

```text
model_comparison.json
forecast_run_manifest.json
```

Acceptance:

- no profitability claim,
- recommendation is conservative,
- report highlights instability and cost-adjusted decay when present.

---

## Phase F6 — Optional future backtest filter integration

Goal:

- integrate forecast model as scorer/filter only after ML evidence exists.

Not part of initial implementation.

Future shape:

```text
Candidate signal
  -> ForecastScorer
  -> RiskEngine
  -> BacktestEngine
```

This phase must not start until F0-F5 produce meaningful out-of-sample forecast evidence.

---

## Testing Strategy

Required tests:

```text
config parsing
invalid config rejection
feature computation no-lookahead
label computation exact horizon
invalid row skipping
walk-forward ordering
embargo boundary
ridge train/test separation
prediction bucket generation
report writer smoke tests
```

Tests must not require the full 2020-2025 dataset.

Use small synthetic candles for unit tests.

---

## Dependency Strategy

Keep dependencies minimal.

Ridge Regression can be implemented directly if practical:

- standardize features,
- solve regularized linear regression,
- predict.

If using a Rust ML crate, evaluate:

- compile time,
- transitive dependencies,
- license,
- deterministic behavior,
- serialization/reporting needs,
- support for Ridge and Random Forest.

Do not add Python dependency.

Do not require external services.

Do not introduce a model artifact format until needed.

---

## Success Definition

The forecast ML roadmap is successful when Northflow can run:

```bash
cargo run -- forecast --config config/forecast.toml
```

and produce deterministic forecast research reports that answer:

1. how many usable rows exist,
2. which features were used,
3. what label was predicted,
4. how walk-forward windows were constructed,
5. how Ridge performed out-of-sample,
6. how Random Forest performed out-of-sample,
7. whether higher prediction buckets map to higher realized returns,
8. whether the signal survives cost assumptions,
9. whether the result is stable enough to justify a future backtest-filter phase.

The system must still preserve:

- one active production strategy,
- strategy-agnostic backtest engine,
- no live/paper trading,
- no exchange calls,
- no hidden generated report files in git.
