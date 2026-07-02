# VWAP Reclaim Research

This document records the current research direction after the first strategy comparison and EMA Trend Pullback preset runs.

## Current finding

The original strategy candidates are not usable yet:

- `screened_vwap_scalp` loses heavily.
- `screened_vwap_scalp_v2` loses heavily.
- `ema_trend_pullback_v1` with mixed `ema21_or_vwap` is more selective, but still loses.
- `ema_trend_pullback_v1` with `ema21` only confirms that EMA21 pullbacks dominate losses.
- `ema_trend_pullback_v1` with `vwap` only is the best current clue: gross PnL improved, but net PnL is still negative after fee and slippage.
- `reports/vwap_reclaim_trend_v1` is still a preset run, not a dedicated final strategy and not a profitability claim.

The next research candidate is VWAP reclaim, not EMA21 pullback.

## Current implementation status

VWAP reclaim is currently implemented as a research preset using the existing `ema_trend_pullback_v1` engine with VWAP-only parameters.

This is intentional. Do not add a dedicated Rust strategy module until the preset can produce stable net-positive results or until the engine needs behavior that cannot be expressed through `etp_*` config fields.

## Preset

Use:

```bash
cargo run --release -- research --config config/research_vwap_reclaim_trend_v1.toml
```

Output:

```text
reports/vwap_reclaim_trend_v1
```

## Why this is a preset first

The existing `ema_trend_pullback_v1` engine already supports the required VWAP-only behavior through config:

```toml
etp_pullback_to = "vwap"
etp_reclaim_mode = "close_reclaim_or_wick"
```

So this research step does not need a new Rust strategy type yet. The goal is to validate whether VWAP reclaim has enough edge before adding a dedicated implementation.

## Decision rule

After running the preset:

- If trade count is too low and gross is still positive, loosen filters carefully.
- If gross is positive but net is negative, reduce cost sensitivity by increasing reward target or filtering bad volatility buckets.
- If gross turns negative, abandon VWAP reclaim and move to a different candidate such as `breakout_retest_v1`.
- If net becomes positive across enough trades and does not depend on one isolated month, then add a dedicated `vwap_reclaim_trend_v1` strategy module.

No result here is a profitability claim. This is historical research only.
