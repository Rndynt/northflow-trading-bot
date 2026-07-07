# Forecast ML Horizon Comparison Prompt

## Phase

```text
F7 — Forecast Horizon Comparison
```

## References

Read these first:

```text
docs/forecast-ml-roadmap.md
prompts/forecast-ml-module-implementation.md
prompts/forecast-ml-completion-correctness-patch.md
prompts/forecast-ml-final-evaluation-quality-patch.md
prompts/forecast-ml-result-analysis.md
docs/forecast-ml-implementation-report.md
docs/forecast-ml-result-analysis.md
config/forecast.toml
```

The F6 result analysis concluded:

```text
research_decision = iterate_feature_label_horizon
```

Reason: current 15m horizon has weak but real ranking signal, but top-decile average effective cost-adjusted return is still negative for both Ridge and Random Forest. Therefore, do not integrate ForecastScorer/backtest filter yet. The next research step is horizon comparison.

This phase must compare whether the same forecast pipeline performs better at longer horizons where fixed round-trip cost is less dominant relative to expected movement.

---

## Core Objective

Run and compare forecast research across these horizons:

```text
15m
30m
1h
4h
```

Create:

```text
docs/forecast-ml-horizon-comparison.md
```

The document must answer:

> Does any horizon produce a cost-adjusted top-decile result strong enough to justify a future ForecastScorer/backtest-filter phase, or should the next iteration move to regime/label/cost analysis first?

---

## Non-Negotiable Boundaries

Do not modify production strategy behavior.
Do not add an ML-backed production strategy.
Do not integrate ForecastScorer.
Do not connect forecast output to `BacktestEngine`.
Do not make `strategy` depend on `forecast`.
Do not change risk sizing, fill simulation, accounting, indicators, or existing research backtest behavior.
Do not enable paper mode or live mode.
Do not add exchange/API/network/LLM calls.
Do not require Python.
Do not commit generated files under `reports/`.
Do not claim profitability.

This task is forecast research and documentation only.

Allowed changes:

```text
- add horizon-specific forecast config files under config/forecast/
- add or update docs/forecast-ml-horizon-comparison.md
- optionally add small Rust helper/report code if needed to support multiple horizon runs cleanly
- optionally update docs/forecast-ml-result-analysis.md with a link to horizon comparison
- optionally add tests for config/report helper logic
```

Generated report outputs must stay ignored and untracked.

---

## Required Horizon Configs

Create horizon-specific config files:

```text
config/forecast/btcusdt_1m_h15.toml
config/forecast/btcusdt_1m_h30.toml
config/forecast/btcusdt_1m_h1h.toml
config/forecast/btcusdt_1m_h4h.toml
```

Each config must keep the same baseline setup except horizon and reports directory.

Base fields must remain consistent:

```toml
[mode]
run_mode = "forecast"
dry_run = true

[pairs]
symbols = ["BTCUSDT"]
source_timeframe = "1m"
entry_timeframe = "1m"

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
embargo_bars = <horizon_bars>
```

Horizon-specific values:

```text
15m:
  forecast_horizon = "15m"
  horizon_bars = 15
  embargo_bars = 15
  reports_dir = "reports/forecast/btcusdt_1m_h15"

30m:
  forecast_horizon = "30m"
  horizon_bars = 30
  embargo_bars = 30
  reports_dir = "reports/forecast/btcusdt_1m_h30"

1h:
  forecast_horizon = "1h"
  horizon_bars = 60
  embargo_bars = 60
  reports_dir = "reports/forecast/btcusdt_1m_h1h"

4h:
  forecast_horizon = "4h"
  horizon_bars = 240
  embargo_bars = 240
  reports_dir = "reports/forecast/btcusdt_1m_h4h"
```

Keep root `config/forecast.toml` unchanged unless you intentionally want it to remain the default 15m preset. If updating it, document why.

---

## Required Commands

Run each config:

```bash
cargo run --release -- forecast --config config/forecast/btcusdt_1m_h15.toml
cargo run --release -- forecast --config config/forecast/btcusdt_1m_h30.toml
cargo run --release -- forecast --config config/forecast/btcusdt_1m_h1h.toml
cargo run --release -- forecast --config config/forecast/btcusdt_1m_h4h.toml
```

If full Random Forest runs are too slow, do not fake results. Use one of these approaches:

1. Let the runs complete in release mode and document wall-clock time.
2. If the environment cannot complete them, create a temporary local-only smoke config with fewer trees, but do not use smoke results as final horizon decision.
3. Document exactly which horizons completed and which did not.

Final decision must be based only on completed full-config runs.

Also run:

```bash
cargo fmt --check
cargo test
```

---

## Required Reports Per Horizon

Each horizon run must produce, under its own report directory:

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

If any required report is missing, document it and mark that horizon incomplete.

---

## Required Analysis Document

Create:

```text
docs/forecast-ml-horizon-comparison.md
```

The document must contain these sections.

### 1. Executive Decision

Choose exactly one:

```text
candidate_horizon_found
iterate_regime_attribution_next
iterate_label_design_next
iterate_cost_sensitivity_next
insufficient_completed_horizon_runs
```

Decision rules:

```text
if fewer than 4 full horizon runs completed:
  insufficient_completed_horizon_runs

else if one or more horizons have top_decile_avg_effective_actual_bps > 0, positive correlation, and reasonably monotonic buckets:
  candidate_horizon_found

else if longer horizons improve ranking but still fail after cost:
  iterate_regime_attribution_next or iterate_cost_sensitivity_next

else if no horizon shows improved ranking/cost behavior:
  iterate_label_design_next
```

