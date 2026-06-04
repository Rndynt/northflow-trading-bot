# Strategy Research Guide

Northflow supports deterministic, configurable strategy variants for controlled research comparison.

> **Disclaimer:** All backtest results are historical simulation only.
> They are not financial advice and do not guarantee future profitability.
> This is a research/diagnostic tool only. Paper and live trading are disabled.

---

## Strategy variants

| `strategy_id` | Description |
|---|---|
| `screened_vwap_scalp` | Original deterministic multi-timeframe scalp strategy. |
| `screened_vwap_scalp_v2` | Stricter, cost-aware research variant with configurable filters. |

V2 adds: strict MTF confirmation, EMA ribbon alignment, ATR bps range, VWAP/EMA21 distance, minimum expected reward bps, minimum expected net edge bps, TP/SL ATR multipliers, volume ratio, and cooldown bars.

V2 is a diagnostic/research variant only. It is not a profitability claim and is not an optimizer.

---

## Switching strategy

Edit `config/research.toml`:

```toml
[strategy]
strategy_id = "screened_vwap_scalp"   # V1
# strategy_id = "screened_vwap_scalp_v2"  # V2
```

Then run:

```bash
cargo run -- research --config config/research.toml
```

---

## Comparing V1 vs V2 with separate reports directories

Run V1:

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

```bash
cargo run --release -- research --config config/research.toml
```

Run V2:

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

```bash
cargo run --release -- research --config config/research.toml
```

Compare `reports/v1_reanchor/` vs `reports/v2_reanchor/` side by side.

---

## Recommended diagnostic mode

For research comparison, disable risk guards so every signal that passes the strategy gets a trade:

```toml
[risk]
max_drawdown_pct = 100.0
max_daily_loss_pct = 100.0
```

This allows the full signal set to flow through to trades, giving the best view of the strategy's raw edge before portfolio-level risk constraints are applied.

---

## V2 configurable parameters

| Parameter | Default | Description |
|---|---|---|
| `v2_require_strict_confirmation` | `true` | 5m regime must exactly match 15m regime direction. |
| `v2_require_ema_ribbon_alignment` | `true` | 1m ema_8 / ema_21 / ema_50 must align with side. |
| `v2_allow_neutral_confirmation` | `false` | Allow neutral 5m when strict confirmation is off. |
| `v2_min_expected_reward_bps` | `20.0` | Minimum expected reward in basis points before cost. |
| `v2_min_expected_net_edge_bps` | `5.0` | Minimum expected net edge in basis points after cost. |
| `v2_min_atr_bps` | `5.0` | Minimum ATR in basis points (volatility floor). |
| `v2_max_atr_bps` | `150.0` | Maximum ATR in basis points (volatility ceiling). |
| `v2_tp_atr_multiple` | `2.0` | Take profit = entry ± atr × this multiple. |
| `v2_sl_atr_multiple` | `1.0` | Stop loss = entry ± atr × this multiple. |
| `v2_min_volume_ratio` | `1.0` | Volume must be ≥ this multiple of volume_sma_20. |
| `v2_vwap_distance_atr_min` | `0.0` | Minimum distance from VWAP or EMA21 in ATR units. |
| `v2_vwap_distance_atr_max` | `2.0` | Maximum distance from VWAP or EMA21 in ATR units. |
| `v2_cooldown_bars` | `0` | Bars to wait after a signal before evaluating again. |
| `v2_enable_long` | `true` | Allow long signals. |
| `v2_enable_short` | `true` | Allow short signals. |

---

## Report files

Every backtest run (V1 or V2) produces the same set of report files in `reports_dir/`:

```
backtest_summary.json
trades.csv
equity_curve.csv
risk_rejections.csv
signal_flow_summary.json
attribution_summary.json
attribution_by_regime.csv
attribution_by_exit_reason.csv
attribution_by_side.csv
attribution_by_filter.csv
attribution_by_strategy.csv
audit_report.json
report_manifest.json
signal_diagnostics.csv
rejection_by_stage_reason.csv
monthly_summary.csv
cost_edge_distribution.csv
trade_distribution_summary.json
```

`attribution_by_strategy.csv` groups performance by strategy variant, which is most useful when a future multi-strategy run combines V1 and V2 trades in a single backtest.

---

## Important notes

- All results are historical simulation only.
- Do not use backtest results as financial advice or profitability claims.
- V2 reduces signal count compared to V1, which may reduce or increase edge depending on the dataset.
- No optimizer, grid search, or walk-forward optimization is used.
- Strategy parameters are set by the researcher before the run, not by the engine.
