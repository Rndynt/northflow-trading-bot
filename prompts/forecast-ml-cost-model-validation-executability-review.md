# Forecast ML Cost Model Validation & Executability Review Prompt

## Phase

```text
F9 — Cost Model Validation & Executability Review
```

## Context

F8 is complete and produced:

```text
docs/forecast-ml-cost-sensitivity-regime-attribution.md
config/forecast/cost_sensitivity/*.toml
```

F8 decision:

```text
iterate_cost_model_validation_next
```

F8 found that the 4h Ridge candidate becomes positive in the top decile at lower cost assumptions:

```text
14 bps: -3.67512960 bps
10 bps:  0.32487040 bps
7 bps :  3.32487040 bps
5 bps :  5.32487040 bps
```

But it still failed the full candidate gate because monotonicity stayed weak:

```text
monotonicity_ratio = 0.44444444
required >= 0.60
```

Several 7 bps regime subsets were positive, but subset quality was uneven and not stable enough for `ForecastScorer` or backtest-filter integration.

Therefore F9 must answer one narrow question:

```text
Are 7–10 bps round-trip costs realistically achievable for the intended BTCUSDT execution assumptions, or did F8 only become positive under unrealistic cost assumptions?
```

---

## Required Inputs

Read these first:

```text
docs/forecast-ml-roadmap.md
docs/forecast-ml-result-analysis.md
docs/forecast-ml-horizon-comparison.md
docs/forecast-ml-cost-sensitivity-regime-attribution.md
config/forecast/btcusdt_1m_h4h.toml
config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_14bps.toml
config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_10bps.toml
config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_7bps.toml
config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_5bps.toml
```

If committed F8 reports exist, read the small summaries only. Do not commit or depend on large walk-forward CSVs unless explicitly necessary.

---

## Core Objective

Create:

```text
docs/forecast-ml-cost-model-validation.md
```

This document must validate whether the F8 positive 7–10 bps scenarios are executable enough to justify another research pass, or whether the project should reject the current feature/label setup and move to label redesign.

This is not a profitability test. It is a cost realism and executability review.

---

## Non-Negotiable Boundaries

Do not integrate forecast output into strategy.
Do not add `ForecastScorer`.
Do not add an ML backtest filter.
Do not change live/paper trading behavior.
Do not change risk sizing, fill engine, accounting, order simulation, or existing strategy entry/exit logic.
Do not add exchange API calls.
Do not scrape the web from code.
Do not claim profitability.
Do not introduce a production execution module.

Allowed changes:

```text
- documentation
- small cost-model analysis helper under src/forecast if needed
- small static cost table/config under config/forecast if needed
- tests for pure cost calculation utilities if added
```

---

## Required Cost Model Review

F9 must explicitly compare the following execution assumptions:

```text
A. Taker-only execution
B. Maker/taker mixed execution
C. Maker-only optimistic execution
D. Stress/slippage adverse execution
```

For each assumption, estimate total round-trip cost in bps using components:

```text
entry_fee_bps
exit_fee_bps
entry_spread_bps
exit_spread_bps
entry_slippage_bps
exit_slippage_bps
market_impact_bps
round_trip_total_bps
```

At minimum, include scenarios around:

```text
14 bps baseline
10 bps moderate
7 bps optimistic
5 bps aggressive
```

The review must state what each scenario implies operationally. Example:

```text
14 bps: conservative taker/slippage/impact assumption
10 bps: moderate taker or mixed execution assumption
7 bps : optimistic low-slippage/maker-assisted assumption
5 bps : aggressive and likely fragile unless maker execution is reliable
```

Do not invent certainty. If exact venue fees are not configured in the repo, say that the repo lacks venue-specific fee evidence and classify assumptions as hypothetical.

---

## Historical Candle-Derived Proxy Review

If the repository's 1m OHLCV files are available locally, compute lightweight proxies from BTCUSDT 1m candles:

```text
high_low_range_bps
close_to_close_abs_return_bps
open_to_close_abs_return_bps
volume regime bucket
hour_of_day/session bucket
```

Use these only as crude proxies. State limitations clearly:

```text
- OHLCV candles do not provide order book spread
- OHLCV candles do not provide queue position
- OHLCV candles do not prove maker fill probability
- candle ranges overestimate/underestimate actual execution depending on order type
```