Be conservative.
Do not claim profitability.

### 2. Run Matrix

Create a table:

```text
horizon
config_path
reports_dir
completed
input_rows
output_rows
window_count
ridge_prediction_count
rf_prediction_count
model_comparison_recommendation
```

### 3. Dataset And Label Comparison

For each horizon, compare:

```text
input_rows
output_rows
skipped_label_horizon
avg_future_return_bps
avg_future_return_after_cost_bps
```

Interpret:

```text
- Does raw average forward return change with horizon?
- Does after-cost average become less negative at longer horizons?
- Does label horizon create meaningful row loss?
```

### 4. Ridge Horizon Comparison

For each horizon, include:

```text
MAE
RMSE
correlation
directional_accuracy
avg_predicted_bps
avg_actual_after_cost_bps
top_decile_avg_effective_actual_bps
top_decile_hit_rate_effective_target
monotonicity_ratio
top_minus_bottom_spread_bps
```

Interpret:

```text
- Which horizon is best for Ridge by top-decile effective return?
- Which horizon is best by correlation?
- Does longer horizon improve cost-adjusted ranking?
```

### 5. Random Forest Horizon Comparison

For each horizon, include the same metrics:

```text
MAE
RMSE
correlation
directional_accuracy
avg_predicted_bps
avg_actual_after_cost_bps
top_decile_avg_effective_actual_bps
top_decile_hit_rate_effective_target
monotonicity_ratio
top_minus_bottom_spread_bps
```

Also include top 5 RF features per horizon:

```text
feature
split_count
importance
```

Interpret:

```text
- Does RF improve more at longer horizons?
- Are dominant features stable across horizons?
- Do time/calendar features dominate all horizons or only shorter horizons?
- Does feature importance shift from microstructure to trend/volatility as horizon increases?
```

### 6. Cross-Horizon Ranking Power

Create a combined table:

```text
horizon
best_model_by_rmse
best_model_by_correlation
best_model_by_top_decile_return
best_model_name
best_top_decile_avg_effective_actual_bps
best_correlation
best_monotonicity_ratio
best_top_minus_bottom_spread_bps
recommendation
```

Interpret:

```text
- Does the best horizon survive cost?
- Is improvement monotonic from 15m -> 30m -> 1h -> 4h?
- Does longer horizon reduce cost decay?
- Is any improvement strong enough for ForecastScorer testing?
```

### 7. Decision

Choose exactly one final decision:

```text
candidate_horizon_found
iterate_regime_attribution_next
iterate_label_design_next
iterate_cost_sensitivity_next
insufficient_completed_horizon_runs
```

If `candidate_horizon_found`, state:

```text
candidate_horizon = <15m|30m|1h|4h>
candidate_model = <ridge|random_forest>
why it passed
what still must be validated before strategy integration
```

If not candidate, state the next research phase and why.

### 8. Next Phase Recommendation

Use this ordering:

```text
1. If a horizon survives cost: regime attribution for that horizon.
2. If longer horizons improve but remain negative: cost sensitivity and regime attribution.
3. If all horizons are poor: alternative labels first.
4. If runs incomplete: complete runs before further analysis.
```

Do not recommend ForecastScorer unless a horizon actually survives cost.

---

## Required Helper Logic

If parsing reports manually is too error-prone, create a small helper under:

```text
src/forecast/analysis.rs
```

or

```text
tests/forecast_horizon_analysis.rs
```

Only if needed.

Any helper must remain report-analysis-only. It must not be used by strategy/backtest/risk/accounting.

Do not introduce heavy dependencies just to parse JSON/CSV. Prefer simple std parsing if current project avoids extra crates.

---

## Required Quality Checks

Search the result doc for forbidden phrases:

```text
guaranteed profit
profitable strategy
production-ready
live-ready
edge confirmed
safe to trade
```

Allowed phrasing:

```text
cost-adjusted predictive signal candidate
ranking power
weak signal
does not survive cost
candidate for future regime attribution
candidate for future backtest-filter testing
not a profitability claim
```

---

## Required Tests

If new helper logic is added, test it.

Minimum tests if helper exists:

```text
parses model_comparison.json summary
parses prediction bucket top/bottom metrics
computes monotonicity_ratio
computes top_minus_bottom_spread_bps
handles missing report file cleanly
```

If no helper logic is added and this is documentation-only plus config files, no new Rust tests are required beyond existing `cargo test`.

---

## Git Hygiene

Before commit:

```bash
git status --short
```

Ensure only intended files are staged:

```text
config/forecast/btcusdt_1m_h15.toml
config/forecast/btcusdt_1m_h30.toml
config/forecast/btcusdt_1m_h1h.toml
config/forecast/btcusdt_1m_h4h.toml
docs/forecast-ml-horizon-comparison.md
optional docs pointer update
optional helper/tests only if needed
```

Do not stage or commit:

```text
reports/
target/
large CSV outputs
local scratch files
```

---

## Acceptance Criteria

This phase is complete only when:

```text
- horizon-specific configs exist
- all completed runs use full configs, not smoke configs
- docs/forecast-ml-horizon-comparison.md exists
- the doc compares 15m, 30m, 1h, and 4h, or explicitly states which full runs did not complete
- the doc includes dataset, label, Ridge, RF, bucket, feature-importance, and model-comparison analysis
- one explicit final decision is made
- no profitability claim is made
- no strategy/backtest/risk/fill/accounting behavior is changed
- reports/ is not committed
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
new model family
hyperparameter optimization
new indicators
paper trading
live trading
exchange integration
Python analysis pipeline
UI/dashboard
```

This phase is horizon comparison and research decision only.
