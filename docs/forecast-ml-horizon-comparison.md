# Forecast ML Horizon Comparison & Research Decision

Phase: `F7 — Forecast Horizon Comparison`

This document compares the forecast research pipeline across four horizons
(15m, 30m, 1h, 4h) on the full BTCUSDT 2020–2025 1-minute historical set. It
does not change strategy, backtest, risk, fill, accounting, or indicator
behavior, and it does not claim profitability. `reports/` is gitignored;
nothing under it is committed as part of this change.

## Important environment caveat (read first)

**Only the 15m horizon has a complete full-config run** (Ridge + Random
Forest + `model_comparison.json` + `forecast_run_manifest.json`), carried
over unchanged from the F6 result-analysis phase. For 30m, 1h, and 4h, this
session's sandboxed execution environment could not sustain the ~40 minutes
of continuous single-core compute a full 100-tree Random Forest walk-forward
run requires: repeated attempts showed background processes and long
synchronous commands both being terminated well before completion (observed
survival window roughly 60–300 seconds), even though an equivalent 15m run
completed successfully in a prior session using the same code and config
shape. Ridge is a closed-form solve and completes in well under a minute, so
**full, real Ridge results were obtained for all four horizons.** For Random
Forest at 30m/1h/4h, a clearly-labeled **local-only smoke config** (`trees =
3`, `max_depth = 4`, everything else identical, reports written to `/tmp`,
never committed) was used to get a fast, directional-only read, per this
phase's explicit allowance for environments that cannot complete full runs.
**Smoke RF results are marked SMOKE throughout this document and are not used
as the basis for the final decision.**

This is documented, not hidden, per the phase's requirement to "document
exactly which horizons completed and which did not."

---

## 1. Executive Decision

```text
insufficient_completed_horizon_runs
```

Only 1 of 4 horizons (15m) has a complete full-config run with all required
reports (Ridge, Random Forest, `model_comparison.json`,
`forecast_run_manifest.json`). Per the decision rules, fewer than 4 full
horizon runs completed, so the conservative, rule-compliant decision is
`insufficient_completed_horizon_runs` — regardless of how suggestive the
partial (full-Ridge-only) and smoke-RF data below may look. **This is not a
profitability claim in either direction.** The partial data is still reported
in full below because it is directly useful for prioritizing which horizons
to complete first.

## 2. Run Matrix

| Horizon | Config path | Reports dir | Completed (full) | input_rows | output_rows | window_count | ridge_prediction_count | rf_prediction_count | model_comparison_recommendation |
|---|---|---|---|---|---|---|---|---|---|
| 15m | `config/forecast/btcusdt_1m_h15.toml` | `reports/forecast/btcusdt_1m_h15` | **Yes** | 3,156,480 | 3,156,059 | 20 | 2,592,000 | 2,592,000 | `reject_due_to_cost_adjusted_decay` |
| 30m | `config/forecast/btcusdt_1m_h30.toml` | `reports/forecast/btcusdt_1m_h30` | No (Ridge only; RF did not finish) | 3,156,480 | 3,156,044 | 20 | 2,592,000 | — (SMOKE only, trees=3) | n/a (no `model_comparison.json` written) |
| 1h | `config/forecast/btcusdt_1m_h1h.toml` | `reports/forecast/btcusdt_1m_h1h` | No (Ridge only; RF did not finish) | 3,156,480 | 3,156,014 | 20 | 2,592,000 | — (SMOKE only, trees=3) | n/a (no `model_comparison.json` written) |
| 4h | `config/forecast/btcusdt_1m_h4h.toml` | `reports/forecast/btcusdt_1m_h4h` | No (Ridge only; RF did not finish) | 3,156,480 | 3,155,834 | 20 | 2,592,000 | — (SMOKE only, trees=3) | n/a (no `model_comparison.json` written) |

`model_comparison.json` and `forecast_run_manifest.json` are only written at
the end of a run after every enabled model has been evaluated, so none exist
for the 30m/1h/4h full-config attempts (Random Forest never finished, so the
run never reached that step). This is why those three horizons show `n/a`
above rather than a recommendation string.

## 3. Dataset And Label Comparison

