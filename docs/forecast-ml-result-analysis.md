# Forecast ML Result Analysis & Research Decision

Phase: `F6 — Forecast Result Analysis & Research Decision`

This document is analysis of already-generated forecast reports. It does not
change strategy, backtest, risk, fill, accounting, or indicator behavior, and
it does not claim profitability. `reports/` is gitignored; nothing under it is
committed as part of this change.

Reports analyzed (from `reports/forecast/btcusdt_1m_h15/`, produced by
`cargo run --release -- forecast --config config/forecast.toml` against the
full BTCUSDT 1m 2020–2025 historical set): `dataset_summary.json`,
`feature_summary.csv`, `label_summary.json`, `walk_forward_windows.csv`,
`ridge_summary.json`, `ridge_prediction_buckets.csv`,
`random_forest_summary.json`, `random_forest_prediction_buckets.csv`,
`random_forest_feature_importance.csv`, `model_comparison.json`,
`forecast_run_manifest.json`. All expected files were present; none were
missing or stale relative to the current forecast code.

---

## 1. Executive Decision

```text
iterate_feature_label_horizon
```

Both models show a small amount of real, cost-*un*adjusted ranking signal
(monotonic-ish prediction buckets, weak positive correlation), but that signal
does not survive the configured round-trip trading cost at the current 15-bar
(15-minute) horizon: the top decile's average cost-adjusted return is negative
for both Ridge (-12.71 bps) and Random Forest (-12.98 bps), per
`model_comparison.json`'s own `reject_due_to_cost_adjusted_decay`
recommendation. This is not a "no signal at all" result (Random Forest's
buckets are 8/9 monotonic), so a full `reject_current_setup` would discard a
data point that horizon/cost iteration might still resolve. It is also not
`candidate_for_forecast_scorer_backtest_filter`, because the decision rule for
that requires the top-decile result to survive costs, which it does not here.
This is **not a profitability claim** either way — it is a statement about
whether this specific feature/label/horizon/cost combination is worth carrying
forward as-is.

## 2. Run Context

From `forecast_run_manifest.json`:

| Field | Value |
|---|---|
| symbols | BTCUSDT |
| source_timeframe | 1m |
| entry_timeframe | 1m |
| forecast_horizon | 15m |
| horizon_bars | 15 |
| configured_target | future_return_bps |
| effective_target | future_return_after_cost_bps |
| cost_adjusted | true |
| enabled_features | return_1m, return_5m, return_15m, atr_bps, volume_ratio, vwap_distance_bps, ema_8_21_spread_bps, range_position, hour_of_day, day_of_week |
| enabled_models | ridge, random_forest |
| walk_forward.train_months | 12 |
| walk_forward.test_months | 3 |
| walk_forward.step_months | 3 |
| walk_forward.embargo_bars | 15 |
| walk_forward.month_model | fixed_30_day_months |
| reports_written | all 13 expected files (listed above) |
| limitations | source timeframe currently supports only 1m; walk-forward months use fixed 30-day approximation; classification target is not implemented; forecast module does not emit production trading signals; paper/live remain disabled |

Round-trip cost assumptions in `config/forecast.toml`: `taker_fee_bps = 4.0`,
`slippage_bps = 2.0`, `spread_bps = 1.0`, `market_impact_bps = 1.0`,
`stop_slippage_bps = 5.0`. These are applied per row as a synthetic
round-trip cost, not a live fill simulation — see Section 3.

## 3. Dataset Summary

From `dataset_summary.json` and `label_summary.json`:

| Metric | Value |
|---|---|
| input_rows | 3,156,480 |
| output_rows | 3,156,059 |
| feature_count | 10 |
| skipped_missing_feature | 406 |
| skipped_invalid_feature | 0 |
| skipped_label_horizon | 15 |
| skipped_invalid_close | 0 |
| skipped_invalid_label | 0 |
| avg_future_return_bps | 0.17851031 |
| avg_future_return_after_cost_bps | -13.82148969 |

Interpretation:

- **Usable row count**: 3,156,059 of 3,156,480 rows (99.99%) survive to become
  labeled training rows — comfortably sufficient for 20 walk-forward windows
  of ~518k train rows each.
