# Forecast ML Cost Model Validation & Executability Review (F9)

## 1. Executive Decision

**Decision: `needs_real_fee_slippage_data_before_decision`.**

The F8 4h Ridge top-decile result is too cost-sensitive to justify `ForecastScorer` or backtest-filter integration. The 10 bps case is only barely positive at `0.32487040` bps, while the 7 bps case requires optimistic maker-assisted or unusually low-slippage execution assumptions that are not evidenced by repo configuration or any venue-specific fill data. The 5 bps case is aggressive and likely fragile unless maker execution is reliable, queue priority is favorable, adverse selection is measured, and realized exits remain cheap.

This review does **not** claim profitability. It only concludes that the current cost-sensitive edge cannot be accepted or rejected with confidence until real BTCUSDT fee, spread, slippage, maker-fill, and adverse-selection evidence is collected. Because F8 monotonicity also stayed weak, no forecast output should be integrated into strategy, risk, fill simulation, or backtest filtering.

## 2. Why F9 exists

F8 found that the 4h Ridge candidate becomes positive in the top decile only when round-trip costs are lowered from the baseline 14 bps assumption. However, F8 also found weak monotonicity and uneven regime subsets, so F9 exists to answer a narrower question:

```text
Are 7–10 bps round-trip costs realistically achievable for the intended BTCUSDT execution assumptions, or did F8 only become positive under unrealistic cost assumptions?
```

This phase is a cost realism and executability review, not a profitability test and not a production integration step.

## 3. Inputs reviewed

Primary inputs reviewed:

- `docs/forecast-ml-roadmap.md`
- `docs/forecast-ml-result-analysis.md`
- `docs/forecast-ml-horizon-comparison.md`
- `docs/forecast-ml-cost-sensitivity-regime-attribution.md`
- `config/forecast/btcusdt_1m_h4h.toml`
- `config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_14bps.toml`
- `config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_10bps.toml`
- `config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_7bps.toml`
- `config/forecast/cost_sensitivity/btcusdt_1m_h4h_cost_5bps.toml`
- Local BTCUSDT 1m OHLCV files under `data/historical/BTCUSDT/1m/` for candle-derived proxy analysis.

The repository contains static cost configs under `config/cost/`, but this F9 review does not treat them as proof of achieved execution because they do not include realized spread, queue position, fill probability, partial fill, or adverse-selection measurements.

## 4. F8 result recap

F8 tested the same 4h Ridge setup under four round-trip cost assumptions. The top-decile effective actual return changed almost one-for-one with the cost reduction:

| Round-trip cost | F8 4h Ridge top-decile effective actual return | Interpretation |
|---:|---:|---|
| 14 bps | `-3.67512960` bps | Negative under conservative baseline. |
| 10 bps | `0.32487040` bps | Barely positive and not robust to estimation error. |
| 7 bps | `3.32487040` bps | Positive, but depends on optimistic cost realism. |
| 5 bps | `5.32487040` bps | Stronger on paper, but requires aggressive execution assumptions. |

The candidate still failed the full F8 gate because:

```text
monotonicity_ratio = 0.44444444
required >= 0.60
```

Therefore F8 did not authorize forecast integration. It only showed that the label is highly sensitive to cost assumptions.

## 5. Cost scenario definitions

The repository's F8 configs express total cost as a compact round-trip model:

```text
round_trip_total_bps =
  entry_fee_bps
+ exit_fee_bps
+ entry_spread_bps
+ exit_spread_bps
+ entry_slippage_bps
+ exit_slippage_bps
+ market_impact_bps
```

The table below decomposes the required scenarios into operational assumptions. Because the repo lacks venue-specific realized execution evidence, these are hypothetical scenario definitions, not measured facts.

