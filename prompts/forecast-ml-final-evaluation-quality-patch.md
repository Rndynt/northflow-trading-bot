# Forecast ML Final Evaluation Quality Patch

## References

Read these first:

- `docs/forecast-ml-roadmap.md`
- `prompts/forecast-ml-module-implementation.md`
- `prompts/forecast-ml-completion-correctness-patch.md`
- `docs/forecast-ml-implementation-report.md`

This prompt is a final quality patch for the independent `forecast` ML research module.

The previous completion patch fixed major correctness gaps, but review found three remaining quality issues:

1. `model_comparison.json` exists but does not actually compare model metrics.
2. `random_forest_feature_importance.csv` exists but is still a zero placeholder.
3. `docs/forecast-ml-implementation-report.md` does not contain exact command outputs; it only points to the final agent response.

This patch must fix those issues completely.

---

## Non-negotiable boundaries

Do not modify production strategy behavior.
Do not add a production ML trading strategy.
Do not make `BacktestEngine` depend on `forecast`.
Do not make `strategy` depend on `forecast`.
Do not change risk sizing, fill simulation, accounting, existing indicator formulas, or existing research backtest reports.
Do not enable paper mode or live mode.
Do not add exchange, API, network, or LLM calls.
Do not require Python.
Do not commit generated files under `reports/`.
Do not claim profitability.

Keep this patch isolated to the forecast module, forecast reports, config/report docs, and tests.

---

## Required outcome

After this patch, a successful forecast run with both Ridge and Random Forest enabled must write meaningful, deterministic forecast research outputs:

```text
model_comparison.json
random_forest_feature_importance.csv
forecast_run_manifest.json
ridge_summary.json
random_forest_summary.json
ridge_prediction_buckets.csv
random_forest_prediction_buckets.csv
```

The comparison report must be computed from real model summaries and prediction buckets, not hardcoded placeholders.

The Random Forest feature-importance report must reflect actual split usage, not a zero-placeholder unless no tree split occurred.

The implementation report must contain exact command results.

---

# Item 1 — Real Model Comparison

## Current issue

`model_comparison.json` is currently too shallow. It writes null values for:

```text
best_model_by_rmse
best_model_by_correlation
best_model_by_top_decile_return
```

and hardcodes:

```text
recommendation = weak_signal_needs_more_validation
```

This is not acceptable as final model comparison.

## Requirement

Implement real model comparison based on in-memory per-model evaluation results.

Add a structure similar to:

```rust
pub struct ModelEvaluationResult {
    pub model_name: String,
    pub metrics: RegressionMetrics,
    pub buckets: Vec<PredictionBucket>,
    pub prediction_count: usize,
    pub window_count: usize,
}
```

The runner must collect one `ModelEvaluationResult` per model.

Then generate `model_comparison.json` from those results.

## Required model comparison fields

`model_comparison.json` must include at least:

```json
{
  "configured_target": "future_return_bps",
  "effective_target": "future_return_after_cost_bps",
  "horizon_bars": 15,
  "models_compared": ["ridge", "random_forest"],
  "best_model_by_rmse": "ridge",
  "best_model_by_correlation": "random_forest",
  "best_model_by_top_decile_return": "random_forest",
  "recommendation": "weak_signal_needs_more_validation",
  "models": [
    {
      "model_name": "ridge",
      "prediction_count": 12345,
      "window_count": 4,
      "mae": 0.0,
      "rmse": 0.0,
      "correlation": 0.0,
      "directional_accuracy": 0.0,
      "avg_predicted_bps": 0.0,
      "avg_actual_bps": 0.0,
      "avg_actual_after_cost_bps": 0.0,
      "top_decile_avg_effective_actual_bps": 0.0,
      "top_decile_hit_rate_after_cost": 0.0
    }
  ]
}
```

Use actual values from metrics and buckets.

## Selection rules

Use deterministic rules:

```text
best_model_by_rmse = model with lowest RMSE among models with prediction_count > 0
best_model_by_correlation = model with highest correlation among models with prediction_count > 0
best_model_by_top_decile_return = model with highest top-decile average effective actual return
```