If data is not available, do not fake the proxy table. Document that the proxy could not be computed in the current environment and keep the decision based on repo/config evidence only.

---

## Required Comparison Against F8

Tie the cost review directly back to F8 results:

```text
14 bps -> F8 4h Ridge top decile = -3.67512960 bps
10 bps -> F8 4h Ridge top decile =  0.32487040 bps
7 bps  -> F8 4h Ridge top decile =  3.32487040 bps
5 bps  -> F8 4h Ridge top decile =  5.32487040 bps
```

Then answer:

```text
1. Is 10 bps realistically achievable?
2. Is 7 bps realistically achievable?
3. Is 5 bps too optimistic?
4. Is the expected residual edge large enough after cost uncertainty?
```

A tiny positive value such as `0.32487040 bps` at 10 bps must not be treated as robust. The document must account for estimation error.

---

## Required Executability Review

Evaluate whether the current forecast label is executable:

```text
future_return_after_cost_bps over 4h horizon
```

Answer these points:

```text
- Does the label assume entering and exiting at prices that the strategy can actually obtain?
- Does it ignore waiting time, maker fill probability, partial fills, or adverse selection?
- Does 4h horizon create too much overlap between predictions?
- Does the project currently model turnover and trade frequency for this forecast filter?
- Would a positive top-decile bucket survive realistic position lifecycle constraints?
```

If the codebase does not yet model these, state that explicitly.

---

## Required Decision

The final F9 document must choose exactly one:

```text
candidate_for_label_redesign
candidate_for_cost_adjusted_backtest_filter_experiment
reject_current_cost_sensitive_edge
needs_real_fee_slippage_data_before_decision
```

Decision rules:

```text
if 7–10 bps is not defensible from repo evidence:
  reject_current_cost_sensitive_edge

if 10 bps is barely positive and 7 bps needs optimistic assumptions:
  needs_real_fee_slippage_data_before_decision

if 7–10 bps is defensible but monotonicity remains weak:
  candidate_for_label_redesign

if 7–10 bps is defensible and residual edge is meaningfully above uncertainty:
  candidate_for_cost_adjusted_backtest_filter_experiment
```

Be conservative. Do not use `candidate_for_cost_adjusted_backtest_filter_experiment` unless the document gives a strong reason why cost assumptions and residual edge are both robust.

---

## Required Document Structure

Create:

```text
docs/forecast-ml-cost-model-validation.md
```

Required sections:

```text
1. Executive Decision
2. Why F9 exists
3. Inputs reviewed
4. F8 result recap
5. Cost scenario definitions
6. Taker-only cost review
7. Maker/taker mixed cost review
8. Maker-only optimistic cost review
9. Stress/slippage adverse review
10. Candle-derived proxy review
11. Executability review of current label
12. Decision against F8 residual edge
13. Final decision
14. Next phase recommendation
15. Commands run
16. Boundary confirmation
```

Boundary confirmation must explicitly state:

```text
- no strategy logic changed
- no ForecastScorer added
- no backtest filter added
- no paper/live execution added
- no profitability claim made
```

---

## Optional Utility

If helpful, add a tiny pure utility for round-trip cost calculations under `src/forecast/` and test it.

Example pure function behavior:

```text
round_trip_total_bps = entry_fee_bps + exit_fee_bps + entry_spread_bps + exit_spread_bps + entry_slippage_bps + exit_slippage_bps + market_impact_bps
```

Do not wire this into strategy or execution code.

---

## Validation

Run:

```bash
cargo fmt --check
cargo test
```

If no code changed, still run validation if environment has Rust toolchain and data is not required. If unavailable, document that validation could not be run and why.

---

## Git Hygiene

Allowed to commit:

```text
docs/forecast-ml-cost-model-validation.md
small pure helper/tests if added
small static config/table files if added
```

Do not commit:

```text
reports/forecast/**/ridge_walk_forward.csv
reports/forecast/**/random_forest_walk_forward.csv
target/
large CSVs
notebook outputs
```

---

## Acceptance Criteria

F9 is complete only when:

```text
- docs/forecast-ml-cost-model-validation.md exists
- decision is explicit and conservative
- 7/10 bps cost realism is directly judged against F8 results
- current 4h forecast label executability is reviewed
- no ForecastScorer/backtest/strategy integration is added
- no profitability claim is made
- cargo fmt --check and cargo test status are documented
```