| Scenario | entry_fee_bps | exit_fee_bps | entry_spread_bps | exit_spread_bps | entry_slippage_bps | exit_slippage_bps | market_impact_bps | round_trip_total_bps | Operational meaning |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 14 bps baseline | 4.0 | 4.0 | 1.0 | 1.0 | 1.5 | 1.5 | 1.0 | 14.0 | Conservative taker/slippage/impact assumption. |
| 10 bps moderate | 3.0 | 3.0 | 1.0 | 1.0 | 1.0 | 1.0 | 0.0 | 10.0 | Moderate taker or mixed execution assumption with limited slippage. |
| 7 bps optimistic | 2.0 | 2.0 | 0.75 | 0.75 | 0.75 | 0.75 | 0.0 | 7.0 | Optimistic low-slippage or maker-assisted assumption. |
| 5 bps aggressive | 1.5 | 1.5 | 0.5 | 0.5 | 0.5 | 0.5 | 0.0 | 5.0 | Aggressive and likely fragile unless maker execution is reliable. |
| Stress/adverse | 4.0 | 4.0 | 1.5 | 1.5 | 2.5 | 2.5 | 2.0 | 18.0 | Adverse taker/slippage/impact assumption. |

The F8 config naming uses round-trip costs of 14, 10, 7, and 5 bps. The component splits above are an explanatory decomposition for executability review, not a claim that the repo measured those exact components.

## 6. Taker-only cost review

Taker-only execution is the most executable assumption because it does not depend on maker queue priority, but it also has the least room for a small residual edge.

| Case | Cost realism | F8 residual edge implication |
|---|---|---|
| 14 bps | Most consistent with a conservative taker baseline that includes fees, spread/slippage, and impact. | Top decile is negative at `-3.67512960` bps. |
| 10 bps | Possible only if fee tier and realized slippage/spread are materially lower than the baseline. Repo evidence does not prove this. | Top decile is barely positive at `0.32487040` bps. |
| 7 bps | Hard to defend as taker-only without explicit low-fee and low-slippage evidence. | Positive at `3.32487040` bps, but execution assumption is optimistic. |
| 5 bps | Not defensible as taker-only from current repo evidence. | Positive on paper, likely not executable under taker-only assumptions. |

Conclusion: taker-only execution does not support treating the F8 positive 7–10 bps scenarios as robust. The 10 bps edge is too small, and 7 bps is not justified by repo evidence.

## 7. Maker/taker mixed cost review

Maker/taker mixed execution could plausibly reduce average round-trip cost, especially if entries or exits can rest passively. However, the current forecast label does not model the operational costs of waiting for maker fills.

Key unresolved execution questions:

- What share of entries can be maker-filled before the forecast edge decays?
- What share of exits must cross the spread because risk exits cannot wait?
- Are passive fills adversely selected, i.e. filled more often before unfavorable moves?
- Are partial fills tracked?
- Does order waiting time change the effective 4h holding horizon?

Under a mixed assumption, 10 bps may be realistic in some conditions, but the F8 residual edge at 10 bps is only `0.32487040` bps. That is too small to survive ordinary uncertainty in slippage, spread, fill timing, or implementation details. The 7 bps case may be achievable only with a high maker-fill share and low adverse selection, neither of which is measured in this repository.

Conclusion: maker/taker mixed execution makes 7–10 bps plausible enough to study, but not proven enough to integrate.

## 8. Maker-only optimistic cost review

Maker-only execution is the most favorable cost scenario, but it is also the least directly executable without a dedicated execution model. A maker-only assumption must account for queue position, non-fills, partial fills, stale signals, cancellation logic, and adverse selection. The current forecast pipeline does not model those constraints.

Operational implications:

- 7 bps can be plausible only if maker participation is high and passive fills are not systematically adverse.
- 5 bps likely requires consistently favorable fee tier, low spread capture loss, low slippage, and minimal impact.
- A 4h forecast label based on candle close-to-future returns does not prove that a passive entry and passive exit can be obtained at the assumed prices.

Conclusion: maker-only assumptions can explain why F8 becomes positive at 7 bps and 5 bps, but the repo lacks evidence that those fills are available when the model emits top-decile predictions.

