# Northflow Trading Bot — Generic Market Regime Restore Prompt

## Role

You are an implementation agent working on `Rndynt/northflow-trading-bot`, a Rust deterministic crypto trading research/backtest project.

Your task is to restore **market regime classification** as a generic shared domain/market component after the strategy cleanup.

This is not a strategy restoration task. This is not a trading edge task. This is not a profitability task.

The project must still contain only one production strategy: `BasicSampleStrategy` with strategy ID `basic_sample_strategy`.

---

## Primary Objective

Reintroduce a small, generic, reusable market regime model that can be used by the sample strategy, reports, attribution, diagnostics, and future strategies without bringing back old strategy implementations or old strategy IDs.

The regime component must be:

1. Strategy-agnostic.
2. Deterministic.
3. Pure and side-effect-free.
4. Independent from backtest execution.
5. Independent from risk sizing.
6. Independent from report writing.
7. Free of any old strategy-specific logic.

---

## Non-Negotiable Constraints

Do not violate these rules.

1. Do **not** restore old strategies.
2. Do **not** restore old strategy aliases.
3. Do **not** accept old strategy IDs again.
4. Do **not** create more than one production strategy.
5. Do **not** move regime back as a strategy module if it is generic market logic.
6. Do **not** couple regime classification to `BacktestEngine`.
7. Do **not** couple regime classification to risk sizing.
8. Do **not** use randomness.
9. Do **not** use system time.
10. Do **not** call network, exchange APIs, LLMs, or external services.
11. Do **not** alter indicator formulas unless required to compile.
12. Do **not** claim the regime classifier has predictive edge.
13. Do **not** tune for profitability.
14. Do **not** hide failing tests.

---

## Current Context

The strategy cleanup intentionally removed old strategy modules and left only:

```text
src/strategy/basic_sample.rs
src/strategy/registry.rs
src/strategy/traits.rs
```

That is correct for strategy cleanup.

However, regime classification is not inherently a strategy. It is useful as shared market context and attribution metadata.

The `Signal` domain object already has a `regime: String` field. That field should remain.

The current `BasicSampleStrategy` directly writes string labels such as:

```text
sample_bullish
sample_bearish
```

This works, but it is better to centralize generic regime labels in a shared domain/market module.

---

# Desired Design

## Preferred Location

Use one of these locations:

### Preferred

```text
src/market/regime.rs
```

Because market regime is market context.

### Acceptable Alternative

```text
src/core/regime.rs
```

Only use this if the codebase style suggests regime is a core domain enum.

Do **not** put generic regime classification back under `src/strategy/regime.rs` unless there is a strong reason. Regime should not be treated as a strategy implementation.

---

# Required API

Create a small enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketRegime {
    Bullish,
    Bearish,
    Ranging,
    Unknown,
}
```

Add stable string conversion:

```rust
impl MarketRegime {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bullish => "bullish",
            Self::Bearish => "bearish",
            Self::Ranging => "ranging",
            Self::Unknown => "unknown",
        }
    }
}
```

Implement `Display`:

```rust
impl std::fmt::Display for MarketRegime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
```

Optional but useful:

```rust
impl Default for MarketRegime {
    fn default() -> Self {
        Self::Unknown
    }
}
```

---

# Required Classifier

Create a small pure classifier. Keep it intentionally simple.

Suggested function:

```rust
pub fn classify_basic_regime(
    close: f64,
    vwap: Option<f64>,
    ema_50: Option<f64>,
) -> MarketRegime
```

Suggested deterministic rules:

```text
Unknown:
  - close is non-finite or <= 0
  - both vwap and ema_50 are missing
  - any provided reference value is non-finite or <= 0

Bullish:
  - close > vwap, when vwap exists
  - close > ema_50, when ema_50 exists
  - all available references agree bullish

Bearish:
  - close < vwap, when vwap exists
  - close < ema_50, when ema_50 exists
  - all available references agree bearish

Ranging:
  - references are valid but mixed, equal, or inconclusive
```

Example implementation logic:

```rust
pub fn classify_basic_regime(close: f64, vwap: Option<f64>, ema_50: Option<f64>) -> MarketRegime {
    if !close.is_finite() || close <= 0.0 {
        return MarketRegime::Unknown;
    }

    let refs = [vwap, ema_50]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if refs.is_empty() {
        return MarketRegime::Unknown;
    }

    if refs.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return MarketRegime::Unknown;
    }

    let above_all = refs.iter().all(|v| close > *v);
    let below_all = refs.iter().all(|v| close < *v);

    if above_all {
        MarketRegime::Bullish
    } else if below_all {
        MarketRegime::Bearish
    } else {
        MarketRegime::Ranging
    }
}
```

You may refine this only if needed, but keep it simple and deterministic.

---

# Module Export Requirements

If using `src/market/regime.rs`, update `src/market/mod.rs`:

```rust
pub mod regime;

