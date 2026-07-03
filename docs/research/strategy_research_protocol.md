# Strategy Research Protocol

This bot is not built to produce research reports forever. The research stage exists only to find a deterministic base edge before any AI decision layer is allowed to rank or approve signals.

## Current Rule

Fees are fixed as a constraint, not a tuning object:

```toml
[cost]
taker_fee_bps = 5.0
slippage_bps = 0.0
spread_bps = 0.0
market_impact_bps = 0.0
stop_slippage_bps = 0.0
```

Do not run maker-fee sensitivity until a real maker/limit execution model exists.

## Strategy Research Loop

1. Pick one strategy family.
2. Run 1m, 5m, and 15m.
3. Split long-only and short-only before combining sides.
4. Test TP/SL geometry and signal strictness.
5. Keep only variants with enough trades and positive post-fee expectancy.
6. Reject strategy families that need overfitting or fee assumptions to survive.

## Acceptance Gate

A base strategy candidate must meet all of these before moving toward AI decision:

- At least 100 trades across 2020-2025.
- Net PnL positive after fixed Binance Futures taker fee.
- Profit factor above 1.10.
- Positive expectancy.
- Max drawdown not dominated by one collapse period.
- No single year should carry the entire result.

## Current Diagnosis

The base matrix showed that most current strategies are not profitable after taker fee. The problem is not the market. The problem is that current base strategies either overtrade or have too little edge per trade.

The retune matrix showed only near-break-even behavior on `ema_trend_pullback_v1` 15m strict edge profile. That is not enough. It may be worth one disciplined deep dive, but it is not a validated strategy.

## Immediate Focus

Do not create 5000 new strategies.

Focus on a controlled deep dive:

- `ema_trend_pullback_v1`
- `screened_vwap_scalp_v2`
- timeframes: `1m`, `5m`, `15m`
- side modes: long-only, short-only, both
- profile modes: loose, medium, strict

The goal is to find whether one existing family has a real directional edge. If not, reject it and design a new strategy from a clear market hypothesis.