## 9. Stress/slippage adverse review

A stress/adverse scenario is important because the F8 residual edge is small relative to plausible execution error. If round-trip cost rises above the 14 bps baseline during volatility, spread widening, or urgent exits, the F8 candidate becomes more negative.

Stress risks:

- Taker exits may occur during worse liquidity than entries.
- Stop-like behavior may require crossing the spread.
- 1m candles can hide intraminute spread and slippage spikes.
- High-volume/high-volatility periods may improve opportunity but also increase adverse execution cost.
- Market impact is configured as zero in the 10, 7, and 5 bps sensitivity cases, which is optimistic unless order size is negligible.

Conclusion: the stress case argues against treating a `0.32487040` bps 10 bps result as meaningful. It can be erased by a tiny cost miss.

## 10. Candle-derived proxy review

Local BTCUSDT 1m OHLCV files were available for 2020 through 2025, so a lightweight proxy analysis was computed. These are crude candle proxies only:

- `high_low_range_bps = (high - low) / midpoint(open, close) * 10000`
- `open_to_close_abs_return_bps = abs(close - open) / open * 10000`
- `close_to_close_abs_return_bps = abs(close_t - close_t-1) / close_t-1 * 10000`
- volume buckets were derived from local candle volume quantiles
- session buckets used UTC hour ranges

Overall proxy distribution across `3,156,480` 1m candles:

| Metric | p50 | p75 | p90 | p95 | p99 |
|---|---:|---:|---:|---:|---:|
| high_low_range_bps | 6.7846 | 12.0161 | 19.8278 | 27.0253 | 50.6273 |
| open_to_close_abs_return_bps | 3.2926 | 6.7754 | 12.1092 | 16.9778 | 32.8580 |
| close_to_close_abs_return_bps | 3.2961 | 6.7804 | 12.1142 | 16.9838 | 32.8834 |

Median proxies by local volume bucket:

| Volume bucket | Rows | Median high-low bps | Median open-close abs bps | Median close-close abs bps |
|---|---:|---:|---:|---:|
| bottom 30% | 946,945 | 2.9933 | 1.5715 | 1.5746 |
| middle 40% | 1,262,591 | 6.8220 | 3.3794 | 3.3833 |
| top 30% | 946,944 | 14.3182 | 7.0447 | 7.0450 |

Median proxies by UTC session bucket:

| Session bucket | Rows | Median high-low bps | Median open-close abs bps | Median close-close abs bps |
|---|---:|---:|---:|---:|
| Asia 00–08 UTC | 1,052,160 | 5.9051 | 2.9304 | 2.9345 |
| London 08–13 UTC | 657,600 | 6.1927 | 3.0525 | 3.0542 |
| US/NY 13–21 UTC | 1,052,160 | 8.3972 | 3.9286 | 3.9309 |
| Other 21–24 UTC | 394,560 | 6.3333 | 3.2099 | 3.2135 |

Interpretation:

- The median 1m high-low range of `6.7846` bps is already close to the entire 7 bps round-trip scenario, but candle range is not the same as execution cost.
- High-volume candles have a much larger median high-low range (`14.3182` bps), suggesting that the periods with more activity can also contain enough intraminute movement to overwhelm a small residual edge if execution is poorly timed.
- US/NY session candles show higher median movement than Asia or London in this proxy set.

Limitations:

- OHLCV candles do not provide order book spread.
- OHLCV candles do not provide queue position.
- OHLCV candles do not prove maker fill probability.
- OHLCV candles do not identify whether a strategy could enter at open, close, bid, ask, midpoint, VWAP, or a limit price.
- Candle ranges can overestimate or underestimate actual execution cost depending on order type, urgency, and fill timing.
- These proxies are useful for caution, not for proving venue-level 7–10 bps executability.

## 11. Executability review of current label

The current forecast label is:

```text
future_return_after_cost_bps over 4h horizon
```