From each horizon's `dataset_summary.json` / `label_summary.json` (all four
are from full, non-smoke runs — the dataset and label are computed before any
model training, so this part of every horizon is fully real):

| Horizon | input_rows | output_rows | skipped_label_horizon | avg_future_return_bps | avg_future_return_after_cost_bps |
|---|---|---|---|---|---|
| 15m | 3,156,480 | 3,156,059 | 15 | 0.17851031 | -13.82148969 |
| 30m | 3,156,480 | 3,156,044 | 30 | 0.35392053 | -13.64607947 |
| 1h | 3,156,480 | 3,156,014 | 60 | 0.70009966 | -13.29990034 |
| 4h | 3,156,480 | 3,155,834 | 240 | 2.76486130 | -11.23513870 |

Interpretation:

- **Raw average forward return grows roughly with horizon**, from 0.18 bps at
  15m to 2.76 bps at 4h — expected, since a longer holding period captures
  more of BTCUSDT's long-run upward drift over 2020–2025.
- **After-cost average becomes less negative as horizon grows**: -13.82 bps
  (15m) → -13.65 bps (30m) → -13.30 bps (1h) → -11.24 bps (4h). The fixed
  round-trip cost (~14 bps) is a shrinking fraction of the total move as the
  horizon lengthens, exactly as the F6 analysis anticipated — but even at 4h
  the after-cost average is still solidly negative.
- **Label-horizon row loss is negligible at every horizon tested**: even at
  4h, only 240 of 3,156,480 rows (0.0076%) are dropped for lacking a full
  forward window. This is not a concern at any of these horizons.

## 4. Ridge Horizon Comparison

All four Ridge results below are **full, real runs** (no smoke config
involved — Ridge trains in well under a minute regardless of horizon).

| Horizon | MAE | RMSE | correlation | directional_accuracy | avg_predicted_bps | avg_actual_after_cost_bps | top_decile_avg_effective_actual_bps | top_decile_hit_rate_effective_target | monotonicity_ratio | top_minus_bottom_spread_bps |
|---|---|---|---|---|---|---|---|---|---|---|
| 15m | 19.92677980 | 33.30022337 | 0.03685246 | 0.77391088 | -13.84612279 | -13.86896870 | -12.71114098 | 0.38776620 | 0.667 (6/9) | 1.591 |
| 30m | 27.90523761 | 46.52136532 | 0.04519512 | 0.71552623 | -13.66367398 | -13.74092480 | -12.03276789 | 0.42408565 | 0.667 (6/9) | 2.284 |
| 1h | 39.43330921 | 64.98835806 | 0.03772052 | 0.66095216 | -13.45108548 | -13.49035954 | -11.96930566 | 0.44913194 | 0.667 (6/9) | 1.854 |
| 4h | 80.97427083 | 128.59820494 | 0.03252265 | 0.57677778 | -11.37245335 | -11.97204414 | -3.67512960 | 0.49075617 | 0.444 (4/9) | 8.035 |

Interpretation:

- **Best horizon for Ridge by top-decile effective return: 4h**, by a wide
  margin (-3.68 bps vs. roughly -12 to -13 bps at 15m/30m/1h). Top-decile
  hit rate at 4h (0.4908) is also the closest to a coin flip of any horizon,
  consistent with a decaying-but-still-negative result.
- **Best horizon by correlation: 30m** (0.0452), only marginally ahead of
  15m (0.0369) and 1h (0.0377); 4h is lowest (0.0325). Correlation does not
  show a clean monotonic trend with horizon — it is roughly flat across all
  four, within a narrow 0.033–0.045 band.
- **Directional accuracy falls steadily with horizon** (0.774 → 0.716 →
  0.661 → 0.577), moving toward 0.5 as horizon grows. This is expected once
  the large constant negative-cost bias (which dominates sign at short
  horizons — see the F6 result-analysis caveat about this metric) shrinks
  relative to the raw return distribution; it is not evidence of the model
  getting worse, and directional accuracy should not be read in isolation
  here.
