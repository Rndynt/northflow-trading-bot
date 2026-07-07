# Forecast ML Cost Sensitivity & Regime Attribution (F8)

## 1. Executive Decision

**Decision: `iterate_cost_model_validation_next`.**

The 4h Ridge candidate becomes positive in the top decile at 10 bps, 7 bps, and 5 bps round-trip costs, but it does **not** pass the full candidate gate because bucket monotonicity remains weak at `0.44444444`, below the required `0.60` threshold. Regime attribution on the 7 bps report found several positive simple subsets, but the global gate failure and clear regime instability make this unsuitable for ForecastScorer or backtest-filter integration.

This result should be treated only as research evidence that the label/cost boundary is sensitive to cost assumptions. It is **not** a profitability claim.

## 2. Why F8 exists

F7 completed full Random Forest reports for 30m, 1h, and 4h horizons and chose `iterate_cost_sensitivity_next`. The best F7 candidate was the 4h Ridge ranking result, not Random Forest:

```text
4h Ridge top-decile effective actual return = -3.67512960 bps
```

That result was negative after the configured 14 bps round-trip cost. F8 therefore tests whether the candidate fails only because the baseline cost model is too conservative, or whether it also fails under lower realistic cost assumptions. It then checks whether simple market regime subsets create a stable, positive top-decile edge after cost.

## 3. Inputs used

Reference documents and reports reviewed:

- `docs/forecast-ml-roadmap.md`
- `docs/forecast-ml-implementation-report.md`
- `docs/forecast-ml-result-analysis.md`
- `docs/forecast-ml-horizon-comparison.md`
- `reports/forecast/btcusdt_1m_h30/model_comparison.json`
- `reports/forecast/btcusdt_1m_h1h/model_comparison.json`
- `reports/forecast/btcusdt_1m_h4h/model_comparison.json`
- `config/forecast/btcusdt_1m_h4h.toml`

Cost sensitivity used Ridge-only 4h configs. This was intentional because F7 already showed Ridge beat Random Forest at 4h by RMSE, correlation, and top-decile return; F8 tests the best candidate first instead of rerunning the weaker, slower model.

## 4. Cost sensitivity configs

Created Ridge-only configs under `config/forecast/cost_sensitivity/` using the same 4h setup as `config/forecast/btcusdt_1m_h4h.toml`, changing only cost assumptions, enabled model list, and report directory.

| Config | Round-trip cost | taker_fee_bps | slippage_bps | spread_bps | market_impact_bps | Reports dir |
|---|---:|---:|---:|---:|---:|---|
| `btcusdt_1m_h4h_cost_14bps.toml` | 14 bps | 4.0 | 2.0 | 1.0 | 1.0 | `reports/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_14bps` |
| `btcusdt_1m_h4h_cost_10bps.toml` | 10 bps | 3.0 | 1.5 | 1.0 | 0.0 | `reports/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_10bps` |
| `btcusdt_1m_h4h_cost_7bps.toml` | 7 bps | 2.0 | 1.0 | 1.0 | 0.0 | `reports/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_7bps` |
| `btcusdt_1m_h4h_cost_5bps.toml` | 5 bps | 1.5 | 0.5 | 1.0 | 0.0 | `reports/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_5bps` |

Unchanged setup across all configs:

- Symbol: `BTCUSDT`
- Source timeframe: `1m`
- Entry timeframe: `1m`
- Forecast horizon: `4h`
- Horizon bars: `240`
- Historical data: 2020 through 2025 BTCUSDT 1m CSVs
- Features: unchanged from the 4h baseline
- Ridge alpha: `1.0`
- Ridge standardization: enabled
- Walk-forward: 12 train months, 3 test months, 3 step months, 240 embargo bars

## 5. Ridge cost sensitivity table

Candidate gate used:

```text
top_decile_avg_effective_actual_bps > 0
correlation > 0
monotonicity_ratio >= 0.60
top_minus_bottom_spread_bps > 0
```

| cost_bps | MAE | RMSE | correlation | directional_accuracy | avg_actual_after_cost_bps | top_decile_avg_effective_actual_bps | top_decile_hit_rate_effective_target | bottom_decile_avg_effective_actual_bps | top_minus_bottom_spread_bps | monotonicity_ratio | report recommendation | passes_candidate_gate |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| 14 | 80.97427083 | 128.59820494 | 0.03252265 | 0.57677778 | -11.97204414 | -3.67512960 | 0.49075617 | -11.71038237 | 8.03525277 | 0.44444444 | `reject_due_to_cost_adjusted_decay` | false |
| 10 | 80.97427083 | 128.59820494 | 0.03252265 | 0.55244522 | -7.97204414 | 0.32487040 | 0.50109954 | -7.71038237 | 8.03525277 | 0.44444444 | `candidate_for_backtest_filter_phase` | false |
| 7 | 80.97427083 | 128.59820494 | 0.03252265 | 0.53619753 | -4.97204414 | 3.32487040 | 0.50870370 | -4.71038237 | 8.03525277 | 0.44444444 | `candidate_for_backtest_filter_phase` | false |
| 5 | 80.97427083 | 128.59820494 | 0.03252265 | 0.52492284 | -2.97204414 | 5.32487040 | 0.51364198 | -2.71038237 | 8.03525277 | 0.44444444 | `candidate_for_backtest_filter_phase` | false |

Notes:

- MAE, RMSE, and correlation are identical because lowering cost shifts both the cost-adjusted target and Ridge predictions by the same constant under this label setup.
- Top-minus-bottom spread is positive in every scenario.
- The top decile turns positive at 10 bps and remains positive at 7 bps and 5 bps.
- Monotonicity remains below the required threshold in every scenario, so no cost scenario passes the full F8 candidate gate.

## 6. Candidate gate result

The cost sensitivity result is mixed:

- At 14 bps, top-decile effective actual return is still negative.
- At 10 bps, top-decile effective actual return becomes barely positive at `0.32487040` bps.
- At 7 bps, top-decile effective actual return improves to `3.32487040` bps.
- At 5 bps, top-decile effective actual return improves to `5.32487040` bps.
- However, monotonicity is only `0.44444444` in all scenarios, below the required `0.60` candidate-gate threshold.

Therefore, the Ridge candidate is **not** eligible for ForecastScorer/backtest-filter integration. The positive 7-10 bps result is best interpreted as cost-model-sensitive research evidence, not a robust model candidate.

## 7. Regime attribution results

Regime attribution used the 7 bps Ridge report because it is the lowest optimistic-but-plausible scenario requested and the first scenario with a materially positive top decile. The helper rebuilt feature values from the configured dataset and joined them to Ridge walk-forward predictions by timestamp.

Regime subset gate here is intentionally narrow and diagnostic only:

```text
top_decile_avg_effective_actual_bps > 0
correlation > 0
```

| regime_name | subset | row_count | top_decile_avg_effective_actual_bps | top_decile_hit_rate_effective_target | correlation | passes_candidate_gate |
|---|---|---:|---:|---:|---:|---|
| trend_proxy | `ema_8_21_spread_bps > 0` | 1,306,514 | 1.34286230 | 0.49270581 | 0.01897948 | true |
| trend_proxy | `ema_8_21_spread_bps <= 0` | 1,285,486 | 6.58269148 | 0.52008184 | 0.04360854 | true |
| volatility_proxy | `atr_bps top 30%` | 777,600 | 10.03744692 | 0.52184928 | 0.04759198 | true |
| volatility_proxy | `atr_bps bottom 30%` | 777,601 | -5.11040974 | 0.44950554 | 0.01142197 | false |
| value_distance_proxy | `vwap_distance_bps positive` | 1,961,080 | 8.63342918 | 0.51882126 | 0.04293389 | true |
| value_distance_proxy | `vwap_distance_bps negative` | 630,920 | -21.80331906 | 0.44238572 | -0.02007187 | false |
| session_proxy | `Asia` | 864,107 | -3.20875975 | 0.50045712 | 0.00966751 | false |
| session_proxy | `London` | 540,147 | 6.80161644 | 0.52017032 | 0.04420346 | true |
| session_proxy | `US/NY` | 863,789 | 9.82463781 | 0.52280068 | 0.05112986 | true |
| session_proxy | `Other` | 323,957 | -0.22241149 | 0.48805408 | 0.00597946 | false |

## 8. Interpretation

The model is cost-sensitive. The 14 bps baseline makes the best 4h Ridge top decile negative, while 10 bps and lower costs make it positive. That means the prior F7 failure is partly explained by cost assumptions.

However, the evidence is not strong enough for integration:

- Monotonicity is weak across all cost scenarios.
- The 10 bps top-decile result is only slightly above zero.
- Positive regime subsets are not uniformly stable; low-volatility, negative VWAP-distance, Asia, and Other-session subsets remain weak or negative.
- The strongest subsets appear concentrated in higher ATR, positive VWAP-distance, London, and US/NY periods, but this is diagnostic attribution only and not a production rule.
- The analysis does not test executable trade filters, turnover, fill realism, or interaction with the existing strategy and risk model.

## 9. Final decision

**Final decision: `iterate_cost_model_validation_next`.**

Rationale:

- 7-10 bps costs can make the top decile positive.
- Regime attribution finds several positive subsets.
- But the full candidate gate fails because monotonicity is below threshold.
- Subset quality is uneven, so it is too early to call this a stable regime-scoped forecast filter candidate.
- The next research step should validate whether 7-10 bps is actually achievable for the intended execution assumptions before changing label design or attempting any filter integration.

## 10. Next phase recommendation

Recommended next phase:

```text
F9 — Cost Model Validation & Executability Review
```

Suggested scope:

1. Validate realistic BTCUSDT round-trip costs for the intended venue/order type assumptions.
2. Compare taker-only versus maker/taker or maker-only assumptions without changing strategy behavior.
3. Measure spread/slippage assumptions from historical candle-derived proxies if available, while explicitly documenting limitations.
4. Keep forecasts disconnected from strategy, risk, accounting, fills, and backtest engine.
5. Only after cost validation, revisit whether the label should use a cost model closer to achievable execution.

## 11. Commands run

```bash
cargo run --release -- forecast --config config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_14bps.toml
cargo run --release -- forecast --config config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_10bps.toml
cargo run --release -- forecast --config config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_7bps.toml
cargo run --release -- forecast --config config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_5bps.toml
```

Validation commands:

```bash
cargo fmt --check
cargo test
```

Additional local analysis command used to extract tables:

```bash
python3 <inline analysis script reading ridge_summary.json, ridge_prediction_buckets.csv, ridge_walk_forward.csv, and BTCUSDT 1m CSVs>
```

## 12. Boundary confirmation

- no strategy logic changed
- no backtest engine integration added
- no ForecastScorer added
- no paper/live enabled
- no profitability claim made
