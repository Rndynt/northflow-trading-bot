# Forecast ML Cost Sensitivity & Regime Attribution Prompt

## Phase

```text
F8 — Forecast Cost Sensitivity & Regime Attribution
```

## References

Read these first:

```text
docs/forecast-ml-roadmap.md
docs/forecast-ml-implementation-report.md
docs/forecast-ml-result-analysis.md
docs/forecast-ml-horizon-comparison.md
reports/forecast/btcusdt_1m_h30/model_comparison.json
reports/forecast/btcusdt_1m_h1h/model_comparison.json
reports/forecast/btcusdt_1m_h4h/model_comparison.json
config/forecast/btcusdt_1m_h4h.toml
```

F7 is now complete: full Random Forest reports exist for 30m, 1h, and 4h. The final F7 decision is `iterate_cost_sensitivity_next`.

The best candidate from F7 is not Random Forest. It is the 4h Ridge ranking result:

```text
4h Ridge top-decile effective actual return = -3.67512960 bps
```

This is still negative after the configured cost model, so ForecastScorer/backtest-filter integration is not allowed yet.

---

## Core Objective

Determine whether the 4h Ridge candidate fails only because the configured round-trip cost is too conservative, or whether it fails even under lower realistic cost assumptions.

Then determine whether any simple market regime subset makes the 4h Ridge top decile positive after cost.

Create:

```text
docs/forecast-ml-cost-sensitivity-regime-attribution.md
```

This is an analysis/research phase only.

---

## Non-Negotiable Boundaries

Do not modify production strategy behavior.
Do not add an ML-backed strategy.
Do not integrate ForecastScorer.
Do not connect forecast output to BacktestEngine.
Do not make strategy depend on forecast.
Do not change risk sizing, fill simulation, accounting, existing indicator formulas, paper mode, or live mode.
Do not add exchange/API/network/LLM calls.
Do not claim profitability.
Do not commit large generated files.

Allowed changes:

```text
- add cost-sensitivity forecast configs
- add regime-attribution analysis helper only under src/forecast or tests if needed
- add docs/forecast-ml-cost-sensitivity-regime-attribution.md
- optionally add small summary CSV/JSON reports if they are lightweight
```

---

## Required Cost Sensitivity Configs

Create configs under:

```text
config/forecast/cost_sensitivity/
```

Use the same 4h setup as:

```text
config/forecast/btcusdt_1m_h4h.toml
```

but change only cost assumptions and reports directory.

Required configs:

```text
config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_14bps.toml
config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_10bps.toml
config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_7bps.toml
config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_5bps.toml
```

Definitions:

### 14 bps baseline

Same as current config:

```text
taker_fee_bps = 4.0
slippage_bps = 2.0
spread_bps = 1.0
market_impact_bps = 1.0
round_trip_cost = 14 bps
```

### 10 bps moderate

Example:

```text
taker_fee_bps = 3.0
slippage_bps = 1.5
spread_bps = 1.0
market_impact_bps = 0.0
round_trip_cost = 10 bps
```

### 7 bps optimistic but plausible

Example:

```text
taker_fee_bps = 2.0
slippage_bps = 1.0
spread_bps = 1.0
market_impact_bps = 0.0
round_trip_cost = 7 bps
```

### 5 bps aggressive

Example:

```text
taker_fee_bps = 1.5
slippage_bps = 0.5
spread_bps = 1.0
market_impact_bps = 0.0
round_trip_cost = 5 bps
```

Do not silently change features, model params, data years, walk-forward windows, or horizon.

Reports dirs:

```text
reports/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_14bps
reports/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_10bps
reports/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_7bps
reports/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_5bps
```

---

## Required Commands

Run the 4h cost sensitivity configs.

If full Random Forest is too slow, this phase may use Ridge-only configs because F7 already showed Random Forest is worse at 4h. Document that explicitly.

Preferred implementation:

```text
Ridge-only for cost sensitivity first.
```

Why:

```text
F7 model comparison showed Ridge beats RF on 4h by RMSE, correlation, and top-decile return.
Cost sensitivity should test the best candidate, not spend hours rerunning the weaker RF first.
```

Required commands:

```bash
cargo run --release -- forecast --config config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_14bps.toml
cargo run --release -- forecast --config config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_10bps.toml
cargo run --release -- forecast --config config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_7bps.toml
cargo run --release -- forecast --config config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_5bps.toml
```

