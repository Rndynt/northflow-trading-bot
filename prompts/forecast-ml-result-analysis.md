# Forecast ML Result Analysis & Research Decision Prompt

## Phase

```text
F6 — Forecast Result Analysis & Research Decision
```

## References

Read these first:

```text
docs/forecast-ml-roadmap.md
prompts/forecast-ml-module-implementation.md
prompts/forecast-ml-completion-correctness-patch.md
prompts/forecast-ml-final-evaluation-quality-patch.md
docs/forecast-ml-implementation-report.md
config/forecast.toml
```

The forecast module is now capable of producing deterministic model reports for Ridge and Random Forest. This phase is not about adding new models, new features, or strategy integration.

This phase is about reading the generated forecast reports and producing a clear research decision.

---

## Core Objective

Analyze the actual forecast research outputs and create:

```text
docs/forecast-ml-result-analysis.md
```

The document must answer one question with evidence:

> Does the current forecast feature/label/model setup show enough cost-adjusted predictive signal to justify a future ForecastScorer/backtest-filter phase, or should the current setup be rejected/iterated first?

---

## Non-Negotiable Boundaries

Do not modify production strategy behavior.
Do not add a new ML trading strategy.
Do not integrate ForecastScorer.
Do not connect forecast output to `BacktestEngine`.
Do not make `strategy` depend on `forecast`.
Do not change risk sizing, fill simulation, accounting, indicators, or existing research backtest behavior.
Do not enable paper mode or live mode.
Do not add exchange/API/network/LLM calls.
Do not commit generated files under `reports/`.
Do not claim profitability.

This task is analysis-only plus documentation.

Allowed changes:

```text
- create/update docs/forecast-ml-result-analysis.md
- optionally create small non-runtime helper under scripts/ or tests only if needed to parse local reports
- optionally update docs/forecast-ml-implementation-report.md with a pointer to the result analysis doc
```

Generated report files must remain untracked.

---

## Required Input Reports

Use the report directory from:

```toml
[reports]
reports_dir = "reports/forecast/btcusdt_1m_h15"
```

Expected files:

```text
reports/forecast/btcusdt_1m_h15/dataset_summary.json
reports/forecast/btcusdt_1m_h15/feature_summary.csv
reports/forecast/btcusdt_1m_h15/label_summary.json
reports/forecast/btcusdt_1m_h15/walk_forward_windows.csv
reports/forecast/btcusdt_1m_h15/ridge_summary.json
reports/forecast/btcusdt_1m_h15/ridge_prediction_buckets.csv
reports/forecast/btcusdt_1m_h15/ridge_walk_forward.csv
reports/forecast/btcusdt_1m_h15/random_forest_summary.json
reports/forecast/btcusdt_1m_h15/random_forest_prediction_buckets.csv
reports/forecast/btcusdt_1m_h15/random_forest_walk_forward.csv
reports/forecast/btcusdt_1m_h15/random_forest_feature_importance.csv
reports/forecast/btcusdt_1m_h15/model_comparison.json
reports/forecast/btcusdt_1m_h15/forecast_run_manifest.json
```

If the reports do not exist locally, run:

```bash
cargo run --release -- forecast --config config/forecast.toml
```

If the run is too slow in the current environment, do not fake results. Document that the reports are missing or the run timed out, and explain exactly what could and could not be analyzed.

---

## Required Analysis Sections

Create `docs/forecast-ml-result-analysis.md` with the following sections.

### 1. Executive Decision

Start with one of these decisions:

```text
reject_current_setup
iterate_feature_label_horizon
candidate_for_forecast_scorer_backtest_filter
insufficient_data_or_reports
```

Use conservative language.

Do not claim profitability.

The decision must be evidence-based and must reference the generated reports used.

### 2. Run Context

Summarize from `forecast_run_manifest.json`:

```text
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
walk_forward settings
reports_written
limitations
```

### 3. Dataset Summary

Summarize from `dataset_summary.json` and `label_summary.json`:

```text
input_rows
output_rows
feature_count
skipped_missing_feature
skipped_invalid_feature
skipped_label_horizon
skipped_invalid_close
skipped_invalid_label
avg_future_return_bps
avg_future_return_after_cost_bps
```

Include a short interpretation:

```text
- Is usable row count sufficient?
- Are skipped rows expected?
- Is the average after-cost label already negative?
```

### 4. Walk-Forward Summary

Summarize from `walk_forward_windows.csv`:

```text
window_count
first_train_start
last_test_end
train_rows per window
test_rows per window
embargo_bars
```

Interpret whether the walk-forward design is adequate for this first analysis.

### 5. Model Summary — Ridge

Use `ridge_summary.json` and `ridge_prediction_buckets.csv`.

Include:

```text
prediction_count
MAE
RMSE
correlation
directional_accuracy
avg_predicted_bps
avg_actual_bps
avg_actual_after_cost_bps
top_decile_avg_effective_actual_bps
top_decile_hit_rate_effective_target
bucket monotonicity / ranking quality
```

Interpret:

```text
- Does Ridge show positive ranking power?
- Is top decile cost-adjusted return positive?
- Are buckets monotonic or noisy?
```