If no model has predictions, use `null` for all best-model fields.

## Recommendation rules

Allowed recommendation values only:

```text
no_predictive_signal_detected
weak_signal_needs_more_validation
candidate_for_backtest_filter_phase
reject_due_to_instability
reject_due_to_cost_adjusted_decay
```

Use conservative deterministic logic:

```text
if all prediction_count == 0:
  no_predictive_signal_detected

else if best top_decile_avg_effective_actual_bps <= 0:
  reject_due_to_cost_adjusted_decay

else if best correlation <= 0.0 and best directional_accuracy <= 0.50:
  no_predictive_signal_detected

else if best top_decile_avg_effective_actual_bps > 0 but correlation < 0.02:
  weak_signal_needs_more_validation

else:
  candidate_for_backtest_filter_phase
```

Do not claim profitability.
Do not imply the model is production-ready.
A `candidate_for_backtest_filter_phase` recommendation only means it may be tested later as a scorer/filter.

## Acceptance criteria

- `model_comparison.json` no longer uses hardcoded nulls when predictions exist.
- Recommendation is computed from metrics and bucket results.
- Empty/no-window case is handled cleanly.
- Tests cover empty predictions, best RMSE, best correlation, best top-decile return, and recommendation selection.

---

# Item 2 — Effective-Target Bucket Consistency

## Current issue

Metrics now use `effective_actual_bps`, but prediction buckets still expose raw and after-cost averages only. This can make model comparison ambiguous.

## Requirement

Extend `PredictionBucket` with:

```rust
pub avg_effective_actual_bps: f64
pub hit_rate_effective_target: f64
```

Keep existing raw/after-cost columns for research visibility.

Update bucket CSV headers to include:

```text
avg_effective_actual_bps
hit_rate_effective_target
```

Model comparison must use `avg_effective_actual_bps` from the top decile.

## Acceptance criteria

- Bucket metrics are internally consistent with the effective training target.
- Existing raw and after-cost columns remain available.
- Tests prove bucket average uses `effective_actual_bps`.

---

# Item 3 — Real Random Forest Split-Count Feature Importance

## Current issue

`random_forest_feature_importance.csv` currently writes zero placeholder rows. That is acceptable as a documented limitation, but not final quality.

## Requirement

Track split counts during Random Forest tree construction.

Implement a structure similar to:

```rust
pub struct RandomForestEvaluation {
    pub predictions: Vec<Prediction>,
    pub split_counts: Vec<usize>,
}
```

or return predictions plus feature importance from `random_forest::evaluate`.

During every accepted split, increment:

```rust
split_counts[feature] += 1
```

Aggregate across all trees.

Write:

```text
random_forest_feature_importance.csv
```

Columns:

```text
feature,split_count,importance,method
```

Where:

```text
importance = split_count / total_split_count
method = split_count
```

If there are no splits, still write all feature names with zero importance and method:

```text
split_count_no_splits
```

## Acceptance criteria

- All enabled feature names appear in `random_forest_feature_importance.csv`.
- When splits exist, split counts are greater than zero for selected features.
- Importance values sum to approximately 1.0 when total_split_count > 0.
- Tests cover both split and no-split cases.

---

# Item 4 — Report Writer Cleanup

## Requirement

Refactor report writing enough to avoid duplicated or conflicting Random Forest importance writes.

Currently `write_model` may write a placeholder and runner may call a separate zero-importance writer. After this patch:

- `write_model` should write only model summary, bucket CSV, and walk-forward CSV.
- Random Forest importance should be written by a dedicated function using actual feature names and split counts.
- `write_comparison_and_manifest` should receive computed comparison results, not infer everything from config alone.

## Acceptance criteria

- No duplicate writes with conflicting content.
- Report files have one responsible writer each.
- Tests cover writer smoke behavior.

---

# Item 5 — Manifest Improvements

## Requirement

Update `forecast_run_manifest.json` so it includes:

```text
run_mode
symbols
source_timeframe
entry_timeframe
forecast_horizon
horizon_bars
configured_target
effective_target
cost_adjusted
enabled_features
enabled_models
walk_forward.train_months
walk_forward.test_months
walk_forward.step_months
walk_forward.embargo_bars
walk_forward.month_model = fixed_30_day_months
reports_written
limitations
```

`reports_written` must list the actual report filenames written during the run.

Required limitations:

```text
source timeframe currently supports only 1m
walk-forward months use fixed 30-day approximation
classification target is not implemented
forecast module does not emit production trading signals
paper/live remain disabled
```

## Acceptance criteria

- Manifest is accurate.
- Manifest does not claim completed model evaluation when no windows/predictions exist.
- Tests cover manifest writer output containing required fields.

---

# Item 6 — Implementation Report Command Results

## Current issue

`docs/forecast-ml-implementation-report.md` currently says to see the final agent response for command output.

That is not acceptable.

## Requirement

Update the report to include exact command results from this patch.

Minimum required section:

```md
## Command results

- `cargo fmt --check`: passed / failed with reason
- `cargo test`: passed / failed with exact summary
- `cargo run -- forecast --config config/forecast.toml`: completed / missing data / timed out with exact observed behavior
- `cargo run -- research --config config/research.toml`: completed / skipped with reason / timed out with exact observed behavior
```

Be honest. If the full BTCUSDT dataset run times out, say that clearly. Do not claim full run completion unless it actually completed.

## Acceptance criteria

- Report no longer points to external/final response for command results.
- Exact command outcomes are documented in the repo.

---

# Item 7 — Missing Data Output Formatting

## Requirement

Improve missing-data output from forecast runner to match the research logging style.

Use this style:

```text
Symbol: BTCUSDT
---------------
Missing historical data.
Expected files:
  1. data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv
  2. data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv
How to fix:
  - configure [historical_files], or
  - place fallback CSV at data_dir/<SYMBOL>.csv
```

Do not show a raw loader error when missing files can be detected first.

## Acceptance criteria

- Missing files are numbered one per line.
- Error is actionable.
- No panic.

---

# Required tests

Add or update tests for:

```text
model comparison empty predictions
model comparison best RMSE
model comparison best correlation
model comparison best top-decile effective return
model comparison recommendation rules
prediction bucket effective target average
Random Forest split-count feature importance with splits
Random Forest feature importance no-split fallback
manifest contains required fields
report writer does not duplicate RF importance writes
missing-data formatter lists numbered files
implementation report command-results section exists
```

Use synthetic rows/candles only.
Do not require BTCUSDT historical data in tests.

---

# Final validation commands

Run:

```bash
cargo fmt --check
cargo test
cargo run -- forecast --config config/forecast.toml
```

Also run if data exists or document why it was skipped/timed out:

```bash
cargo run -- research --config config/research.toml
```

Update `docs/forecast-ml-implementation-report.md` with exact outcomes.

---

# Final search checks

Search the codebase for:

```text
best_model_by_rmse
best_model_by_correlation
best_model_by_top_decile_return
random_forest_feature_importance.csv
avg_effective_actual_bps
hit_rate_effective_target
See final agent response
split_count_no_splits_or_not_collected
```

Expected:

- best model fields are computed, not always null.
- RF feature importance uses real split counts when available.
- bucket CSV includes effective target columns.
- implementation report does not contain `See final agent response`.
- old placeholder method names are removed or only used for genuine no-split cases.

---

# Success definition

This patch is complete only when:

```text
- model_comparison.json is computed from real evaluation results
- best model fields are not hardcoded null when predictions exist
- recommendation is deterministic and conservative
- prediction buckets include effective-target columns
- Random Forest feature importance uses actual split counts
- manifest lists actual reports written
- implementation report contains exact command results
- missing-data output is clean and numbered
- tests pass
- no production strategy/backtest/risk/fill/accounting behavior changes
```

Commit and push the completed patch.