- **Skipped rows are expected and small**: 406 rows skipped for missing
  feature warm-up (indicator lookback) and 15 for label horizon (last 15 bars
  of the dataset can't have a future 15-bar label). Nothing indicates a data
  quality problem.
- **The average after-cost label is already deeply negative** (-13.82 bps vs.
  +0.18 bps before cost). This is expected given the cost assumptions: this
  module applies a synthetic full round-trip cost to *every single row* as if
  every bar were an independent completed trade, rather than only to rows a
  strategy would actually act on. At a 15-minute holding horizon, this fixed
  cost (~14 bps) dwarfs the typical raw 15-minute move, so the after-cost
  average is dominated by the cost term, not by directional edge. This is a
  property of the current label design, not evidence about a live strategy's
  real trade frequency or cost exposure.

## 4. Walk-Forward Summary

From `walk_forward_windows.csv`:

| Metric | Value |
|---|---|
| window_count | 20 |
| first_train_start | 2020-01-01 00:19 UTC |
| last_test_end | 2025-11-30 07:00 UTC |
| train_rows per window | 518,400 (constant across all 20 windows) |
| test_rows per window | 129,600 (constant across all 20 windows) |
| embargo_bars | 15 |

Interpretation: a 12-month train / 3-month test / 3-month step design over
~6 years of 1-minute BTCUSDT data gives 20 non-degenerate, embargoed
walk-forward windows with constant sizing — an adequate design for a first
analysis pass. It is not yet enough to draw regime-specific conclusions (e.g.
2020 vs. 2022 vs. 2024 market conditions are pooled together in the headline
metrics); regime attribution is proposed as a next iteration in Section 11.

## 5. Model Summary — Ridge

From `ridge_summary.json` and `ridge_prediction_buckets.csv`:

| Metric | Value |
|---|---|
| prediction_count | 2,592,000 |
| MAE | 19.92677980 |
| RMSE | 33.30022337 |
| correlation | 0.03685246 |
| directional_accuracy | 0.77391088 |
| avg_predicted_bps | -13.84612279 |
| avg_actual_bps | -13.86896870 |
| avg_actual_after_cost_bps | -13.86896870 |
| top_decile_avg_effective_actual_bps | -12.71114098 |
| top_decile_hit_rate_effective_target | 0.38776620 |
| bucket monotonicity | 6/9 adjacent-step increases (ratio 0.667) |

Interpretation:

- **Weak positive ranking power.** Correlation of 0.037 is small but non-zero,
  and consistent with the bucket spread below.
- **Directional accuracy of ~77% is misleading in isolation.** Because both
  the average prediction (-13.85 bps) and the average effective target
  (-13.87 bps) are dominated by the same large negative constant cost term,
  "predicted sign matches actual sign" is satisfied for most rows simply
  because almost everything is negative — this metric is not evidence of
  strong directional skill here and should be read alongside correlation and
  bucket spread, not on its own.
- **Top decile cost-adjusted return is negative** (-12.71 bps): even Ridge's
  highest-predicted decile does not overcome the round-trip cost assumption
  at this horizon.
- **Buckets are directionally ordered but noisy**: 6 of 9 adjacent bucket
  steps increase monotonically (ratio 0.667); the bottom-to-top spread is
  1.59 bps, all still negative. This is a real but small and imperfect
  ranking signal, not a clean monotonic curve.

## 6. Model Summary — Random Forest

From `random_forest_summary.json`, `random_forest_prediction_buckets.csv`, and
`random_forest_feature_importance.csv`:

| Metric | Value |
|---|---|
| prediction_count | 2,592,000 |
| MAE | 19.95254189 |
| RMSE | 33.33399741 |
| correlation | 0.02397112 |
| directional_accuracy | 0.77383063 |
| avg_predicted_bps | -13.82422394 |
| avg_actual_bps | -13.86896870 |
| avg_actual_after_cost_bps | -13.86896870 |
| top_decile_avg_effective_actual_bps | -12.98284627 |
| top_decile_hit_rate_effective_target | 0.34087191 |
| bucket monotonicity | 8/9 adjacent-step increases (ratio 0.889) |

Top features by real split-count importance (see Section 9 for the full
ranked list and interpretation): `atr_bps` (15.4%), `vwap_distance_bps`
(13.6%), `hour_of_day` (13.4%).

Interpretation:

- **Random Forest does not clearly improve over Ridge on the headline
  metrics.** RMSE is marginally worse (33.334 vs. 33.300), correlation is
  lower (0.024 vs. 0.037), and top-decile cost-adjusted return is slightly
  more negative (-12.98 vs. -12.71 bps). On this run, Ridge is the better
  model by every metric `model_comparison.json` tracks.
- **Random Forest's bucket ordering is smoother** (8/9 monotonic steps vs.
  Ridge's 6/9), suggesting it captures a more consistent (if still small)
  monotonic relationship between predicted rank and realized cost-adjusted
  return, even though the average level and overall correlation are not
  better than Ridge's.
- Any improvement Random Forest offers here is **marginal at best and not
  consistent across metrics** — it is not a case of Random Forest clearly
  outperforming a simpler linear model.

## 7. Model Comparison

From `model_comparison.json`:

| Field | Value |
|---|---|
| best_model_by_rmse | ridge |
| best_model_by_correlation | ridge |
| best_model_by_top_decile_return | ridge |
| recommendation | reject_due_to_cost_adjusted_decay |

Independent check against the model summaries and buckets: this
recommendation is justified. Ridge wins on every tracked metric, and neither
model's top decile clears zero after cost (-12.71 bps for Ridge, -12.98 bps
for Random Forest — see Section 8 for the full bucket picture). What decayed:
**top-decile effective after-cost return <= 0 for both models** — there is a
small, measurable ranking signal in both models before cost is considered
(positive top-minus-bottom spread, non-zero correlation), but it does not
survive the round-trip cost assumption at the 15-minute horizon. This is a
"ranking exists but does not survive costs" case, not a "no ranking signal at
all" case.

