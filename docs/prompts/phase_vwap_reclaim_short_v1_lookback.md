# Phase: `vwap_reclaim_short_v1` Lookback-Aware Strategy Module

## Objective

Implement a dedicated `vwap_reclaim_short_v1` strategy module that uses the main engine's generic `entry_lookback` input.

This phase must not create any standalone runner. The strategy must run through the existing main backtest engine, risk model, fill model, and Phase 7 report pipeline.

## Context

Previous research found that the old short candidate was only an `ema_trend_pullback_v1` config preset using VWAP pullback/reclaim behavior. On a 2020–2025 multi-file test, it produced only 7 trades and negative net PnL. Therefore it is not a robust final strategy.

The codebase now supports:

- multi-file historical data via `[historical_files]`
- dynamic timeframe roles from config
- `MultiTimeframeInput.entry_lookback`
- `entry_lookback_bars` from config

Use those capabilities to implement a clean dedicated short strategy.

## Hard Rules

- No standalone runner.
- No hardcoded `1m`, `5m`, or `15m` in engine logic.
- No hardcoded symbols or years.
- Strategy must use role names: entry, confirmation, screening.
- Strategy-specific market-structure logic belongs inside the strategy module, not inside the engine.
- Do not mutate existing strategies except for registration and config plumbing.
- Do not change risk, fill, attribution, or reporting semantics.
- Do not claim profitability.

## Files to Add / Modify

Add:

```text
src/strategy/vwap_reclaim_short.rs
```

Modify:

```text
src/strategy/mod.rs
src/backtest/engine.rs
src/config/mod.rs
config/research_vwap_reclaim_short_v1_2020_2025.toml
```

Add tests in the new strategy module and any registration tests needed in engine/config.

## Strategy ID

Use stable strategy ID:

```text
vwap_reclaim_short_v1
```

Register it anywhere strategy IDs are validated or instantiated.

## Config

Add config struct:

```rust
#[derive(Debug, Clone)]
pub struct VwapReclaimShortConfig {
    pub lookback_bars: usize,
    pub breakout_window_bars: usize,
    pub retest_tolerance_atr: f64,
    pub max_extension_atr: f64,
    pub min_volume_ratio: f64,
    pub min_atr_bps: f64,
    pub max_atr_bps: f64,
    pub sl_atr_multiple: f64,
    pub tp_atr_multiple: f64,
    pub min_reward_risk: f64,
    pub min_expected_reward_bps: f64,
    pub min_expected_net_edge_bps: f64,
    pub cooldown_bars: u64,
}
```

TOML keys should use `vrs_` prefix:

```toml
[strategy]
strategy_id = "vwap_reclaim_short_v1"
min_confidence = 70

vrs_lookback_bars = 50
vrs_breakout_window_bars = 8
vrs_retest_tolerance_atr = 0.25
vrs_max_extension_atr = 0.80
vrs_min_volume_ratio = 1.0
vrs_min_atr_bps = 8.0
vrs_max_atr_bps = 45.0
vrs_sl_atr_multiple = 1.0
vrs_tp_atr_multiple = 2.5
vrs_min_reward_risk = 2.0
vrs_min_expected_reward_bps = 25.0
vrs_min_expected_net_edge_bps = 8.0
vrs_cooldown_bars = 0

[backtest]
entry_lookback_bars = 80
```

Rules:

- `entry_lookback_bars` must be at least enough for `vrs_lookback_bars + vrs_breakout_window_bars`.
- Validation should reject impossible numeric values.
- Keep old configs working.

## Required Strategy Logic

The strategy is short-only.

### 1. Validate candles and required indicators

Use:

- entry candle
- confirmation candle
- screening candle
- entry indicators
- confirmation indicators
- screening indicators
- entry lookback candles

Required indicators:

Entry:

- EMA 8
- EMA 21
- EMA 50
- EMA 200 for warmup if already standard
- ATR 14
- VWAP
- Volume SMA 20

Confirmation:

- EMA 21
- EMA 50
- EMA 200

Screening:

- EMA 50
- EMA 200

### 2. Regime filter

Short only when screening role is bearish:

```text
screening EMA50 < EMA200
screening close < EMA50
```

### 3. Confirmation filter

Short confirmation:

```text
confirmation EMA21 < EMA50
confirmation EMA50 < EMA200
confirmation close < EMA21
```

### 4. Entry alignment

Entry role must support short bias:

```text
EMA8 < EMA21
EMA21 < EMA50
entry close <= EMA21
```

### 5. Lookback market structure

Use only `input.entry_lookback`. The current `entry_candle` is not part of the lookback.

Split lookback:

```text
anchor_range = older lookback portion before recent window
recent_window = most recent `breakout_window_bars` candles before current candle
```

For short setup:

1. Determine recent support / range low from `anchor_range`:

```text
range_low = min(low over anchor_range)
```

2. Require recent breakdown:

```text
some close in recent_window < range_low
```

3. Current candle is bearish retest / rejection:

```text
current high >= range_low - retest_tolerance_atr * ATR
current close < range_low
current close < current open
```

4. Avoid chasing too far below level:

```text
extension_atr = (range_low - current close) / ATR
0 <= extension_atr <= max_extension_atr
```

5. Require current candle near or below VWAP context:

A conservative first version may require:

```text
current close < VWAP
```

Do not hardcode a VWAP distance unless config exposes it.

### 6. Volatility / volume / cost filters

Use config:

```text
ATR bps in [min_atr_bps, max_atr_bps]
volume / volume_sma_20 >= min_volume_ratio
expected_reward_bps >= min_expected_reward_bps
expected_net_edge_bps >= min_expected_net_edge_bps
reward/risk >= min_reward_risk
```

Expected cost comes from `ctx.estimated_cost_bps`.

### 7. Signal geometry

Short signal:

```text
entry = entry_close
stop_loss = entry + ATR * sl_atr_multiple
take_profit = entry - ATR * tp_atr_multiple
```

Require valid short geometry:

```text
take_profit < entry < stop_loss
```

### 8. Signal metadata

Use deterministic signal ID pattern consistent with existing strategies:

```rust
SignalId::new(format!("SIG-BT-{:08X}", ctx.signal_index))
```

Use strategy ID:

```text
vwap_reclaim_short_v1
```

Suggested filters passed:

```text
screening_bearish
confirmation_bearish
entry_ema_alignment_short
lookback_range_low_breakdown
bearish_retest_hold
below_vwap
atr_bps_in_range
volume_ratio_ok
reward_risk_ok
expected_reward_ok
expected_net_edge_ok
confidence_ok
```

Entry reason example:

```text
vwap_reclaim_short_v1_range_breakdown_retest
```

## Engine Registration

Modify `ActiveStrategy` in `src/backtest/engine.rs`:

- add variant for `VwapReclaimShortV1`
- instantiate when `strategy_id == "vwap_reclaim_short_v1"`
- delegate evaluate

No engine-specific breakdown/retest logic should be added.

## Config Validation

Update `validate_strategy_config()` to accept the new strategy ID and validate VRS config values.

Add:

```rust
pub fn vrs_config(&self) -> VwapReclaimShortConfig
```

Update cooldown:

```rust
"vwap_reclaim_short_v1" => self.vrs_cooldown_bars
```

## Example Config

Create:

```text
config/research_vwap_reclaim_short_v1_2020_2025.toml
```

Use the existing multi-file historical layout:

```toml
[historical_files]
BTCUSDT = [
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2022.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2023.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2024.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2025.csv",
]
```

Set:

```toml
[backtest]
reports_dir = "reports/vwap_reclaim_short_v1_2020_2025"
entry_lookback_bars = 80
```

## Tests

Add unit tests for:

1. strategy ID is stable.
2. no signal if entry_lookback is too short.
3. no signal if screening is not bearish.
4. no signal if confirmation is not bearish.
5. no signal if recent breakdown is absent.
6. no signal if current candle does not retest/hold below range low.
7. emits short signal when all filters pass.
8. emitted short signal has valid geometry.
9. emitted signal uses configured timeframe roles from `StrategyContext`.
10. strategy ignores future/current candles because it only reads `entry_lookback` plus current entry candle.

## Validation Commands

Run:

```bash
cargo fmt
cargo test
cargo run --release -- research --config config/research_vwap_reclaim_short_v1_2020_2025.toml
```

Commit:

```bash
git add src config docs
git commit -m "strategy: add lookback vwap reclaim short v1"
```

## Expected Result Reporting

After running the backtest, report:

- total trades
- win rate
- net PnL
- profit factor
- max drawdown
- attribution by regime / side / filter
- whether the strategy is promising, inconclusive, or failed

Do not claim profitability. This is historical research only.
