# Northflow Screened VWAP Scalp V2 Research Patch Prompt

You are working on this repository:

https://github.com/Rndynt/northflow-crypto-trading-bot

Your task is to implement a focused **strategy research patch** after Phase 7, after risk rejection attribution, after entry geometry modes, and after strategy diagnostic reports.

Do not implement a new phase.

Do not implement paper trading.

Do not implement live trading.

Do not implement exchange APIs.

Do not implement websocket feeds.

Do not implement dashboard, Telegram, LLM trading decisions, AI advisor, optimizer, or auto-tuning.

This patch adds a second deterministic strategy variant for controlled research comparison:

```text
screened_vwap_scalp_v2
```

The current strategy must remain intact:

```text
screened_vwap_scalp
```

Do not delete, rewrite, or silently change v1 behavior.

## Current diagnostic result

The latest diagnostic run used:

```toml
[backtest]
entry_geometry_mode = "reanchor_to_actual_entry"
reports_dir = "reports/reanchor"

[risk]
max_drawdown_pct = 100.0
max_daily_loss_pct = 100.0
```

BTCUSDT 1m 2024 result:

```text
Total trades:              4761
Winning trades:            1702
Losing trades:             3059
Win rate:                  35.748792%
Gross PnL:                 -796.835231
Fee:                       1956.647751
Slippage:                  2246.516284
Total cost:                4203.164035
Net PnL:                   -4999.999266
Avg expected edge bps:     12.173674
Avg actual edge bps:       -20.963742
Avg edge realization bps:  -33.137415
Avg total cost bps:        17.211149
Dominant rejection reason: expected_net_edge_not_positive
Dominant rejection count:  60121
```

Interpretation:

- Engine is valid.
- Data pipeline is valid.
- Reanchor geometry is valid.
- Diagnostic reports are valid.
- Current strategy `screened_vwap_scalp` is deeply unprofitable.
- It loses even before cost, because gross PnL is negative.
- Cost makes the loss much worse.
- Current expected edge is not predictive enough.
- Many signals are rejected because expected net edge is not positive.
- Trades that pass still have low win rate and high cost drag.

## Goal

Add a deterministic, configurable v2 strategy for research comparison.

The goal is **not** to guarantee profit.

The goal is to test whether a stricter, cost-aware version of the same idea can reduce bad signals and improve diagnostics.

The strategy must remain deterministic and rule-based.

No LLM decision-making.

No optimizer.

No random behavior.

No lookahead.

## Strategy IDs

Existing v1 strategy ID must remain:

```text
screened_vwap_scalp
```

Add new v2 strategy ID:

```text
screened_vwap_scalp_v2
```

Backtest must select the active strategy from config.

## Required config

Update `config/research.toml` and the config parser.

Add:

```toml
[strategy]
strategy_id = "screened_vwap_scalp_v2"

# Existing common strategy setting
min_confidence = 70

# V2 filters
v2_require_strict_confirmation = true
v2_require_ema_ribbon_alignment = true
v2_allow_neutral_confirmation = false
v2_min_expected_reward_bps = 25.0
v2_min_expected_net_edge_bps = 10.0
v2_min_atr_bps = 8.0
v2_max_atr_bps = 120.0
v2_tp_atr_multiple = 2.5
v2_sl_atr_multiple = 1.0
v2_min_volume_ratio = 1.0
v2_vwap_distance_atr_min = 0.0
v2_vwap_distance_atr_max = 1.5
v2_cooldown_bars = 5
v2_enable_long = true
v2_enable_short = true
```

Defaults must be safe if fields are missing.

Recommended defaults:

```text
strategy_id = "screened_vwap_scalp"
v2_require_strict_confirmation = true
v2_require_ema_ribbon_alignment = true
v2_allow_neutral_confirmation = false
v2_min_expected_reward_bps = 20.0
v2_min_expected_net_edge_bps = 5.0
v2_min_atr_bps = 5.0
v2_max_atr_bps = 150.0
v2_tp_atr_multiple = 2.0
v2_sl_atr_multiple = 1.0
v2_min_volume_ratio = 1.0
v2_vwap_distance_atr_min = 0.0
v2_vwap_distance_atr_max = 2.0
v2_cooldown_bars = 0
v2_enable_long = true
v2_enable_short = true
```

Validation:

- Unknown `strategy_id` must return config error.
- Invalid numeric config must return config error.
- `v2_tp_atr_multiple` must be finite and > 0.
- `v2_sl_atr_multiple` must be finite and > 0.
- `v2_min_expected_reward_bps` must be finite and >= 0.
- `v2_min_expected_net_edge_bps` must be finite and >= 0.
- `v2_min_atr_bps` must be finite and >= 0.
- `v2_max_atr_bps` must be finite and > `v2_min_atr_bps`.
- `v2_min_volume_ratio` must be finite and >= 0.
- `v2_vwap_distance_atr_min` must be finite and >= 0.
- `v2_vwap_distance_atr_max` must be finite and >= `v2_vwap_distance_atr_min`.
- `v2_cooldown_bars` must be >= 0.

