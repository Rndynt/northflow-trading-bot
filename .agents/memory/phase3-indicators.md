---
name: Phase 3 Indicators design
description: API contracts, error variants, and non-obvious rules for EMA/ATR/VWAP/VolumeSma implemented in Phase 3.
---

# Phase 3 Indicators

## Error variants
- Period == 0 → `NorthflowError::ConfigError`
- Non-finite / negative price or volume → `NorthflowError::DataError`
- Invalid candle (geometry) → `NorthflowError::InvalidCandle` (from `Candle::validate()`)
- `NorthflowError::InvalidConfig` does NOT exist — the right variant is `ConfigError`.

## EMA (src/indicators/ema.rs)
- `new(period) -> Result<Self, NorthflowError>`
- alpha = 2 / (period + 1); first price initialises directly (no SMA warmup)
- `is_ready()` = true after the very first price
- `next(price: f64) -> Result<f64, NorthflowError>` — rejects price ≤ 0 or non-finite

## ATR (src/indicators/atr.rs)
- `new(period) -> Result<Self, NorthflowError>`
- Wilder smoothing — two-phase:
  1. Warmup: collect exactly `period` TRs in `self.warmup: Vec<f64>`, then compute initial ATR = mean; `self.warmup` is cleared after.
  2. Smoothed: `atr = (prev_atr * (period-1) + tr) / period`
- First candle always uses `high - low` (no prev_close); subsequent use prev_close for full TR formula.
- `next(candle: Candle) -> Result<Option<f64>, NorthflowError>` — calls `candle.validate()` first
- `is_ready()` = true only after warmup phase is complete

## VWAP (src/indicators/vwap.rs)
- `new() -> Self` (Default), no period
- Zero-volume candles: do NOT update state. Return `self.value()` (Some if ready, None if not).
- **Why:** old stub returned `None` on zero-vol even when already ready — fixed in Phase 3.
- `next(candle: Candle) -> Result<Option<f64>, NorthflowError>` — calls `candle.validate()` first

## VolumeSma (src/indicators/volume.rs)
- `new(period) -> Result<Self, NorthflowError>`
- Rolling window via `VecDeque<f64>` + running `sum: f64` for O(1) update
- Accepts volume == 0 (valid); rejects volume < 0 or non-finite
- `next(volume: f64) -> Result<Option<f64>, NorthflowError>`

## IndicatorSnapshot + IndicatorEngine (src/indicators/snapshot.rs)
- `IndicatorSnapshot`: passive container, all `Option<f64>` fields (ema_8/21/50/200, atr_14, vwap, volume_sma_20). No strategy fields.
- `IndicatorEngine::new_default() -> Result<Self, NorthflowError>`: creates EMA 8/21/50/200, ATR 14, VWAP, VolumeSma 20.
- `next(candle: Candle) -> Result<IndicatorSnapshot, NorthflowError>`: validates candle, feeds close to EMAs, full candle to ATR/VWAP, volume to VolumeSma.
- EMA `.next()` errors are mapped to `None` via `.ok()` (not propagated) because a valid candle always has a valid close.

## Test counts
- Phase 3 added 42 new tests; total at Phase 3 completion = 173.

**Why:** ATR's two-phase warmup with a scratch Vec avoids storing the entire history after warmup. Zero-vol VWAP returning existing value (not None) was a bug in the original stub — Phase 4 strategy code should not have to special-case this.