- **Does longer horizon improve cost-adjusted ranking? Partially and
  unevenly.** Top-minus-bottom spread grows substantially at 4h (8.04 bps vs.
  1.6–2.3 bps at shorter horizons), and monotonicity_ratio is flat at 0.667
  for 15m/30m/1h before **dropping** to 0.444 at 4h — i.e., the 4h bucket
  curve has a much bigger top-decile payoff but is also noisier in between
  (see Section 6 raw bucket values). With only 20 walk-forward windows and a
  4-hour label horizon, individual 4h labels overlap far more with their
  neighbors than 15m labels do, which plausibly inflates bucket-to-bucket
  noise even though the extreme top decile looks the most promising of any
  horizon tested.

## 5. Random Forest Horizon Comparison

**15m is a full, real run (trees=100, max_depth=8). 30m/1h/4h below are SMOKE
runs (trees=3, max_depth=4, local-only, never committed) — directional color
only, not a basis for any decision.**

| Horizon | Run type | MAE | RMSE | correlation | directional_accuracy | avg_predicted_bps | avg_actual_after_cost_bps | top_decile_avg_effective_actual_bps | top_decile_hit_rate_effective_target | monotonicity_ratio | top_minus_bottom_spread_bps |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 15m | **FULL** | 19.95254189 | 33.33399741 | 0.02397112 | 0.77383063 | -13.82422394 | -13.86896870 | -12.98284627 | 0.34087191 | 0.889 (8/9) | 1.370 |
| 30m | SMOKE | 27.91403643 | 46.54764628 | 0.02227797 | 0.71514429 | -13.63867888 | -13.74092480 | -12.08953867 | 0.38111883 | 0.667 (6/9) | 2.046 |
| 1h | SMOKE | 39.45596956 | 65.02193335 | 0.01429049 | 0.66114815 | -13.28215395 | -13.49035954 | -11.44579926 | 0.40749228 | 0.444 (4/9) | 2.049 |
| 4h | SMOKE | 81.14121250 | 128.83093754 | -0.00541690 | 0.56971296 | -10.73884156 | -11.97204414 | -14.90975991 | 0.44931713 | 0.444 (4/9) | **-4.557 (inverted)** |

Top 5 features by split-count importance, per horizon:

| Rank | 15m (FULL, trees=100) | 30m (SMOKE, trees=3) | 1h (SMOKE, trees=3) | 4h (SMOKE, trees=3) |
|---|---|---|---|---|
| 1 | atr_bps (15.38%) | atr_bps (18.56%) | hour_of_day (22.89%) | vwap_distance_bps (29.33%) |
| 2 | vwap_distance_bps (13.57%) | hour_of_day (15.11%) | vwap_distance_bps (19.22%) | hour_of_day (23.33%) |
| 3 | hour_of_day (13.44%) | vwap_distance_bps (15.00%) | atr_bps (18.78%) | day_of_week (22.44%) |
| 4 | day_of_week (12.08%) | day_of_week (12.78%) | day_of_week (16.22%) | atr_bps (19.22%) |
| 5 | ema_8_21_spread_bps (11.48%) | ema_8_21_spread_bps (11.67%) | ema_8_21_spread_bps (9.89%) | ema_8_21_spread_bps (3.22%) |

Interpretation (with the SMOKE caveat firmly in mind for 30m/1h/4h):

- **Does RF improve more at longer horizons? No — if anything the opposite,
  and the 4h smoke result is actively concerning.** At 15m (the only full
  run), RF's bucket ordering (8/9 monotonic) is better than Ridge's (6/9),
  though RF's headline metrics are not better than Ridge's. At 30m/1h SMOKE,
  RF's monotonicity is equal to or worse than Ridge's at the same horizon,
  and correlation is lower. At 4h SMOKE, RF shows a near-zero/slightly
  negative correlation (-0.0054) and an **inverted** top-vs-bottom spread
  (top decile average is *worse* than the bottom decile). Given this is a
  3-tree smoke model, this is at least as likely to be a high-variance
  artifact of severe under-parameterization as a real signal reversal — it
  is exactly the kind of result the smoke-config caveat exists to prevent
  from being over-interpreted, and it must be re-checked with a full
  100-tree run before drawing any conclusion about 4h Random Forest
  specifically.
- **Are dominant features stable across horizons? Partially.** `atr_bps`,
  `vwap_distance_bps`, `hour_of_day`, `day_of_week`, and
  `ema_8_21_spread_bps` occupy the top 5 at every horizon tested — the same
  five features, just reordered. No momentum/microstructure feature
  (`return_1m`, `return_5m`, `return_15m`, `volume_ratio`, `range_position`)
  reaches the top 5 at any horizon.