### 6. Model Summary — Random Forest

Use `random_forest_summary.json`, `random_forest_prediction_buckets.csv`, and `random_forest_feature_importance.csv`.

Include:

```text
prediction_count
MAE
RMSE
correlation
directional_accuracy
avg_predicted_bps
avg_actual_bps
avg_actual_after_cost_bps
top_decile_avg_effective_actual_bps
top_decile_hit_rate_effective_target
bucket monotonicity / ranking quality
top features by split-count importance
```

Interpret:

```text
- Does Random Forest improve over Ridge?
- Is improvement meaningful or marginal?
- Which features dominate split usage?
- Does split usage look plausible or suspicious?
```

### 7. Model Comparison

Use `model_comparison.json`.

Include:

```text
best_model_by_rmse
best_model_by_correlation
best_model_by_top_decile_return
recommendation
```

Then independently interpret whether the recommendation is justified by the model summaries and buckets.

If the report says `reject_due_to_cost_adjusted_decay`, explain what decayed:

```text
top decile effective after-cost return <= 0
or ranking exists but does not survive costs
```

### 8. Prediction Bucket / Decile Analysis

This is the most important section.

For both Ridge and Random Forest, analyze the 10 buckets.

At minimum include:

```text
bottom bucket avg_effective_actual_bps
top bucket avg_effective_actual_bps
top minus bottom spread
count of positive-effective buckets
whether higher predicted buckets generally correspond to better realized effective return
```

Define a simple monotonicity score:

```text
monotonic_steps = count of adjacent bucket pairs where avg_effective_actual_bps increases
max_steps = bucket_count - 1
monotonicity_ratio = monotonic_steps / max_steps
```

Interpret:

```text
- strong ranking power
- weak/noisy ranking power
- inverted ranking
- cost-adjusted decay
```

### 9. Feature Importance Interpretation

Use `random_forest_feature_importance.csv`.

List top 10 features by split-count importance.

For each top feature, give a short interpretation:

```text
return_15m: recent momentum/mean-reversion proxy
atr_bps: volatility regime proxy
volume_ratio: liquidity/participation proxy
vwap_distance_bps: distance from session/rolling value proxy
ema_8_21_spread_bps: trend pressure proxy
hour_of_day/day_of_week: session behavior proxy
```

Do not overclaim causality.

Split-count importance means the feature was used often by the RF splits, not necessarily that it causes profitable trades.

### 10. Research Decision

Choose exactly one final decision:

```text
reject_current_setup
iterate_feature_label_horizon
candidate_for_forecast_scorer_backtest_filter
insufficient_data_or_reports
```

Decision rules:

```text
if required reports are missing:
  insufficient_data_or_reports

else if top decile avg_effective_actual_bps <= 0 for both models:
  reject_current_setup or iterate_feature_label_horizon

else if top decile is positive but buckets are not monotonic/noisy:
  iterate_feature_label_horizon

else if top decile is positive, ranking is reasonably monotonic, correlation > 0, and result survives cost:
  candidate_for_forecast_scorer_backtest_filter
```

Be conservative.

### 11. Next Research Iteration

If decision is not `candidate_for_forecast_scorer_backtest_filter`, propose the next research iteration.

Preferred next iterations, in order:

```text
1. horizon comparison: 15m vs 30m vs 1h vs 4h
2. regime attribution: trend/range/high-vol/low-vol
3. alternative labels: hit_tp_before_sl, mfe_bps, mae_bps, volatility_adjusted_return
4. cost sensitivity: lower/higher cost assumptions
5. session split: Asia/London/US time segments
```

Do not recommend adding more complex models until horizon/label/regime analysis is done.

---

## Required Commands

Run:

```bash
cargo fmt --check
cargo test
```

If reports are missing or stale, run:

```bash
cargo run --release -- forecast --config config/forecast.toml
```

If full forecast is too slow, do not fake it. Document the limitation.

After writing the doc, verify:

```bash
git status --short
```

Ensure generated `reports/` files are not staged or committed.

---

## Required Quality Checks

Search generated analysis doc for banned or misleading language:

```text
guaranteed profit
profitable strategy
production-ready
live-ready
edge confirmed
```

Allowed language:

```text
predictive signal candidate
weak signal
cost-adjusted decay
requires further validation
candidate for future backtest-filter phase
not a profitability claim
```

---

## Acceptance Criteria

The task is complete only when:

```text
- docs/forecast-ml-result-analysis.md exists
- it is based on actual forecast reports, or clearly states reports are unavailable
- it includes dataset, walk-forward, Ridge, Random Forest, comparison, buckets, and feature importance analysis
- it makes one explicit research decision
- it does not claim profitability
- it does not modify strategy/backtest/risk/fill/accounting behavior
- it does not commit reports/
- cargo fmt --check passes
- cargo test passes
```

---

## Out Of Scope

Do not implement:

```text
ForecastScorer
ML-backed strategy
backtest integration
new ML model
hyperparameter optimization
new indicators
paper trading
live trading
exchange integration
Python analysis pipeline
UI/dashboard
```

This phase is analysis and research decision only.