## Files to read first

Read these files before changing anything:

- AGENTS.md
- docs/ROADMAP.md
- README.md
- docs/DATA_DOWNLOAD.md
- docs/STRATEGY_RESEARCH.md if it already exists
- config/research.toml
- src/config/mod.rs
- src/strategy/mod.rs
- src/strategy/screened_vwap_scalp.rs
- src/backtest/engine.rs
- src/backtest/geometry.rs
- src/backtest/risk_trace.rs
- src/backtest/report.rs
- src/report/diagnostics.rs
- src/report/attribution.rs
- src/report/manifest.rs
- src/research/mod.rs
- src/indicators/snapshot.rs
- src/core/signal.rs
- src/core/side.rs
- src/core/timeframe.rs
- src/core/candle.rs

## Required structure

Add:

```text
src/strategy/screened_vwap_scalp_v2.rs
```

Update:

```text
src/strategy/mod.rs
```

Recommended naming:

```rust
ScreenedVwapScalp
ScreenedVwapScalpV2
```

Avoid name collisions with v1.

## Strategy selection

If `BacktestEngine` currently hardcodes v1, change it to select strategy from config.

Allowed implementation options:

1. Simple match in `BacktestEngine` on `cfg.strategy_id`.
2. Small strategy factory function.
3. Enum-based strategy selector.

Keep it simple.

Do not introduce complex dynamic dispatch unless needed.

Unknown strategy ID must return `NorthflowError::ConfigError`.

## V2 strategy concept

V2 keeps the same multi-timeframe structure:

```text
15m = screening / regime
5m  = confirmation
1m  = entry
```

But it adds stricter, cost-aware filters.

### Indicator requirements

If required indicators are missing, return `Ok(None)`, not error.

Required entry timeframe fields:

```text
ema_8
ema_21
ema_50
atr_14
vwap
volume_sma_20
```

Required confirmation timeframe fields:

```text
ema_50
ema_200
close
```

Required screening timeframe fields:

```text
ema_50
ema_200
close
```

Use existing `IndicatorSnapshot` fields.

Do not panic on missing indicator values.

### Screening regime

From 15m:

Bullish:

```text
ema_50 > ema_200
close > ema_50
```

Bearish:

```text
ema_50 < ema_200
close < ema_50
```

Otherwise neutral / unknown.

V2 must not trade neutral screening regime.

### Confirmation

Default strict confirmation:

Long requires 5m bullish:

```text
confirmation ema_50 > ema_200
confirmation close > ema_50
```

Short requires 5m bearish:

```text
confirmation ema_50 < ema_200
confirmation close < ema_50
```

If `v2_allow_neutral_confirmation = true`, neutral confirmation may be allowed only when `v2_require_strict_confirmation = false`.

Default must be strict and no neutral confirmation.

### EMA ribbon alignment

If `v2_require_ema_ribbon_alignment = true`:

Long requires on 1m:

```text
ema_8 > ema_21
ema_21 > ema_50
close > ema_21
```

Short requires on 1m:

```text
ema_8 < ema_21
ema_21 < ema_50
close < ema_21
```

### VWAP / EMA21 distance filter

Compute:

```text
atr = atr_14
distance_to_vwap = abs(close - vwap)
distance_to_ema21 = abs(close - ema_21)
nearest_distance = min(distance_to_vwap, distance_to_ema21)
distance_atr = nearest_distance / atr
```

V2 requires:

```text
v2_vwap_distance_atr_min <= distance_atr <= v2_vwap_distance_atr_max
```

If ATR <= 0, return `Ok(None)`.

### ATR bps filter

Compute:

```text
atr_bps = atr / close * 10000
```

V2 requires:

```text
v2_min_atr_bps <= atr_bps <= v2_max_atr_bps
```

If close <= 0, return `Ok(None)`.

### Volume ratio filter

Compute:

```text
volume_ratio = entry_candle.volume / volume_sma_20
```

V2 requires:

```text
volume_ratio >= v2_min_volume_ratio
```

If volume_sma_20 <= 0, return `Ok(None)`.

### Direction toggles

If `v2_enable_long = false`, V2 must never emit long signals.

If `v2_enable_short = false`, V2 must never emit short signals.

If both false, V2 emits no signals.

### Cooldown bars

V2 must support deterministic cooldown.

Simplest acceptable implementation:

- Add `last_signal_index: Option<u64>` state to V2 strategy.
- Update strategy trait to use `&mut self` only if this does not create broad breakage.
- If changing trait is too invasive, implement cooldown in `BacktestEngine` for V2 only based on last emitted signal index.

Cooldown rule:

```text
if current_signal_index - last_signal_index <= v2_cooldown_bars:
    return no signal
```

Use exact deterministic integer comparison.

Do not use wall-clock time.

Do not use random state.

### Signal geometry

For long:

```text
entry = close
stop_loss = close - atr * v2_sl_atr_multiple
take_profit = close + atr * v2_tp_atr_multiple
```

For short:

```text
entry = close
stop_loss = close + atr * v2_sl_atr_multiple
take_profit = close - atr * v2_tp_atr_multiple
```

Do not hardcode 1.5 in v2.

Use config multipliers.

### Expected reward bps

For long:

```text
expected_reward_bps = (take_profit - entry) / entry * 10000
```

For short:

```text
expected_reward_bps = (entry - take_profit) / entry * 10000
```

Then:

```text
expected_net_edge_bps = expected_reward_bps - estimated_cost_bps
```

### V2 edge filters

V2 must only emit if:

```text
expected_reward_bps >= v2_min_expected_reward_bps
expected_net_edge_bps >= v2_min_expected_net_edge_bps
```

This should reduce noisy signals before RiskEngine.

### Confidence

Keep deterministic confidence.

Recommended scoring:

Start with 50.

Add:

```text
+10 screening and confirmation align
+10 EMA ribbon aligns
+10 volume_ratio passes
+10 expected_net_edge_bps passes
+10 VWAP/EMA21 distance passes
```

Clamp to 100.

If confidence < `min_confidence`, return `Ok(None)`.

### Filters passed / failed

For emitted signals, populate rich `filters_passed`.

Examples:

```text
screening_bullish
screening_bearish
confirmation_bullish
confirmation_bearish
ema_ribbon_long
ema_ribbon_short
near_vwap_or_ema21
atr_bps_in_range
volume_ratio_ok
expected_reward_ok
expected_net_edge_ok
direction_enabled
confidence_ok
cooldown_ok
```

If the strategy returns `Ok(None)`, rejected filters do not need to be recorded unless the interface already supports it.

## Reports

Existing reports already include `strategy_id` per trade.

Add attribution by strategy:

```text
reports/attribution_by_strategy.csv
```

Use same header as other attribution bucket CSVs:

```csv
key,trades,wins,losses,win_rate,net_pnl,gross_pnl,total_fee,total_slippage,avg_net_pnl,avg_expected_edge_bps,avg_actual_edge_bps,avg_bars_held
```

where:

```text
key = strategy_id
```

Update:

- `src/report/attribution.rs`
- `src/report/mod.rs` or writer code if needed
- `src/report/manifest.rs`
- tests

Do not remove existing attribution files.

## CLI output

Update research command to print selected strategy:

```text
Strategy:
  strategy_id = screened_vwap_scalp_v2
```

When v2 is selected, print concise V2 config:

```text
V2 filters:
  strict confirmation: true
  EMA ribbon alignment: true
  min expected reward bps: 25.00
  min expected net edge bps: 10.00
  TP ATR multiple: 2.50
  SL ATR multiple: 1.00
  cooldown bars: 5
```

Keep output readable.

## Documentation

Update README.md.

Add short section:

```markdown
### Strategy variants

Northflow currently supports:

- `screened_vwap_scalp` — original deterministic strategy.
- `screened_vwap_scalp_v2` — stricter cost-aware research variant.

V2 adds configurable filters for strict MTF confirmation, EMA ribbon alignment, ATR bps range, VWAP/EMA21 distance, minimum expected reward bps, minimum expected net edge bps, TP/SL ATR multipliers, volume ratio, and cooldown bars.

V2 is diagnostic/research only. It is not a profitability claim and is not an optimizer.
```

Create or update:

```text
docs/STRATEGY_RESEARCH.md
```

Include:

- how to switch `strategy_id`
- how to compare v1 vs v2 with separate `reports_dir`
- recommended diagnostic mode with `max_drawdown_pct = 100.0` and `max_daily_loss_pct = 100.0`
- warning that this is not live trading and not financial advice

## Recommended comparison configs

Document examples.

### V1 baseline

```toml
[strategy]
strategy_id = "screened_vwap_scalp"

[backtest]
reports_dir = "reports/v1_reanchor"
entry_geometry_mode = "reanchor_to_actual_entry"

[risk]
max_drawdown_pct = 100.0
max_daily_loss_pct = 100.0
```

### V2 research