Executability concerns:

- The label assumes an entry and exit reference price derived from historical candles, but the project does not yet prove those prices are obtainable by the intended strategy.
- The label subtracts a static cost model, but it does not model waiting time for passive orders.
- It does not model maker fill probability.
- It does not model partial fills.
- It does not model adverse selection after passive fills.
- It does not model order cancellation, stale signals, or missed entries.
- A 4h horizon over 1m rows creates heavy overlap between adjacent predictions, so bucket-level evidence can overstate independent opportunity count.
- The project does not currently model turnover and trade frequency for this forecast filter.
- A positive top-decile bucket may not survive realistic position lifecycle constraints, especially if entry delay, exit urgency, spread crossing, and non-fills are included.

Because of these gaps, the current 4h label should remain a research label. It is not yet an executable trade lifecycle model.

## 12. Decision against F8 residual edge

Required F9 answers:

1. **Is 10 bps realistically achievable?**
   - Possibly, under favorable fee tier, low slippage, low spread, and/or mixed maker/taker execution. However, the repo lacks realized execution evidence. More importantly, the F8 residual edge at 10 bps is only `0.32487040` bps, so it is not robust.

2. **Is 7 bps realistically achievable?**
   - Possibly, but only under optimistic maker-assisted or very low-slippage assumptions. The repo does not prove that the model's top-decile timestamps can be executed at 7 bps round trip.

3. **Is 5 bps too optimistic?**
   - Yes for current evidence. It may be possible in a highly favorable maker-heavy setup, but this project does not currently model queue position, fill probability, adverse selection, or realized spread capture. It should not be used as a decision basis.

4. **Is the expected residual edge large enough after cost uncertainty?**
   - No. The 10 bps residual is effectively noise-sized. The 7 bps residual is positive but small relative to candle movement proxies and unmeasured execution uncertainty. The 5 bps residual is larger but depends on a fragile cost assumption.

Decision mapping:

- 7–10 bps is not fully defensible from repo evidence.
- 10 bps is barely positive.
- 7 bps needs optimistic assumptions.
- Monotonicity remains weak.

Therefore the conservative decision is `needs_real_fee_slippage_data_before_decision`.

## 13. Final decision

**Final decision: `needs_real_fee_slippage_data_before_decision`.**

Rationale:

- The F8 10 bps case is too close to zero to be meaningful.
- The F8 7 bps case is positive but requires assumptions that are not evidenced by repo data.
- The 5 bps case is too optimistic for current evidence.
- The current label is not an executable lifecycle model.
- The F8 monotonicity gate failed at `0.44444444` versus the required `0.60`.
- Candle-derived proxies reinforce that small bps-level edges can be erased by ordinary 1m movement and execution uncertainty.

## 14. Next phase recommendation

Recommended next phase:

```text
F10 — Real Fee/Slippage Evidence Collection or Label Redesign Prep
```

Suggested scope:

1. Collect real venue fee-tier assumptions in static config or documentation.
2. Add a research-only realized spread/slippage evidence table if order book or execution logs become available.
3. If real execution evidence is unavailable, redesign labels to avoid depending on fragile 7–10 bps residual assumptions.
4. Keep forecast output disconnected from strategy and backtest execution until a cost-adjusted, executable label is defensible.
5. Preserve the conservative candidate gate: positive top decile alone is insufficient without monotonicity, sufficient residual edge, and executable cost assumptions.

Do not proceed to a cost-adjusted backtest-filter experiment unless real evidence shows that 7–10 bps is achievable and the expected residual edge is meaningfully above uncertainty.

## 15. Commands run

```bash
python3 <inline candle proxy script reading data/historical/BTCUSDT/1m/BTCUSDT-1m-202*.csv>
cargo fmt --check
cargo test
```

## 16. Boundary confirmation

- no strategy logic changed
- no ForecastScorer added
- no backtest filter added
- no paper/live execution added
- no profitability claim made
