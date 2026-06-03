# Northflow Phase 3 Build Prompt

You are working on this repository:

https://github.com/Rndynt/northflow-crypto-trading-bot

Your task is to implement **Phase 3: Deterministic Indicators** exactly according to the repository documentation.

Read and follow these files first:

- `AGENTS.md`
- `docs/ROADMAP.md`
- `README.md`
- `config/research.toml`
- `src/core/candle.rs`
- `src/core/timeframe.rs`
- `src/market/ohlcv_loader.rs`
- `src/market/timeframe_builder.rs`
- `src/market/candle_store.rs`
- existing files under `src/indicators/`

Do not ignore the repository documentation.

## Project mission

Northflow is a deterministic research-first crypto trading engine.

It is not:

- a dashboard
- a React app
- a Telegram bot
- an AI trading agent
- a live trading system
- a paper trading loop
- a strategy router

The current goal is to build a deterministic indicator layer that later phases can use for strategy evaluation, risk validation, backtesting, and reporting.

Do **not** implement Phase 4 strategy yet.

Do **not** implement risk sizing.

Do **not** implement backtest execution.

Do **not** generate trades.

Do **not** generate reports.

Do **not** claim profitability.

## Current phase

Implement:

```text
Phase 3 — Indicators
```

From `docs/ROADMAP.md`, Phase 3 must support:

```text
EMA 8
EMA 21
EMA 50
EMA 200
ATR 14
VWAP
Volume SMA 20
```

Target structure from roadmap:

```text
src/indicators/
  ema.rs
  atr.rs
  vwap.rs
  volume.rs
```

If the current repository already has partial indicator files, review them first and improve them instead of blindly duplicating logic.

## Important constraints

Keep the indicator system:

- deterministic
- pure Rust
- no network
- no exchange API
- no async
- no database
- no UI
- no LLM
- no strategy decision
- no fake trading
- no synthetic candles
- no interpolation
- no forward-fill

Avoid unnecessary external dependencies.

Use the existing `crate::core::Candle`.

Use the existing `crate::core::NorthflowError` where errors are needed.

## Phase 1 and Phase 2 preservation

Do not break Phase 1 core types.

Do not break Phase 2 market data loader, data quality rules, or timeframe builder.

The following must remain true:

```text
Phase 1: core domain types remain valid
Phase 2: 1m CSV source remains validated and transformed into 5m/15m candles
Phase 3: indicators consume validated candles, not raw CSV rows
```

Do not import from `legacy/`.

Do not reintroduce `src/data` as an active loader.

Active market data path must remain:

```rust
crate::market::OhlcvLoader
crate::market::CandleStore
```

## Required active structure

Ensure this structure exists:

```text
src/indicators/mod.rs
src/indicators/ema.rs
src/indicators/atr.rs
src/indicators/vwap.rs
src/indicators/volume.rs
```

You may add one extra file if useful:

```text
src/indicators/snapshot.rs
```

Only add `snapshot.rs` if it keeps the system cleaner and does not expand into strategy logic.

Do not add strategy modules in this phase.

Do not add risk modules in this phase.

Do not add backtest modules in this phase.

## Required exports

Update `src/indicators/mod.rs` to cleanly export:

```rust
pub use ema::Ema;
pub use atr::Atr;
pub use vwap::Vwap;
pub use volume::VolumeSma;
```

If you add a snapshot type, export it clearly:

```rust
pub use snapshot::IndicatorSnapshot;
```

Do not expose unstable internal helper types unless needed.

## EMA requirements

Implement or repair `src/indicators/ema.rs`.

EMA must be deterministic and streaming.

Required behavior:

```text
period > 0
alpha = 2 / (period + 1)
first value initializes EMA directly from the first price
next values update with:
ema = previous_ema + alpha * (price - previous_ema)
```

Required public API:

```rust
#[derive(Debug, Clone)]
pub struct Ema {
    // internal fields
}

impl Ema {
    pub fn new(period: usize) -> Result<Self, NorthflowError>;
    pub fn period(&self) -> usize;
    pub fn is_ready(&self) -> bool;
    pub fn value(&self) -> Option<f64>;
    pub fn next(&mut self, price: f64) -> Result<f64, NorthflowError>;
    pub fn reset(&mut self);
}
```

If the existing API already uses `Ema::new(period) -> Self`, update usages and tests to the safer `Result` API.

Validation:

- reject `period == 0`
- reject non-finite price
- reject price <= 0

Important:

- EMA readiness can be `true` after the first valid price, because EMA has an initialized value.
- Do not use SMA warmup unless explicitly documented; for this phase, first price initializes EMA.

Required tests:

```text
ema_rejects_zero_period
ema_rejects_nan_price
ema_rejects_negative_price
ema_first_value_equals_first_price
ema_second_value_uses_alpha_formula
ema_reset_clears_value
ema_period_returns_period
```

Also add tests for specific periods used by Phase 3:

```text
ema_8_can_be_created
ema_21_can_be_created
ema_50_can_be_created
ema_200_can_be_created
```

## ATR requirements

Implement or repair `src/indicators/atr.rs`.

Use ATR 14 as required by roadmap.

ATR must be deterministic and streaming.

Use True Range:

```text
TR = max(
  high - low,
  abs(high - previous_close),
  abs(low - previous_close)
)
```

For the first candle, where no previous close exists:

```text
TR = high - low
```

Use Wilder-style smoothing after warmup:

```text
initial ATR = average of first N true ranges
next ATR = ((previous_atr * (N - 1)) + current_tr) / N
```

Required public API:

```rust
#[derive(Debug, Clone)]
pub struct Atr {
    // internal fields
}

impl Atr {
    pub fn new(period: usize) -> Result<Self, NorthflowError>;
    pub fn period(&self) -> usize;
    pub fn is_ready(&self) -> bool;
    pub fn value(&self) -> Option<f64>;
    pub fn next(&mut self, candle: Candle) -> Result<Option<f64>, NorthflowError>;
    pub fn reset(&mut self);
}
```

Behavior:

- `next()` returns `Ok(None)` until enough true ranges are collected.
- Once ready, `next()` returns `Ok(Some(atr))`.
- Use `Candle::validate()` before processing.
- Reject invalid candles through error.
- `period == 0` must return error.

Required tests:

```text
atr_rejects_zero_period
atr_rejects_invalid_candle
atr_first_values_not_ready_until_period
atr_initial_value_is_average_true_range
atr_uses_previous_close_for_true_range
atr_wilder_smoothing_after_ready
atr_reset_clears_state
atr_14_can_be_created
```

## VWAP requirements

Implement or repair `src/indicators/vwap.rs`.

VWAP must be deterministic and streaming.

Use typical price:

```text
typical_price = (high + low + close) / 3
vwap = cumulative(typical_price * volume) / cumulative(volume)
```

Required public API:

```rust
#[derive(Debug, Clone, Default)]
pub struct Vwap {
    // internal fields
}

impl Vwap {
    pub fn new() -> Self;
    pub fn is_ready(&self) -> bool;
    pub fn value(&self) -> Option<f64>;
    pub fn next(&mut self, candle: Candle) -> Result<Option<f64>, NorthflowError>;
    pub fn reset(&mut self);
}
```

Behavior:

- Use `Candle::validate()` before processing.
- If volume is zero, do not divide by zero.
- For zero-volume candles:
  - Either return current VWAP if already ready, or `None` if not ready.
  - Do not update cumulative volume.
  - Do not panic.
- VWAP is ready only after cumulative volume > 0.

Required tests:

```text
vwap_starts_not_ready
vwap_first_nonzero_volume_candle_calculates_value
vwap_accumulates_multiple_candles
vwap_zero_volume_before_ready_returns_none
vwap_zero_volume_after_ready_returns_existing_value
vwap_rejects_invalid_candle
vwap_reset_clears_state
```

## Volume SMA 20 requirements

Create `src/indicators/volume.rs`.

Implement `VolumeSma`.

Volume SMA must be deterministic and streaming.

Required public API:

```rust
#[derive(Debug, Clone)]
pub struct VolumeSma {
    // internal fields
}

impl VolumeSma {
    pub fn new(period: usize) -> Result<Self, NorthflowError>;
    pub fn period(&self) -> usize;
    pub fn is_ready(&self) -> bool;
    pub fn value(&self) -> Option<f64>;
    pub fn next(&mut self, volume: f64) -> Result<Option<f64>, NorthflowError>;
    pub fn reset(&mut self);
}
```

Behavior:

- reject `period == 0`
- reject non-finite volume
- reject volume < 0
- return `None` until period samples collected
- once ready, return average of last `period` volumes
- keep only last `period` values
- use efficient enough implementation; a simple `VecDeque` plus rolling sum is acceptable

Required tests:

```text
volume_sma_rejects_zero_period
volume_sma_rejects_nan_volume
volume_sma_rejects_negative_volume
volume_sma_not_ready_until_period
volume_sma_computes_average
volume_sma_rolls_window_forward
volume_sma_reset_clears_state
volume_sma_20_can_be_created
```

## Optional IndicatorSnapshot

If useful, create `src/indicators/snapshot.rs`.

Purpose:

A passive data container for current indicator values.

It must not contain strategy decisions.

Allowed fields:

```rust
#[derive(Debug, Clone, Default)]
pub struct IndicatorSnapshot {
    pub ema_8: Option<f64>,
    pub ema_21: Option<f64>,
    pub ema_50: Option<f64>,
    pub ema_200: Option<f64>,
    pub atr_14: Option<f64>,
    pub vwap: Option<f64>,
    pub volume_sma_20: Option<f64>,
}
```

Forbidden fields:

- signal
- side
- entry
- stop_loss
- take_profit
- confidence
- strategy decision
- risk sizing
- trade decision

