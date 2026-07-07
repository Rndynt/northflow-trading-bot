# Forecast ML Horizon Comparison & Research Decision

Phase: `F7 — Forecast Horizon Comparison`

This document is the finalized F7 decision after the full Random Forest horizon reports from Kaggle were attached to the repository in commit `b349427450cfdd0a94d8f8dd226832e080fb7b4d`.

The previous version of this document had decision `insufficient_completed_horizon_runs` because the chat sandbox could not finish full Random Forest runs for 30m, 1h, and 4h. That is no longer the active status.

Current status:

```text
15m : full run available from F6/F7 prior analysis
30m : full run available, reports committed
1h  : full run available, reports committed
4h  : full run available, reports committed
```

This remains a forecast research document only. It does not change strategy, backtest, risk, fill, accounting, or indicator behavior. It does not claim profitability.

---

## 1. Executive Decision

```text
iterate_cost_sensitivity_next
```

Reason:

- Every completed horizon still returns `reject_due_to_cost_adjusted_decay`.
- Ridge beats Random Forest by RMSE, correlation, and top-decile effective return on the committed 30m, 1h, and 4h full reports.
- The best observed result remains **4h Ridge**, with top-decile effective actual return of `-3.67512960 bps`; this is much better than 15m/30m/1h, but still negative after the configured cost model.
- Random Forest does not improve the decision. At 4h, RF correlation is nearly zero (`0.00124745`) and RF top-decile effective actual return is `-10.59296989 bps`.

Therefore the project should not integrate `ForecastScorer` or any ML backtest filter yet. The next useful phase is cost sensitivity, focused on whether the 4h Ridge ranking candidate survives under different realistic cost assumptions. If it still fails, move to regime attribution and label redesign.

---

## 2. Run Matrix

| Horizon | Full RF report committed | Best model by RMSE | Best model by correlation | Best model by top decile | Recommendation |
|---|---:|---|---|---|---|
| 15m | Yes | ridge | ridge | ridge | `reject_due_to_cost_adjusted_decay` |
| 30m | Yes | ridge | ridge | ridge | `reject_due_to_cost_adjusted_decay` |
| 1h | Yes | ridge | ridge | ridge | `reject_due_to_cost_adjusted_decay` |
| 4h | Yes | ridge | ridge | ridge | `reject_due_to_cost_adjusted_decay` |

The 30m/1h/4h committed report files include:

```text
random_forest_summary.json
random_forest_prediction_buckets.csv
random_forest_feature_importance.csv
model_comparison.json
forecast_run_manifest.json
```

---

## 3. Model Comparison Summary

### 15m baseline from prior F6/F7 analysis

| Model | Correlation | Top-decile effective actual bps | Decision |
|---|---:|---:|---|
| Ridge | 0.03685246 | -12.71114098 | reject |
| Random Forest | 0.02397112 | -12.98284627 | reject |

### 30m full report

| Model | RMSE | Correlation | Directional accuracy | Top-decile effective actual bps | Top-decile hit rate | Decision |
|---|---:|---:|---:|---:|---:|---|
| Ridge | 46.52136532 | 0.04519512 | 0.71552623 | -12.03276789 | 0.42408565 | reject |
| Random Forest | 46.58033071 | 0.03325657 | 0.71518557 | -12.55640802 | 0.37937886 | reject |

30m interpretation:

- Ridge is better than Random Forest on all headline selection metrics.
- RF still has ranking shape in buckets, but the top decile remains negative after cost.
- No 30m result justifies a ForecastScorer phase.

### 1h full report

| Model | RMSE | Correlation | Directional accuracy | Top-decile effective actual bps | Top-decile hit rate | Decision |
|---|---:|---:|---:|---:|---:|---|
| Ridge | 64.98835806 | 0.03772052 | 0.66095216 | -11.96930566 | 0.44913194 | reject |
| Random Forest | 65.26776953 | 0.01369991 | 0.65766782 | -12.57501030 | 0.40711420 | reject |

1h interpretation:

- Ridge again beats RF.
- The top-decile effective result improves only slightly versus 30m.
- RF weakens materially by correlation and top-decile result.

### 4h full report

| Model | RMSE | Correlation | Directional accuracy | Top-decile effective actual bps | Top-decile hit rate | Decision |
|---|---:|---:|---:|---:|---:|---|
| Ridge | 128.59820494 | 0.03252265 | 0.57677778 | -3.67512960 | 0.49075617 | reject, but closest |
| Random Forest | 131.97286014 | 0.00124745 | 0.55173302 | -10.59296989 | 0.45266590 | reject |

4h interpretation:

- 4h Ridge is the only result close enough to deserve another research pass.
- It still does not survive the configured cost model.
- 4h RF is not useful as a candidate because its correlation is essentially zero and its top-decile effective return remains strongly negative.

---

## 4. Cross-Horizon Decision

The key pattern is:

```text
15m Ridge top decile : -12.71114098 bps
30m Ridge top decile : -12.03276789 bps
1h Ridge top decile  : -11.96930566 bps
4h Ridge top decile  :  -3.67512960 bps
```

The longer horizon reduces cost drag and improves the Ridge top-decile bucket, but it does not cross zero.

Random Forest does not improve the situation:

```text
15m RF top decile : -12.98284627 bps
30m RF top decile : -12.55640802 bps
1h RF top decile  : -12.57501030 bps
4h RF top decile  : -10.59296989 bps
```

The final F7 conclusion is therefore:

```text
Do not integrate forecast into strategy/backtest yet.
Do not build ForecastScorer yet.
Do not add a more complex model yet.
Run cost sensitivity on the 4h Ridge candidate first.
```

---

## 5. Random Forest Feature Importance Notes

The committed RF feature importance reports show real split-count importance.

### 30m top features

| Feature | Importance |
|---|---:|
| atr_bps | 0.16811994 |
| vwap_distance_bps | 0.16301405 |
| hour_of_day | 0.16057286 |
| day_of_week | 0.13895517 |
| ema_8_21_spread_bps | 0.10710609 |

### 1h top features

| Feature | Importance |
|---|---:|
| vwap_distance_bps | 0.20661102 |
| hour_of_day | 0.19827372 |
| atr_bps | 0.18096969 |
| day_of_week | 0.16012055 |
| ema_8_21_spread_bps | 0.09835352 |

### 4h top features

| Feature | Importance |
|---|---:|
| vwap_distance_bps | 0.28159859 |
| hour_of_day | 0.21614160 |
| atr_bps | 0.19290822 |
| day_of_week | 0.18609448 |
| ema_8_21_spread_bps | 0.06504915 |

Interpretation:

- RF split usage increasingly concentrates around `vwap_distance_bps`, `hour_of_day`, `atr_bps`, and `day_of_week` at longer horizons.
- This is useful for attribution, but it is not a profitability claim.
- Because RF does not outperform Ridge, this does not justify a model-complexity increase.

---

## 6. Next Phase

Create and run:

```text
F8 — Forecast Cost Sensitivity & Regime Attribution
```

F8 should focus on:

1. 4h Ridge cost sensitivity.
2. 4h Ridge regime attribution: trend/range/high-vol/low-vol.
3. Session attribution only if cost sensitivity shows a near-positive candidate.
4. No ForecastScorer unless a subset survives cost.

Decision gate for ForecastScorer remains strict:

```text
top_decile_avg_effective_actual_bps > 0
correlation > 0
monotonicity_ratio >= 0.60
top_minus_bottom_spread_bps > 0
result persists across walk-forward windows or regimes
```

Current F7 does not pass that gate.
