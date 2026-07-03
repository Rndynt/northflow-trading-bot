# Forecast ML Completion & Correctness Patch

Read first:

- `docs/forecast-ml-roadmap.md`
- `prompts/forecast-ml-module-implementation.md`

The current forecast module is a good foundation but is not complete. This patch must close the review gaps before the module is treated as roadmap-complete.

## Mandatory scope

Keep the forecast module independent. Do not change existing research backtest behavior, strategy behavior, risk sizing, fill simulation, accounting, indicators, paper mode, or live mode. Do not add exchange/API/network calls. Do not require Python. Do not commit generated files under `reports/`.

## Current status from review

```text
F0 forecast skeleton                 OK
F1 dataset builder                   mostly OK
F2 walk-forward splitter             OK MVP; document fixed 30-day month limitation
F3 Ridge baseline                    partial; current implementation is not true multivariate Ridge
F4 Random Forest                     partial; feature_subsample_ratio and feature importance incomplete
F5 model comparison/recommendation   missing
Implementation report                missing
```

## Required fixes

### 1. Add implementation report

Create `docs/forecast-ml-implementation-report.md` with: roadmap reference, completed phases, incomplete phases, module boundaries, config schema, implemented features/labels, walk-forward policy, Ridge details, Random Forest details, metrics, prediction bucket behavior, report files, dependency decisions, command results, limitations, and confirmation that existing strategy/backtest/risk/fill/accounting were not changed.

### 2. Fix target handling

`label.target` and `cost_adjusted` must control the effective model target. Add typed target selection. Supported regression targets:

```text
future_return_bps
future_return_after_cost_bps
```

For now, reject `future_direction_after_cost` unless a real classification evaluator is implemented. Reports must show both configured target and effective target.

### 3. Fix metrics consistency

Predictions must include or derive an effective target value. MAE, RMSE, correlation, directional accuracy, and prediction buckets must compare `predicted_bps` against the same target used for model training. Reports may still include both raw return and after-cost return.

### 4. Correct Ridge

The current Ridge implementation is per-feature shrinkage, not true multivariate Ridge. Implement true deterministic Ridge using normal equations or a small local solver:

```text
(X^T X + alpha I) w = X^T y
```

Use a non-regularized intercept, train-only standardization, and test-only evaluation. If true Ridge is not implemented, rename the current model honestly and document Ridge as pending. Preferred outcome: real Ridge.

### 5. Wire Random Forest ratio

`models.random_forest.feature_subsample_ratio` must be validated and used. Valid range:

```text
0.0 < ratio <= 1.0
```

Changing the ratio should change candidate feature subset behavior.

### 6. Add Random Forest feature importance report

Write `random_forest_feature_importance.csv`. Preferred method: split-count importance.

Columns:

```text
feature,split_count,importance,method
```

If no split exists, write all features with zero importance and a clear method/note.

### 7. Add model comparison

Write `model_comparison.json`. It must compare Ridge and Random Forest when both are enabled. Include configured/effective target, horizon, models compared, best model by RMSE, best model by correlation, best model by top-decile return, model metrics, prediction counts, window counts, and conservative recommendation.

Allowed recommendation values:

```text
no_predictive_signal_detected
weak_signal_needs_more_validation
candidate_for_backtest_filter_phase
reject_due_to_instability
reject_due_to_cost_adjusted_decay
```

Do not claim profitability.

### 8. Add run manifest

Write `forecast_run_manifest.json` with run mode, symbols, timeframes, horizon, configured/effective target, cost flag, features, models, walk-forward settings, fixed 30-day month policy, reports written, and limitations.

### 9. Fix default config usability

Update `config/forecast.toml` so the default setup can produce walk-forward windows when the standard BTCUSDT yearly files exist. Preferred: list 2020–2025 files like the research config. If files are missing, show clean missing-data output.

### 10. Improve missing data handling

Forecast runner must list expected missing files one per line and return an actionable error. Do not show a raw panic.

### 11. Required report behavior

For a successful run with both models enabled, these files must be written:

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

If no walk-forward windows exist, write dataset reports, windows file, manifest, and comparison with zero predictions. Do not fake successful model reports.

### 12. Boundary audit

Search and document whether forecast is coupled to forbidden layers. Expected: forecast may be used by `main.rs`, `lib.rs`, and `src/forecast/*`; the backtest engine and strategy layer must not depend on forecast.

## Required tests

Add synthetic-data tests for: config validation, invalid RF ratio, unsupported classification target, effective target selection, feature no-lookahead, exact-horizon labels, cost-adjusted labels, metric target consistency, prediction buckets, invalid row skipping, walk-forward ordering, embargo boundary, true Ridge behavior, train-only standardization, RF ratio wiring, deterministic RF predictions, RF feature importance report, model comparison, manifest writer, and report writer.

## Final validation

Run and document:

```bash
cargo fmt --check
cargo test
cargo run -- forecast --config config/forecast.toml
```

Also run research command if historical data exists:

```bash
cargo run -- research --config config/research.toml
```

## Success definition

This patch is complete only when the implementation report exists, target behavior is correct, metrics use the effective target, Ridge is honest/true, Random Forest uses its ratio config, feature importance report exists, model comparison exists, manifest exists, missing-data handling is clean, required reports are deterministic, boundaries remain clean, and tests pass.
