# Phase: `vwap_reclaim_short_v2` Structure Stop + Breakdown Quality

## Objective

Create `vwap_reclaim_short_v2` as a separate strategy version. Do not mutate or delete `vwap_reclaim_short_v1` because v1 is now a baseline failure case.

The goal is to test whether the v1 failure is caused by:

1. ATR-only stop being too close to noise.
2. Breakdown/retest validation being too weak.
3. Too many entries during high-volatility whipsaw regimes.
4. Missing diagnostics for candidate rejection stages.

This phase must run through the existing main backtest engine. Do not create a standalone runner.

## Evidence from v1 Report

`reports/vwap_reclaim_short_v1_2020_2025/backtest_summary.json`:

- 308 trades
- 24.03% win rate
- -3299.34 net PnL
- 0.2128 profit factor
- 65.99% max drawdown

`attribution_by_exit_reason.csv`:

- stop_loss: 234 trades, 0 wins, -4191.41 net PnL
- take_profit: 74 trades, +892.07 net PnL

So v1 is not just losing from fees. Gross PnL is negative and stop losses dominate.

`cost_edge_distribution.csv`:

- all edge buckets are negative.
- the highest expected edge bucket `edge_gte_50` is also negative.

This means expected edge is badly overestimated by the strategy logic.

## Hard Rules

- No standalone runner.
- No hardcoded timeframes in engine logic.
- Use strategy versioning: keep `vwap_reclaim_short_v1`; add `vwap_reclaim_short_v2`.
- Do not change risk model, fill model, attribution report writer, or engine accounting semantics.
- Do not claim profitability.
- Strategy-specific logic must stay inside the strategy module.
- Reuse `MultiTimeframeInput.entry_lookback`.

## Files

Add:

```text
src/strategy/vwap_reclaim_short_v2.rs
config/research_vwap_reclaim_short_v2_2020_2025.toml
```

Modify:

```text
src/strategy/mod.rs
src/backtest/engine.rs
src/config/mod.rs
```

## Strategy ID

```text
vwap_reclaim_short_v2
```

## Config Prefix

Use `vrs2_` prefix.

Suggested config struct:

```rust
#[derive(Debug, Clone)]
pub struct VwapReclaimShortV2Config {
    pub lookback_bars: usize,
    pub breakout_window_bars: usize,
    pub retest_tolerance_atr: f64,
    pub max_extension_atr: f64,
    pub min_volume_ratio: f64,
    pub min_atr_bps: f64,
    pub max_atr_bps: f64,
    pub min_reward_risk: f64,
    pub min_expected_reward_bps: f64,
    pub min_expected_net_edge_bps: f64,
    pub cooldown_bars: u64,

    pub max_anchor_range_atr: f64,
    pub min_breakdown_close_atr: f64,
    pub max_breakdown_close_atr: f64,
    pub require_breakdown_volume_expansion: bool,
    pub min_breakdown_volume_ratio: f64,

    pub stop_mode: String, // "atr" or "structure"
    pub sl_atr_multiple: f64,
    pub tp_atr_multiple: f64,
    pub structure_stop_buffer_atr: f64,
    pub max_structure_stop_atr: f64,

    pub require_current_bearish_body: bool,
    pub min_current_body_ratio: f64,
    pub min_upper_wick_rejection_ratio: f64,
}
```

## Required Improvements vs v1

### 1. Anchor range quality

Compute `anchor_range_high`, `anchor_range_low`, and `anchor_range_width_atr` from the anchor range.

Reject if:

```text
anchor_range_width_atr > max_anchor_range_atr
```

Purpose: avoid trading breakdowns from already-chaotic structures.

### 2. Breakdown quality

Find the latest breakdown candle in the recent window:

```text
close < anchor_range_low
```

Compute:

```text
breakdown_close_atr = (anchor_range_low - breakdown_close) / atr
```

Require:

```text
min_breakdown_close_atr <= breakdown_close_atr <= max_breakdown_close_atr
```

Purpose:

- too small = fake breakdown/noise
- too large = chasing panic after breakdown already extended

If `require_breakdown_volume_expansion = true`, require:

```text
breakdown_volume / current_volume_sma_20 >= min_breakdown_volume_ratio
```

### 3. Retest quality

Current candle must still pass v1 retest hold:

```text
current high >= anchor_range_low - retest_tolerance_atr * ATR
current close < anchor_range_low
current close < current open
```

Additional optional body/wick checks:

```text
body_ratio >= min_current_body_ratio
upper_wick_ratio >= min_upper_wick_rejection_ratio
```

Only enforce those if configs require them.

### 4. Structure stop

This is the key change.

V1 used:

```text
stop_loss = entry + ATR * sl_atr_multiple
```

V2 should support:

```text
stop_mode = "atr"
stop_loss = entry + ATR * sl_atr_multiple
```

or:

```text
stop_mode = "structure"
structure_stop = max(current_high, recent_window_high, anchor_range_low) + ATR * structure_stop_buffer_atr
```

Reject if structure stop distance is too large:

```text
(stop_loss - entry) / ATR > max_structure_stop_atr
```

Purpose: stop should sit beyond the retest invalidation area, not inside normal 1m noise.

### 5. Take profit

Keep TP ATR-based first:

```text
take_profit = entry - ATR * tp_atr_multiple
```

Do not add trailing stop in this phase.

### 6. Filters passed

Add granular filters so reports are more useful:

```text
anchor_range_quality_ok
breakdown_close_depth_ok
breakdown_volume_ok
retest_hold_ok
current_body_ok
upper_wick_rejection_ok
structure_stop_ok
below_vwap
atr_bps_in_range
volume_ratio_ok
reward_risk_ok
expected_reward_ok
expected_net_edge_ok
```

## Important Diagnostic Requirement

Current `attribution_by_filter.csv` is not useful because it only records filters for opened trades, so every row has the same performance.

Add strategy-level debug filters to `filters_passed`, but do not modify report writer in this phase.

If broader rejected-candidate diagnostics are needed, create a separate future prompt. Do not expand engine/reporting now.

## Example Config

Create:

```text
config/research_vwap_reclaim_short_v2_2020_2025.toml
```

Use multi-file BTCUSDT historical files and:

```toml
[strategy]
strategy_id = "vwap_reclaim_short_v2"
min_confidence = 70

vrs2_lookback_bars = 80
vrs2_breakout_window_bars = 12
vrs2_retest_tolerance_atr = 0.20
vrs2_max_extension_atr = 0.60
vrs2_min_volume_ratio = 1.0
vrs2_min_atr_bps = 8.0
vrs2_max_atr_bps = 35.0
vrs2_min_reward_risk = 1.8
vrs2_min_expected_reward_bps = 20.0
vrs2_min_expected_net_edge_bps = 6.0
vrs2_cooldown_bars = 6

vrs2_max_anchor_range_atr = 6.0
vrs2_min_breakdown_close_atr = 0.15
vrs2_max_breakdown_close_atr = 1.20
vrs2_require_breakdown_volume_expansion = true
vrs2_min_breakdown_volume_ratio = 1.2

vrs2_stop_mode = "structure"
vrs2_sl_atr_multiple = 1.2
vrs2_tp_atr_multiple = 2.2
vrs2_structure_stop_buffer_atr = 0.20
vrs2_max_structure_stop_atr = 2.0

vrs2_require_current_bearish_body = true
vrs2_min_current_body_ratio = 0.35
vrs2_min_upper_wick_rejection_ratio = 0.15

[backtest]
reports_dir = "reports/vwap_reclaim_short_v2_2020_2025"
entry_lookback_bars = 120
```

## Tests

Add tests for:

1. strategy ID stable.
2. no signal if lookback too short.
3. no signal if anchor range too wide.
4. no signal if breakdown depth too shallow.
5. no signal if breakdown depth too extended.
6. no signal if breakdown volume required but absent.
7. no signal if retest hold absent.
8. no signal if structure stop is too wide.
9. emits short signal when all filters pass.
10. emitted short signal has valid geometry.
11. emitted signal uses timeframe roles from `StrategyContext`.

## Validation

Run:

```bash
cargo fmt
cargo test
cargo run --release -- research --config config/research_vwap_reclaim_short_v2_2020_2025.toml
```

Commit:

```bash
git add src config docs reports/vwap_reclaim_short_v2_2020_2025
git commit -m "strategy: add vwap reclaim short v2 structure stop"
```

## Evaluation Criteria

After backtest, compare v2 to v1:

- trade count should be much lower than 308 unless win rate materially improves.
- win rate must improve above 30% for this R:R family to be interesting.
- profit factor must move materially above v1's 0.21.
- max drawdown should drop far below 66%.
- if gross PnL is still negative, stop this family and move to a different hypothesis.