This type is optional. Add it only if it improves testability or future Phase 4 integration.

## Optional IndicatorEngine

You may add a small `IndicatorEngine` only if it stays strictly within Phase 3.

If added, it must do only this:

- own EMA 8/21/50/200
- own ATR 14
- own VWAP
- own Volume SMA 20
- update them from one validated `Candle`
- return `IndicatorSnapshot`

Allowed API:

```rust
pub struct IndicatorEngine {
    // internal indicators
}

impl IndicatorEngine {
    pub fn new_default() -> Result<Self, NorthflowError>;
    pub fn next(&mut self, candle: Candle) -> Result<IndicatorSnapshot, NorthflowError>;
    pub fn reset(&mut self);
}
```

Rules:

- It must not emit signals.
- It must not evaluate strategy.
- It must not check bullish/bearish conditions.
- It must not place orders.
- It must not call risk model.
- It must not backtest.
- It must only compute indicator values.

If this feels too much for Phase 3, do not add it. Individual indicators are enough.

## Research CLI behavior for Phase 3

Update `src/research/mod.rs` lightly.

The command:

```text
cargo run -- research --config config/research.toml
```

should still:

- validate config
- load market data
- build candle store
- print truthful data summary

Add Phase 3 indicator readiness summary only if clean and simple.

Example acceptable output:

```text
Next: Phase 4 — strategy engine
Indicators ready:
  EMA 8/21/50/200
  ATR 14
  VWAP
  Volume SMA 20
```

Do not run a strategy.

Do not generate signals.

Do not generate fake trades.

Do not produce backtest results.

Do not write reports.

If no CSV exists, keep the existing friendly message.

## Documentation update

Update `README.md` to state:

- Current phase is Phase 3.
- Phase 1 core domain is complete.
- Phase 2 market data foundation is complete.
- Phase 3 indicators are implemented.
- Paper and live modes remain disabled.
- No strategy/backtest/report generation yet.
- Required indicators:
  - EMA 8/21/50/200
  - ATR 14
  - VWAP
  - Volume SMA 20

Do not remove `docs/ROADMAP.md`.

Do not rewrite the entire roadmap unless needed to mark Phase 3 status.

## Strictly forbidden in Phase 3

Do not create:

- React app
- TypeScript app
- dashboard
- web UI
- Telegram integration
- LLM trading decision
- manager agent
- learning agent
- survival agent
- orchestrator
- live exchange order placement
- paper trading loop
- strategy router
- portfolio optimizer
- 100x leverage logic
- fake trades
- fake backtest report
- synthetic candles
- interpolated candles
- exchange API integration
- websocket feed
- database requirement

Do not implement:

- Kalman filter
- HMM
- VPIN
- order flow
- alpha gate
- Kelly
- portfolio optimization
- strategy signal generation
- screened_vwap_scalp
- risk sizing
- backtest engine
- report writers

Those are not Phase 3.

## Required tests

All existing Phase 1 and Phase 2 tests must keep passing.

Add tests for:

```text
EMA
ATR
VWAP
VolumeSma
module exports
optional IndicatorSnapshot or IndicatorEngine if implemented
```

Minimum test coverage must include:

```text
cargo test
```

No failing tests.

No ignored tests unless there is a clear reason.

No TODO stubs in active indicator behavior.

## Minimum command target

These commands must pass:

```text
cargo fmt
cargo build
cargo test
cargo run -- research --config config/research.toml
cargo run -- help
```

If any command fails, fix it before finishing.

Do not claim success unless all commands pass.

## Expected final result

At the end of Phase 3, the repository should have:

- Phase 1 core domain still intact
- Phase 2 market data still intact
- deterministic EMA implementation
- deterministic ATR implementation using Wilder smoothing
- deterministic VWAP implementation
- deterministic Volume SMA 20 implementation
- tests for all indicators
- clean indicator exports from `src/indicators/mod.rs`
- README updated to Phase 3
- no strategy logic
- no risk logic
- no backtest execution
- no report generation
- `cargo fmt` passing
- `cargo build` passing
- `cargo test` passing
- `cargo run -- research --config config/research.toml` working

## Suggested implementation order

1. Read `AGENTS.md` and `docs/ROADMAP.md`.
2. Review existing `src/indicators/` files.
3. Repair `ema.rs`.
4. Repair `atr.rs`.
5. Repair `vwap.rs`.
6. Add `volume.rs`.
7. Update `src/indicators/mod.rs`.
8. Optionally add passive `IndicatorSnapshot`.
9. Optionally add `IndicatorEngine` only if it stays indicator-only.
10. Update `src/research/mod.rs` with Phase 3 status only.
11. Update README.
12. Run `cargo fmt`.
13. Run `cargo build`.
14. Run `cargo test`.
15. Run `cargo run -- research --config config/research.toml`.
16. Run `cargo run -- help`.

## Commit message suggestion

```text
phase3: implement deterministic indicators
```