## 8. Prediction Bucket / Decile Analysis

This is the most load-bearing section for the decision above.

| | Ridge | Random Forest |
|---|---|---|
| bottom bucket avg_effective_actual_bps | -14.30232585 | -14.35260940 |
| top bucket avg_effective_actual_bps | -12.71114098 | -12.98284627 |
| top minus bottom spread | 1.5912 bps | 1.3698 bps |
| count of positive-effective buckets (of 10) | 0 | 0 |
| monotonic_steps / max_steps | 6 / 9 | 8 / 9 |
| monotonicity_ratio | 0.667 | 0.889 |

Do higher predicted buckets generally correspond to better realized effective
return? **Yes, directionally, for both models** — the top decile is always
the least-negative bucket and the bottom decile the most-negative, for both
Ridge and Random Forest, and the ordering is mostly (Ridge) to almost
entirely (Random Forest) monotonic in between. **But no bucket, for either
model, has a positive average effective (cost-adjusted) return.** The entire
decile spread sits below zero; the model can meaningfully rank rows from
"less bad" to "worse," but not yet identify rows with a positive expected
cost-adjusted 15-minute return on average.

Overall pattern for both models: **weak-to-moderate ranking power combined
with cost-adjusted decay** — real, non-random ordering, fully consumed by the
round-trip cost assumption at this horizon. Not an inverted-ranking result
(nothing suggests the models are anti-predictive), and not pure noise either
(Random Forest in particular is close to fully monotonic).

## 9. Feature Importance Interpretation

From `random_forest_feature_importance.csv`, all 10 enabled features ranked
by split-count importance (this is the complete feature set, so "top 10" here
is the full ranked list):

| Rank | Feature | Split count | Importance |
|---|---|---|---|
| 1 | atr_bps | 78,450 | 15.38% |
| 2 | vwap_distance_bps | 69,203 | 13.57% |
| 3 | hour_of_day | 68,548 | 13.44% |
| 4 | day_of_week | 61,583 | 12.08% |
| 5 | ema_8_21_spread_bps | 58,567 | 11.48% |
| 6 | return_15m | 47,548 | 9.32% |
| 7 | return_5m | 39,970 | 7.84% |
| 8 | volume_ratio | 33,812 | 6.63% |
| 9 | return_1m | 27,159 | 5.33% |
| 10 | range_position | 25,160 | 4.93% |

(Importances sum to ~1.00, as expected for a split-count-normalized measure.)

