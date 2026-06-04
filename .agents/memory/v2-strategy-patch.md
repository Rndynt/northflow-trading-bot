---
name: V2 strategy patch
description: Design decisions for screened_vwap_scalp_v2 and all supporting changes.
---

## Strategy file
`src/strategy/screened_vwap_scalp_v2.rs` — `ScreenedVwapScalpV2::new(V2Config)`.
Strategy ID literal: `"screened_vwap_scalp_v2"`.
Signal ID format: `SIG-BT-{index:08}` (shared monotonic counter in engine, same as V1).

## Config
`V2Config` struct defined in `src/config/mod.rs` (not in the strategy file).
`ResearchConfig` gains 15 `v2_*` flat fields + `strategy_id: String` + `v2_config()` method + `validate_strategy_config()`.
Parser accepts both `strategy_id` and legacy `active` keys.
`validate_strategy_config()` must be called before BacktestEngine::run() CSV check so unknown strategy_id always returns Err even without a CSV.

## Engine
`ActiveStrategy { V1(ScreenedVwapScalp), V2(ScreenedVwapScalpV2) }` enum in engine.rs (module-private).
Strategy selected via `cfg.strategy_id.as_str()` match at start of replay loop setup.
Cooldown: `last_signal_bar: Option<usize>` tracked locally in the loop; set to `Some(i)` when signal preapproved; check `i.saturating_sub(last) <= cooldown_bars` before strategy eval.
Cooldown applies to all strategies but only activates when `cooldown_bars > 0`.

## Attribution
`AttributionReport` gains `by_strategy: Vec<AttributionBucket>`.
`AttributionEngine::build()` computes it via `bucket_by(|t| t.strategy_id.as_str().to_string())`.
`AttributionWriter::write_all()` writes `attribution_by_strategy.csv`.
`ManifestWriter::build()` includes the file.
Empty trades case: `by_strategy: vec![]`.

## Test helpers for V2 long
close=102, ema_8=101.5>ema_21=101.0>ema_50=100.5 (ribbon long ok), atr=1.0, vwap=102.1 (dist 0.1 atr), volume=2000, sma20=1000 (ratio=2.0), atr_bps≈98 ✓, reward≈196 bps ✓.

## Test helpers for V2 short
close=98, ema_8=98.5<ema_21=99.0<ema_50=99.5 (ribbon short ok), atr=1.0, vwap=97.9, volume=2000, sma20=1000.

**Why:** V2 is a diagnostic/research variant. All filters are hard gates so confidence always hits 100. The `min_confidence` check still runs (future softening possible). Cooldown in engine not strategy keeps the Strategy trait stateless.

**How to apply:** When adding a third strategy variant, add a new `ActiveStrategy` arm, extend `validate_strategy_config()`, and add `by_strategy` will automatically group it correctly.
