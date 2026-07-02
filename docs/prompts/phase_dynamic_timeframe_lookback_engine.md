# Phase: Dynamic Timeframe Lookback Support in Main Engine

## Goal

Add generic entry-timeframe lookback support to the existing main backtest engine.

Do not create any standalone research runner. The final architecture must stay as one main engine with strategy modules.

## Hard Rules

- Do not hardcode entry, confirmation, or screening timeframes in code.
- The source of truth is `ResearchConfig` / TOML: `entry_timeframe`, `confirmation_timeframe`, `screening_timeframe`.
- Keep the engine role-based: entry, confirmation, screening.
- Do not introduce new names like `c5`, `s5`, `c15`, or `s15`.
- Prefer names like `confirmation_candle`, `confirmation_snapshot`, `screening_candle`, `screening_snapshot`.
- Do not add strategy-specific fields to the engine such as previous range high, breakout candle, or retest candle.
- Do not add or restore standalone runners such as `src/bin/bull_breakout_retest_research.rs` or `src/bin/bbr_research.rs`.
- Do not implement a bullish strategy in this phase.

## Required Changes

### 1. Strategy input

File: `src/strategy/traits.rs`

Add generic entry history to `MultiTimeframeInput`:

```rust
pub entry_lookback: Vec<Candle>,
```

Meaning: completed entry-timeframe candles before the current `entry_candle`.

Rules:

- It must never include the current `entry_candle`.
- It must never include future candles.
- It uses configured entry timeframe data, not a hardcoded timeframe.
- Existing strategies may ignore it.

### 2. Config

File: `src/config/mod.rs`

Add:

```rust
pub entry_lookback_bars: usize,
```

Default value:

```rust
0
```

TOML key under `[backtest]`:

```toml
entry_lookback_bars = 0
```

Existing configs must keep working if this field is absent.

### 3. Main engine

File: `src/backtest/engine.rs`

When building `MultiTimeframeInput`, populate lookback from the configured entry candle stream:

```rust
let lookback_start = i.saturating_sub(cfg.entry_lookback_bars);
let entry_lookback = entry_candles[lookback_start..i].to_vec();
```

Then include it in the input.

Important:

- Use `..i`, not `..=i`.
- Do not include current candle.
- Do not change risk, fill, report, or trade accounting logic.
- Keep confirmation and screening snapshot lookup dynamic using the parsed timeframe roles and `to_millis()`.

### 4. Tests and fixtures

Update every manual `MultiTimeframeInput` literal with:

```rust
entry_lookback: vec![],
```

Add tests proving:

- lookback excludes current candle.
- lookback length is capped by `entry_lookback_bars`.
- first candles have shorter or empty lookback.
- existing strategies still compile and can ignore lookback.

## Forbidden Scope

Do not add these in this phase:

- `bull_breakout_retest_v1`
- `vwap_reclaim_short_v1`
- `screened_vwap_scalp_v3`
- `ema_trend_pullback_v2`
- any standalone research runner

## Validation

Run:

```bash
cargo fmt
cargo test
cargo run --release -- research --config config/research_vwap_reclaim_mid_edge_cd0.toml
```

## Expected Commit

```bash
git add src config docs
git commit -m "engine: add dynamic entry lookback input"
```

## Expected Summary

Report:

1. Files changed.
2. Config field added.
3. How engine populates lookback.
4. Tests added.
5. Confirmation that no hardcoded timeframe logic was added.
6. Confirmation that no standalone runner exists.