Per-feature interpretation (proxy meaning, not a causal claim):

- **atr_bps** (top feature): a volatility-regime proxy. Plausible that
  volatility regime shifts the distribution of forward returns and of the
  fixed cost's relative impact.
- **vwap_distance_bps**: distance from a rolling/session value-area proxy;
  plausibly captures short-term mean-reversion or extension state.
- **hour_of_day** and **day_of_week**: session/calendar behavior proxies.
  Together these two account for over a quarter of total split usage, which
  is notable for two purely calendar-based features with no explicit
  cyclical encoding. This is **plausible** (crypto liquidity and volatility do
  have known session/day patterns) but also the kind of result that
  **requires further validation** before being trusted — a tree model with
  `min_samples_leaf = 50` over 500k+ rows can find calendar splits that
  describe historical idiosyncrasies of this specific 2020–2025 sample
  without necessarily generalizing forward.
- **ema_8_21_spread_bps**: a trend-pressure proxy (short vs. medium EMA
  spread).
- **return_15m / return_5m / return_1m**: recent momentum/mean-reversion
  proxies at different lookbacks; usage decreases as the lookback shortens,
  which is a plausible pattern (shorter lookbacks are noisier at 1-minute
  bar resolution).
- **volume_ratio**: liquidity/participation proxy.
- **range_position** (lowest importance): position within a recent
  high-low range; still used, but least often of the ten features.

Split-count importance means these features were used often by Random
Forest's splits on this training data — it does not mean any of them causes
profitable trades, and it does not mean the ranking survives cost (Section 8
shows it does not, on average).

## 10. Research Decision

```text
iterate_feature_label_horizon
```

Rationale recap: required reports were all present (ruling out
`insufficient_data_or_reports`). Top-decile average effective return is <= 0
for both models, which the decision rules map to
`reject_current_setup or iterate_feature_label_horizon`. Given that Random
Forest's bucket ordering is 8/9 monotonic and both models show non-zero
correlation and a consistent (if small) top-minus-bottom spread, there is
enough real ranking signal to prefer iterating on horizon/label/cost design
over discarding the feature set outright. This is **not a profitability
claim** and is not `candidate_for_forecast_scorer_backtest_filter`, since the
result does not currently survive cost at this horizon.

## 11. Next Research Iteration

In preferred order:

1. **Horizon comparison**: re-run with 30m, 1h, and 4h forecast horizons
   alongside the current 15m. The fixed round-trip cost assumption is a much
   larger fraction of a 15-minute expected move than of a longer move, so a
   longer horizon is the most direct lever to test whether ranking signal can
   survive cost.
2. **Regime attribution**: break down bucket/correlation results by
   trend/range and high-vol/low-vol regimes rather than pooling all 20
   windows together, to check whether the weak aggregate signal is
   concentrated in specific regimes.
3. **Alternative labels**: try `hit_tp_before_sl`, `mfe_bps`, `mae_bps`, or a
   volatility-adjusted return label instead of a flat future-return-after-cost
   label, since the current label penalizes every row with the same fixed
   cost regardless of realized favorable excursion.
4. **Cost sensitivity**: re-run with lower and higher cost assumptions than
   the current `taker_fee_bps = 4.0` / `slippage_bps = 2.0` / `spread_bps =
   1.0` / `market_impact_bps = 1.0` to see how sensitive the
   `reject_due_to_cost_adjusted_decay` outcome is to the specific cost inputs.
5. **Session split**: analyze Asia/London/US session segments separately,
   given `hour_of_day` and `day_of_week` already rank highly in feature
   importance (Section 9) — this may be a more direct way to test whether
   that split usage reflects a real, exploitable session pattern.

Adding more complex models is **not** recommended before this horizon/label/
regime iteration is done, consistent with the roadmap's guidance.

---

## Language check

This document does not claim guaranteed returns, does not call the current
setup a profitable or ready-to-deploy strategy, and does not claim any edge
has been confirmed. It uses conservative, evidence-based framing throughout —
describing results as a weak or partial predictive signal, noting where
cost-adjusted decay occurs, flagging where findings need further validation,
and treating "candidate for a future backtest-filter phase" as the most
positive available outcome — and repeatedly notes that none of the above is a
profitability claim.