```toml
[strategy]
strategy_id = "screened_vwap_scalp_v2"
v2_require_strict_confirmation = true
v2_require_ema_ribbon_alignment = true
v2_allow_neutral_confirmation = false
v2_min_expected_reward_bps = 25.0
v2_min_expected_net_edge_bps = 10.0
v2_min_atr_bps = 8.0
v2_max_atr_bps = 120.0
v2_tp_atr_multiple = 2.5
v2_sl_atr_multiple = 1.0
v2_min_volume_ratio = 1.0
v2_vwap_distance_atr_min = 0.0
v2_vwap_distance_atr_max = 1.5
v2_cooldown_bars = 5
v2_enable_long = true
v2_enable_short = true

[backtest]
reports_dir = "reports/v2_reanchor"
entry_geometry_mode = "reanchor_to_actual_entry"

[risk]
max_drawdown_pct = 100.0
max_daily_loss_pct = 100.0
```

## Tests required

Add focused tests.

### Config tests

- parses_strategy_id_v1
- parses_strategy_id_v2
- rejects_unknown_strategy_id
- v2_defaults_are_safe
- parses_v2_tp_sl_multipliers
- rejects_invalid_v2_tp_multiplier
- rejects_invalid_v2_atr_range
- rejects_invalid_v2_vwap_distance_range

### V2 signal tests

- v2_returns_none_when_indicators_missing
- v2_returns_none_when_screening_neutral
- v2_long_requires_bullish_screening
- v2_short_requires_bearish_screening
- v2_long_requires_strict_confirmation_by_default
- v2_short_requires_strict_confirmation_by_default
- v2_long_requires_ema_ribbon_alignment
- v2_short_requires_ema_ribbon_alignment
- v2_rejects_expected_reward_below_min
- v2_rejects_expected_net_edge_below_min
- v2_rejects_atr_bps_below_min
- v2_rejects_atr_bps_above_max
- v2_rejects_volume_ratio_below_min
- v2_rejects_too_far_from_vwap_or_ema21
- v2_emits_long_signal_with_valid_geometry
- v2_emits_short_signal_with_valid_geometry
- v2_uses_configurable_tp_atr_multiple
- v2_uses_configurable_sl_atr_multiple
- v2_signal_id_is_deterministic
- v2_strategy_id_is_screened_vwap_scalp_v2
- v2_filters_passed_are_populated

### Backtest integration tests

- backtest_selects_v1_from_config
- backtest_selects_v2_from_config
- unknown_strategy_id_returns_config_error
- v2_trade_reports_strategy_id

### Attribution/report tests

- attribution_by_strategy_groups_v1_or_v2
- attribution_by_strategy_csv_header_is_stable
- manifest_includes_attribution_by_strategy

Existing tests must continue passing.

## Required commands

Run:

```bash
cargo fmt
cargo build
cargo test
cargo run -- help
```

If `data/historical/BTCUSDT.csv` exists, run both configs.

### V1 baseline

```toml
[strategy]
strategy_id = "screened_vwap_scalp"

[backtest]
reports_dir = "reports/v1_reanchor"
entry_geometry_mode = "reanchor_to_actual_entry"

[risk]
max_drawdown_pct = 100.0
max_daily_loss_pct = 100.0
```

Run:

```bash
cargo run --release -- research --config config/research.toml
```

### V2 research

```toml
[strategy]
strategy_id = "screened_vwap_scalp_v2"

[backtest]
reports_dir = "reports/v2_reanchor"
entry_geometry_mode = "reanchor_to_actual_entry"

[risk]
max_drawdown_pct = 100.0
max_daily_loss_pct = 100.0
```

Run:

```bash
cargo run --release -- research --config config/research.toml
```

Do not hardcode expected profit.

Expected:

- V2 may reduce trades significantly.
- V2 reports must be self-consistent.
- Audit must pass.
- Diagnostics must be generated.
- No paper/live/exchange/LLM behavior.

## Strictly forbidden

Do not implement:

- auto optimizer
- grid search
- genetic algorithm
- walk-forward optimization
- paper trading
- live trading
- exchange order placement
- exchange adapter
- websocket
- database
- dashboard
- Telegram
- LLM signal generation
- AI advisor
- profitability claims

Do not say V2 is profitable.

It is only a deterministic research strategy variant.

## Expected final result

At the end of this patch:

- v1 strategy remains unchanged.
- v2 strategy exists as `screened_vwap_scalp_v2`.
- Strategy selection works from config.
- V2 filters are configurable from TOML.
- V2 emits deterministic `Signal` only.
- Backtest still handles risk, sizing, fills, and reports.
- Reports include strategy_id.
- Attribution by strategy is available.
- Docs explain how to compare v1 vs v2.
- All tests pass.

## Commit message suggestion

strategy: add screened vwap scalp v2
