# Strategy Research Status

This document is the current research status for the BTCUSDT 1m scalping strategy search.

No result in this document is a live-trading claim. All numbers are historical backtest research only.

## Current best candidate

The best candidate so far is the bearish short-only VWAP reclaim preset:

```text
config/research_vwap_reclaim_mid_edge_cd0.toml
config/research_vwap_reclaim_mid_edge_cd6.toml
```

Best observed report:

```text
reports/vwap_reclaim_mid_edge_cd0
reports/vwap_reclaim_mid_edge_cd6
```

Summary:

```text
total_trades: 8
win_rate: 75.00%
net_pnl: +110.83
profit_factor: 3.03
max_drawdown: 0.56%
max_consecutive_losses: 1
```

Interpretation:

- This is a bearish-regime short strategy candidate.
- It does not short during bullish 15m regime; bullish regime becomes no-signal because long is disabled.
- Sample size is still too small to claim robustness.
- This candidate must go through out-of-sample / walk-forward validation before paper trading.

## Bearish candidate settings

Core settings:

```toml
strategy_id = "ema_trend_pullback_v1"
etp_allow_long = false
etp_allow_short = true
etp_pullback_to = "vwap"
etp_max_atr_bps = 20.8
etp_cooldown_bars = 0 # or 6; both produced the best observed summary
```

Why this works better:

- Short-only removed the losing long side.
- ATR cap removed the bad `edge_gte_50` bucket.
- Cooldown 0/6 allowed one additional profitable trade compared with cooldown 12/24.

## Abandoned bullish paths

The following bullish/long paths failed and should not be tuned further as-is:

### VWAP reclaim long

Reports:

```text
reports/vwap_reclaim_bull_baseline
reports/vwap_reclaim_u_mid_edge
```

Result:

- Baseline long-only remained net-negative.
- Mid-edge capped long-only became worse.

### EMA21 pullback long

Report:

```text
reports/ema21_long_mid_edge_cd0
```

Result:

```text
total_trades: 193
win_rate: 18.65%
net_pnl: -2780.48
profit_factor: 0.185
max_drawdown: 56.06%
max_consecutive_losses: 24
```

Decision: abandon.

### V2 bullish continuation

Reports:

```text
reports/v2_bull_continuation_mid_edge
reports/v2_bull_vol15
reports/v2_bull_tp20_vol15
```

Result:

- All remained deeply net-negative.
- Tightening volume / anchor distance reduced trade count but did not reveal edge.
- Lowering TP did not fix the expectancy problem.

Decision: abandon V2 config tuning for bullish side.

## Current decision

Do not force one strategy to handle both long and short.

The current architecture direction should be:

```text
Bearish regime:
  use VWAP reclaim short candidate after validation

Bullish regime:
  needs a new dedicated bullish strategy module

Neutral regime:
  no trade until separate range strategy is researched
```

## Next implementation direction

The next bullish candidate should not be another config preset from the existing ETP/V2 strategies.

Implement a dedicated bullish strategy module only if it has a materially different hypothesis, for example:

```text
bull_breakout_retest_v1
```

Candidate concept:

- 15m bullish trend filter.
- 5m bullish confirmation.
- 1m breakout above recent range high or prior swing high.
- Retest holds above breakout level.
- Volume expansion on breakout or retest.
- ATR-based stop below retest low.
- TP based on fixed RR or next ATR expansion.
- Avoid chasing extended candles.

This should be built as a separate research strategy, not as another VWAP reclaim or EMA21 pullback preset.
