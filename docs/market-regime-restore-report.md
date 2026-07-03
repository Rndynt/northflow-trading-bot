# Market Regime Restore Report

## Why regime was restored

Market regime classification was restored as generic market context. Signals already carry a `regime` attribution field, and shared labels help reports, attribution, diagnostics, and future strategies describe market context without embedding sample-only strings in each strategy.

This restore does not claim predictive edge and does not tune strategy behavior for profitability.

## Why regime lives under `market`

The restored component lives in `src/market/regime.rs` because regime classification is market context, not a strategy implementation. Keeping it under `market` preserves the runtime boundary:

- strategies may read prepared market context and emit signals,
- risk sizing remains independent,
- backtest execution remains independent,
- report writing remains independent.

The regime classifier is not placed under `src/strategy/` and no old strategy module was restored.

## Exact regime labels

The stable labels are:

- `bullish`
- `bearish`
- `ranging`
- `unknown`

## Classifier rules

`classify_basic_regime(close, vwap, ema_50)` is deterministic and side-effect-free.

It returns `unknown` when:

- `close` is non-finite or `<= 0`,
- both references are missing,
- any provided reference value is non-finite or `<= 0`.

It returns `bullish` when `close` is above every available valid reference.

It returns `bearish` when `close` is below every available valid reference.

It returns `ranging` when valid references are mixed, equal, or otherwise inconclusive.

## Strategy policy confirmation

Old strategies were not restored. The active production strategy remains only `BasicSampleStrategy` with strategy ID `basic_sample_strategy`.

The sample strategy now uses the generic market classifier only for `Signal.regime` metadata. The classifier is not an additional entry filter and is not coupled to risk sizing or backtest execution.

## Tests added or updated

Added unit coverage for:

- stable `MarketRegime::as_str()` labels,
- `Display` output,
- default `Unknown` regime,
- bullish classification,
- bearish classification,
- ranging classification for mixed/equal references,
- unknown classification for invalid close,
- unknown classification for missing references,
- unknown classification for invalid references.

Updated sample strategy tests to confirm:

- long signals use `bullish`, not `sample_bullish`,
- short signals use `bearish`, not `sample_bearish`,
- signal regimes are one of the generic stable labels,
- the sample strategy still emits valid signals for clear setups,
- no-setup behavior still returns `Ok(None)`.

Existing registry tests continue to verify that `basic_sample_strategy` resolves and old strategy IDs are rejected.

## Search and cleanup results

Searched for stale sample-only labels and old strategy references. `sample_bullish` and `sample_bearish` no longer appear in production code. The remaining occurrences are in historical prompt text and negative test assertions documenting that emitted signals must not use those old labels.

Old strategy names remain only in historical prompt/report files and registry rejection tests; they were not reintroduced in active strategy code, config, or registry acceptance paths.

## Commands run and results

The final validation commands were run after implementation:

- `cargo fmt --check` — passed.
- `cargo test` — passed.
- `cargo run -- research --config config/research.toml` — passed.