Also run:

```bash
cargo fmt --check
cargo test
```

---

## Required Cost Sensitivity Analysis

For each cost config, extract Ridge metrics from `model_comparison.json` or `ridge_summary.json` + `ridge_prediction_buckets.csv`:

```text
round_trip_cost_bps
MAE
RMSE
correlation
directional_accuracy
avg_actual_after_cost_bps
top_decile_avg_effective_actual_bps
top_decile_hit_rate_effective_target
bottom_decile_avg_effective_actual_bps
top_minus_bottom_spread_bps
monotonicity_ratio
recommendation
```

Create a table:

```text
cost_bps
top_decile_avg_effective_actual_bps
top_decile_hit_rate_effective_target
top_minus_bottom_spread_bps
monotonicity_ratio
correlation
passes_candidate_gate
```

Candidate gate:

```text
top_decile_avg_effective_actual_bps > 0
correlation > 0
monotonicity_ratio >= 0.60
top_minus_bottom_spread_bps > 0
```

Do not call it profitable. Passing this gate means only: eligible for regime attribution or future backtest-filter experiment.

---

## Required Regime Attribution

If any cost sensitivity result gets close to or above zero, run regime attribution on the best 4h Ridge report.

Minimum regimes:

```text
trend_proxy:
  ema_8_21_spread_bps > 0 vs <= 0

volatility_proxy:
  atr_bps top 30% vs bottom 30%

value_distance_proxy:
  vwap_distance_bps positive vs negative

session_proxy:
  hour_of_day buckets: Asia, London, US/NY, Other
```

For each regime, compute top-decile effective actual return using the model predictions and rows in that subset.

Required output table:

```text
regime_name
subset
row_count
top_decile_avg_effective_actual_bps
top_decile_hit_rate_effective_target
correlation
passes_candidate_gate
```

If the existing report files do not contain enough feature values to compute regimes, implement a lightweight forecast analysis helper that rebuilds the dataset from config and joins predictions by timestamp.

Keep helper under `src/forecast/*` only. Do not touch strategy/backtest/risk/accounting.

---

## Required Decision

The doc must choose exactly one:

```text
candidate_for_regime_scoped_forecast_filter
iterate_label_design_next
iterate_cost_model_validation_next
reject_current_feature_label_setup
```

Decision rules:

```text
if no cost scenario makes top decile positive:
  iterate_label_design_next or reject_current_feature_label_setup

if only aggressive 5 bps cost makes top decile positive:
  iterate_cost_model_validation_next

if 7-10 bps cost makes top decile positive and regime attribution finds stable subset:
  candidate_for_regime_scoped_forecast_filter

if cost sensitivity is positive but regimes are unstable/noisy:
  iterate_cost_model_validation_next
```

Be conservative.

---

## Required Documentation

Create:

```text
docs/forecast-ml-cost-sensitivity-regime-attribution.md
```

Sections:

```text
1. Executive Decision
2. Why F8 exists
3. Inputs used
4. Cost sensitivity configs
5. Ridge cost sensitivity table
6. Candidate gate result
7. Regime attribution results
8. Interpretation
9. Final decision
10. Next phase recommendation
11. Commands run
12. Boundary confirmation
```

Boundary confirmation must state:

```text
- no strategy logic changed
- no backtest engine integration added
- no ForecastScorer added
- no paper/live enabled
- no profitability claim made
```

---

## Git Hygiene

Do not commit large reports.

Allowed to commit:

```text
config/forecast/cost_sensitivity/*.toml
docs/forecast-ml-cost-sensitivity-regime-attribution.md
small helper/test files if added
small summary report only if intentionally created and reviewed
```

Do not commit:

```text
reports/forecast/cost_sensitivity/**/ridge_walk_forward.csv
reports/forecast/cost_sensitivity/**/random_forest_walk_forward.csv
target/
large CSVs
```

---

## Acceptance Criteria

This phase is complete only when:

```text
- cost sensitivity configs exist
- 4h Ridge is evaluated under 14/10/7/5 bps cost assumptions
- docs/forecast-ml-cost-sensitivity-regime-attribution.md exists
- decision is explicit and conservative
- no ForecastScorer integration is added
- no strategy/backtest/risk/fill/accounting behavior changes
- cargo fmt --check passes
- cargo test passes
```