- **Do time/calendar features dominate all horizons, or only shorter ones?
  They dominate more at longer horizons, not less.** `hour_of_day` +
  `day_of_week` combined account for 25.5% of split usage at 15m (full),
  growing to 27.9% (30m smoke), 39.1% (1h smoke), and 45.8% (4h smoke). This
  is the opposite of "calendar effects only matter short-term" — if this
  pattern holds under full RF runs, it suggests session/day-of-week
  structure becomes relatively more informative as the forecast window
  lengthens, though this needs confirmation with full (non-smoke) runs
  before being trusted, especially at 1h/4h where only 3 trees drove the
  result.
- **Does feature importance shift from microstructure to
  trend/volatility as horizon increases? Yes, directionally.**
  `return_1m`/`return_5m`/`volume_ratio`/`range_position` combined fall from
  a already-modest ~20% of split usage at 15m to under 2% at 4h (smoke),
  while `vwap_distance_bps` (a value-area/trend-distance proxy) rises from
  13.6% to 29.3%. This is a plausible pattern (short-lookback momentum
  features carry less information as the forecast window grows far beyond
  their lookback) but is not causal evidence, and the 30m/1h/4h magnitudes
  specifically come from 3-tree smoke models and should be treated as
  suggestive, not confirmed.

## 6. Cross-Horizon Ranking Power

| Horizon | best_model_by_rmse | best_model_by_correlation | best_model_by_top_decile_return | best_model_name (headline) | best_top_decile_avg_effective_actual_bps | best_correlation | best_monotonicity_ratio | best_top_minus_bottom_spread_bps | recommendation |
|---|---|---|---|---|---|---|---|---|---|
| 15m | ridge | ridge | ridge | ridge | -12.71114098 | 0.03685246 | 0.889 (RF) | 1.591 (ridge) | `reject_due_to_cost_adjusted_decay` (from real `model_comparison.json`) |
| 30m | ridge (only complete model) | ridge (only complete model) | ridge (only complete model) | ridge | -12.03276789 | 0.04519512 | 0.667 | 2.284 | n/a — RF incomplete, no `model_comparison.json` |
| 1h | ridge (only complete model) | ridge (only complete model) | ridge (only complete model) | ridge | -11.96930566 | 0.03772052 | 0.667 | 1.854 | n/a — RF incomplete, no `model_comparison.json` |
| 4h | ridge (only complete model) | ridge (only complete model) | ridge (only complete model) | ridge | -3.67512960 | 0.03252265 | 0.444 | 8.035 | n/a — RF incomplete, no `model_comparison.json` |

For 30m/1h/4h, Ridge is listed as "best" only because it is the *only*
complete model at those horizons — this is not a real model comparison and
should not be read as Ridge beating Random Forest at those horizons.

Interpretation:

- **Does the best horizon survive cost? No.** Every horizon's best available
  top-decile effective return is negative. 4h comes by far the closest to
  zero (-3.68 bps) among real, non-smoke results, but "closest to zero" is
  still not "positive," and `insufficient_completed_horizon_runs` remains
  the correct decision regardless.
- **Is improvement monotonic from 15m → 30m → 1h → 4h? No, not cleanly.**
  Top-decile effective return is roughly flat and negative across
  15m/30m/1h (-12.7, -12.0, -12.0 bps) before jumping sharply less-negative
  at 4h (-3.68 bps). Correlation does not trend monotonically at all across
  the four horizons (0.037 → 0.045 → 0.038 → 0.033). Monotonicity_ratio is
  flat at 0.667 for 15m/30m/1h then drops to 0.444 at 4h.
- **Does longer horizon reduce cost decay? Yes, but unevenly, and only
  clearly so at 4h.** The after-cost drag itself shrinks steadily with
  horizon (Section 3), and Ridge's top-decile result improves sharply at 4h,
  but 30m and 1h do not show a meaningful improvement over 15m on this
  metric.