pub use regime::{classify_basic_regime, MarketRegime};
```

If using `src/core/regime.rs`, update `src/core/mod.rs` accordingly.

Do not export regime from `src/strategy/mod.rs` unless strategy specifically needs a re-export. Prefer importing it from `crate::market` or `crate::core`.

---

# BasicSampleStrategy Integration

Update `src/strategy/basic_sample.rs` to use the generic regime classifier.

## Current pattern to replace

The strategy may currently assign direct strings such as:

```rust
"sample_bullish"
"sample_bearish"
```

Replace this with the generic classifier.

Suggested integration:

```rust
use crate::market::classify_basic_regime;
```

Inside evaluation:

```rust
let regime = classify_basic_regime(
    input.screening_candle.close,
    input.screening_indicators.vwap,
    input.screening_indicators.ema_50,
);
```

Then set:

```rust
regime: regime.as_str().to_string(),
```

## Important Behavior

The sample strategy may still use its own entry conditions to decide whether to emit long/short signals.

The classifier should only provide a label for `Signal.regime`.

Do not make regime classification a hard extra filter unless you explicitly document it and tests cover it. Preferred: use it as metadata only.

---

# Tests Required

## Regime Unit Tests

Add tests for the new regime module:

1. `MarketRegime::Bullish.as_str() == "bullish"`.
2. `MarketRegime::Bearish.as_str() == "bearish"`.
3. `MarketRegime::Ranging.as_str() == "ranging"`.
4. `MarketRegime::Unknown.as_str() == "unknown"`.
5. `Display` returns the same stable labels.
6. `classify_basic_regime` returns `Bullish` when close is above all available valid references.
7. `classify_basic_regime` returns `Bearish` when close is below all available valid references.
8. `classify_basic_regime` returns `Ranging` when references are mixed or equal.
9. `classify_basic_regime` returns `Unknown` when close is invalid.
10. `classify_basic_regime` returns `Unknown` when no references exist.
11. `classify_basic_regime` returns `Unknown` when any provided reference is invalid.

## Sample Strategy Tests

Update or add tests to ensure:

1. Long sample signal uses a generic regime string, not `sample_bullish`.
2. Short sample signal uses a generic regime string, not `sample_bearish`.
3. The regime field is one of:

```text
bullish
bearish
ranging
unknown
```

4. The strategy still emits valid signals when sample conditions are met.
5. The strategy still returns `Ok(None)` when no setup exists.

## Registry Tests

Keep existing registry tests that confirm:

- `basic_sample_strategy` resolves successfully.
- old strategy IDs are rejected.

Do not weaken these tests.

---

# Documentation Requirement

Update or create:

```text
docs/market-regime-restore-report.md
```

Include:

1. Why regime was restored.
2. Why it lives under `market` or `core`, not `strategy`.
3. The exact regime labels.
4. Classifier rules.
5. Confirmation that old strategies were not restored.
6. Confirmation that the only active strategy remains `basic_sample_strategy`.
7. Tests added/updated.
8. Commands run and results.

---

# Search And Cleanup Requirement

After implementation, search for stale sample-only regime labels:

```text
sample_bullish
sample_bearish
```

These should not remain in production code after this task unless explicitly documented as historical text.

Search for old strategy references again:

```text
screened_vwap_scalp
screened_vwap_scalp_v2
ema_trend_pullback
ema_trend_pullback_v1
vwap_reclaim_short
vwap_reclaim_short_v1
vwap_reclaim_short_v2
mean_revert
mean_revert_v1
liquidity_sweep_reclaim
liquidity_sweep_reclaim_v1
```

Old strategy names may remain only in historical prompt/report files, not in active code/config/tests/registry.

---

# Final Validation Commands

Run:

```bash
cargo fmt --check
cargo test
cargo run -- research --config config/research.toml
```

If historical data files are missing, the research command may stop with a clear missing-data message. That is acceptable only if config parsing and strategy registry validation pass first.

---

# Acceptance Criteria

This task is complete when all of the following are true:

1. A generic `MarketRegime` model exists.
2. A deterministic `classify_basic_regime` helper exists.
3. The regime module is located in `src/market` or `src/core`, not as an old strategy module.
4. `BasicSampleStrategy` uses the generic regime label for `Signal.regime`.
5. `sample_bullish` and `sample_bearish` are removed from active production code.
6. Old strategy implementations are not restored.
7. Old strategy IDs are still rejected by the registry.
8. Only `basic_sample_strategy` remains as the active production strategy.
9. New regime tests pass.
10. Existing strategy registry tests pass.
11. `cargo fmt --check` passes.
12. `cargo test` passes.
13. `docs/market-regime-restore-report.md` exists and documents the result.

---

# Out Of Scope

Do not implement these in this task:

- New trading strategies.
- Strategy parameter tuning.
- Profitability optimization.
- Advanced market regime modeling.
- Machine learning classification.
- Volatility regime buckets beyond the simple enum.
- Multi-position engine.
- Paper trading.
- Live trading.
- Exchange integration.
- Report redesign beyond adding/using the generic regime label.

---

# Success Definition

The project regains a clean generic market regime abstraction without reintroducing old strategy code, while preserving the single-strategy codebase policy and keeping the engine strategy-agnostic.