- **Is any improvement strong enough for ForecastScorer testing? No.** No
  horizon/model combination — full or smoke — produced a positive
  cost-adjusted top-decile return. 4h Ridge is the most encouraging single
  data point in the whole comparison and the clearest candidate for
  follow-up, but it is one data point from one model at one horizon, without
  a completed Random Forest counterpart at that horizon, and with a noisier
  intermediate bucket structure than the shorter horizons. That is not
  sufficient grounds for `candidate_horizon_found`.

## 7. Decision

```text
insufficient_completed_horizon_runs
```

Fewer than 4 full horizon runs completed (only 15m has the complete required
report set). This is a conservative, rule-driven outcome, not a judgment that
the pipeline has failed — the partial data gathered (full Ridge at all four
horizons, plus smoke RF at three) is directly useful and is summarized above.
The most notable single finding is that **Ridge's top-decile cost-adjusted
return is meaningfully less negative at 4h (-3.68 bps) than at 15m/30m/1h
(all roughly -12 to -13 bps)**, though it does not cross zero and the 4h
bucket structure between the extremes is noisier than at shorter horizons.
This is **not a profitability claim** and is not
`candidate_horizon_found`, since no horizon's top-decile result is actually
positive and Random Forest has no completed full run at 4h to corroborate or
contradict the Ridge result.

## 8. Next Phase Recommendation

Per the phase's required ordering, since runs are incomplete, completing the
runs takes priority over further analysis:

1. **Complete the full Random Forest runs for 30m, 1h, and 4h** (`trees =
   100`, `max_depth = 8`, as specified in the committed configs at
   `config/forecast/btcusdt_1m_h30.toml`, `config/forecast/btcusdt_1m_h1h.toml`,
   `config/forecast/btcusdt_1m_h4h.toml`) in an environment that can sustain
   ~40 minutes of continuous single-core compute per horizon (e.g. a
   persistent server, CI runner, or local machine — not this interactive
   sandbox, which could not keep a background or synchronous process alive
   long enough in this session). This alone would resolve the
   `insufficient_completed_horizon_runs` gap for 3 of 4 horizons and turn
   this comparison into one based entirely on real, full-config data.
2. **Prioritize re-running 4h and 1h first** once full-run capacity is
   available: 4h's Ridge result is the most encouraging data point in this
   whole comparison and its Random Forest smoke result (near-zero/negative
   correlation, inverted top-vs-bottom spread) is the most surprising and
   most in need of a real (non-3-tree) check.
3. After all four horizons have complete full-config reports, proceed to
   **regime attribution and cost sensitivity** for whichever horizon(s) show
   the most promising cost-adjusted top-decile behavior (4h is the leading
   candidate based on current partial evidence) — both per this phase's
   ordering rule ("if longer horizons improve but remain negative: cost
   sensitivity and regime attribution") and because the noisier 4h bucket
   structure is exactly the kind of pattern regime attribution (trend vs.
   range, high-vol vs. low-vol) is designed to explain.
4. Do not recommend ForecastScorer integration at this stage — no horizon,
   full or smoke, has produced a positive cost-adjusted top-decile result.

---

## Configuration notes

- Root `config/forecast.toml` is left unchanged (still the 15m default
  preset); it is functionally identical to the new
  `config/forecast/btcusdt_1m_h15.toml` (verified via `diff`, which shows
  only a cosmetic multi-line-vs-single-line array formatting difference, no
  semantic change), so no 15m re-run was needed — the existing
  `reports/forecast/btcusdt_1m_h15/` reports from the F6 phase remain valid
  and current.
- No new Rust helper/analysis code was added. All parsing above was done
  directly against the generated JSON/CSV report files with standard tools;
  given the small size and number of these reports, dedicated helper code
  was not needed for reliable analysis, per this phase's "only if needed"
  allowance.
- Smoke configs and their outputs live entirely under `/tmp` (never part of
  the repository) and are not referenced by any production or test code.

## Language check

This document does not claim guaranteed returns, does not call any horizon
or model combination profitable or ready to trade, and does not claim any
edge has been established or that anything here is fit for live trading. It
uses conservative, evidence-based framing throughout — describing results in
terms of ranking power, noting explicitly where results do not survive cost,
and flagging smoke-config findings as candidates for future regime
attribution and full-run confirmation rather than conclusions — and
repeatedly notes that none of the above is a profitability claim.
