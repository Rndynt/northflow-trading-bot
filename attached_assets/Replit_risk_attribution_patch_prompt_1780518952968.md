# Northflow Backtest Realism & Risk Attribution Patch Prompt

You are working on this repository:

https://github.com/Rndynt/northflow-crypto-trading-bot

Your task is to implement a focused research-core hardening patch after Phase 7.

The engine is already able to run a full BTCUSDT 1m 2024 backtest and write reports. The latest run produced:

- 1m candles: 527,040
- 5m candles: 105,408
- 15m candles: 35,136
- data quality errors: 0
- total trades: 18
- win rate: 44.44%
- net PnL: about -241
- final equity: about 4,759 from initial 5,000
- audit passed: true
- audit warnings: 18
- attribution reports written successfully

The research core works. The next issue is analytical correctness and explainability, not new features.

Do not implement a new phase.

Do not tune the strategy.

Do not change indicator formulas.

Do not change risk sizing formulas except where actual-entry revalidation requires using the real entry price.

Do not implement paper/live/exchange/LLM/dashboard/Telegram.

This patch is only for:

1. Risk rejection attribution.
2. Signal flow summary.
3. Actual-entry re-risking.
4. Effective reward/risk reporting.
5. Reducing false audit warnings for accepted trades with empty filters_failed.

## Files to read first

Read these files before changing anything:

- AGENTS.md
- docs/ROADMAP.md
- README.md
- docs/DATA_DOWNLOAD.md
- config/research.toml
- src/backtest/engine.rs
- src/backtest/fill_model.rs
- src/backtest/report.rs
- src/backtest/metrics.rs
- src/research/mod.rs
- src/report/mod.rs
- src/report/attribution.rs
- src/report/audit.rs
- src/report/manifest.rs
- src/report/validation.rs
- src/risk/guard.rs
- src/risk/position_sizing.rs
- src/risk/cost_model.rs
- src/core/signal.rs
- src/core/trade.rs

## Problem summary

The current BTCUSDT 2024 result shows only 18 trades, all happening early in the year, then no more trades.

Likely cause:

- max_drawdown_pct guard is reached around 5%.
- RiskEngine then rejects all later signals.
- Current reports do not show generated signals, approved signals, or risk rejection reasons.

There is also a realism issue:

- Strategy creates Signal from signal candle close.
- Backtest enters at next 1m candle open with adverse slippage.
- Current position sizing/risk approval can be based on the signal entry price, not the actual entry fill price.
- SL/TP remain based on the original signal geometry.
- This can distort actual reward/risk, position size, expected edge, and realized edge.

This must be fixed before any strategy tuning.

## Strict scope

Allowed:

- Add risk rejection report files.
- Add signal flow summary report.
- Revalidate risk at the actual next-open entry price.
- Add effective reward/risk fields to Trade/report output if needed.
- Add tests.
- Update docs.

Forbidden:

- No new strategy rules.
- No new indicators.
- No parameter optimization.
- No automatic tuning.
- No paper trading.
- No live trading.
- No exchange API.
- No websocket.
- No database.
- No dashboard.
- No Telegram.
- No LLM trading decisions.
- No AI advisor.
- No profitability claims.

## Required output files

After:

```bash
cargo run --release -- research --config config/research.toml
```

with valid historical data, reports should now include these additional files:

```text
reports/risk_rejections.csv
reports/signal_flow_summary.json
```

Existing report files must remain:

```text
reports/backtest_summary.json
reports/trades.csv
reports/equity_curve.csv
reports/attribution_summary.json
reports/attribution_by_regime.csv
reports/attribution_by_exit_reason.csv
reports/attribution_by_side.csv
reports/attribution_by_filter.csv
reports/audit_report.json
reports/report_manifest.json
```

Update report_manifest.json to include the new files and correct row counts.

## Part 1 - Add RiskRejection model

Add a deterministic model for risk rejection attribution.

Recommended location:

```text
src/backtest/engine.rs
```

or cleaner:

```text
src/backtest/risk_trace.rs
```

If creating a new file, update:

```text
src/backtest/mod.rs
```

Recommended types:

```rust
#[derive(Debug, Clone)]
pub struct RiskRejection {
    pub signal_id: String,
    pub timestamp: i64,
    pub side: String,
    pub regime: String,
    pub reason: String,
    pub equity: f64,
    pub peak_equity: f64,
    pub drawdown_pct: f64,
    pub daily_realized_pnl: f64,
    pub expected_reward_bps: f64,
    pub expected_cost_bps: f64,
    pub expected_net_edge_bps: f64,
}
```

Use stable reason strings.

RiskEngine already returns `RiskAssessment` with `failed: Vec<String>` for normal rejection.

For every normal rejected signal:

- create one RiskRejection row per failed reason, or
- create one row with reasons joined by `|`.

Preferred for analysis:

- one row per failed reason.

Example reasons:

```text
max_open_positions_reached
daily_loss_limit_reached
max_drawdown_reached
reward_risk_below_minimum
expected_net_edge_not_positive
actual_entry_invalid_geometry
actual_entry_risk_error
```

Do not use random IDs.

Do not use system time.

## Part 2 - Add SignalFlowSummary

Add a signal flow summary model.

Recommended type:

```rust
#[derive(Debug, Clone)]
pub struct SignalFlowSummary {
    pub signals_generated: usize,
    pub signals_preapproved: usize,
    pub signals_rejected_initial_risk: usize,
    pub signals_rejected_actual_entry: usize,
    pub trades_opened: usize,
    pub trades_closed: usize,
    pub risk_rejections: usize,
    pub rejections_max_drawdown: usize,
    pub rejections_daily_loss: usize,
    pub rejections_reward_risk: usize,
    pub rejections_expected_net_edge: usize,
    pub rejections_other: usize,
}
```

Definitions:

- signals_generated: strategy returned `Ok(Some(signal))`.
- signals_preapproved: signal passed initial risk check at signal candle close.
- signals_rejected_initial_risk: initial RiskEngine assessment returned approved=false.
- signals_rejected_actual_entry: signal passed initial risk but failed after adjusting to actual next-open entry.
- trades_opened: simulated position was opened.
- trades_closed: final closed trades count.
- risk_rejections: total RiskRejection rows or total rejected signals; choose one and document it. Preferred: total RiskRejection rows.
- rejection counters should count failed reason occurrences.

Add this to BacktestResult:

```rust
pub struct BacktestResult {
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<EquityPoint>,
    pub summary: BacktestSummary,
    pub risk_rejections: Vec<RiskRejection>,
    pub signal_flow: SignalFlowSummary,
}
```

Update all existing tests that construct or inspect BacktestResult.

## Part 3 - Initial risk assessment must record rejections

Currently, when strategy returns a Signal, the engine calls risk assessment and either stores pending entry or skips.

Change the flow so normal risk rejection is recorded.

Current behavior conceptually:

```rust
match try_assess_risk(...) {
    Some((sig, qty)) => pending_entry = Some((sig, qty)),
    None => {}
}
```

New behavior:

1. On `Ok(Some(signal))` from strategy:
   - increment `signals_generated`.
   - call RiskEngine::assess.
2. If assessment returns Err:
   - invalid config/context should return Err and stop backtest.
   - invalid signal from strategy should return Err and stop backtest.
3. If assessment approved=false:
   - increment `signals_rejected_initial_risk`.
   - record RiskRejection rows using `assessment.failed`.
   - do not set pending entry.
4. If approved=true:
   - increment `signals_preapproved`.
   - set pending entry.

Important:

For pending entry, do not rely permanently on the quantity calculated from the signal close. Actual entry price must be re-risked in Part 4.

Recommended pending type:

```rust
let mut pending_entry: Option<Signal> = None;
```

instead of:

```rust
Option<(Signal, f64)>
```

because final qty should be calculated after actual entry adjustment.

## Part 4 - Re-risk at actual next-open entry price

This is critical.

Current flow:

- Signal created on candle i using close price.
- Entry occurs on candle i+1 open with adverse slippage.
- Existing signal SL/TP are kept.
- Position may use qty derived from old signal entry price.

Required flow:

1. Signal generated on candle i.
2. Initial risk assessment is performed to reject obvious unsafe signals at signal time.
3. If initially approved, store pending Signal.
4. On candle i+1:
   - compute actual adverse entry price from candle open and side.
   - clone/adjust the signal to actual entry.
   - re-run RiskEngine on adjusted signal and current equity/risk context.
   - if actual-entry assessment is rejected:
     - increment `signals_rejected_actual_entry`.
     - record RiskRejection rows.
     - do not open position.
   - if actual-entry assessment returns Err due to adjusted signal geometry:
     - treat it as actual-entry rejection, not fatal.
     - record reason `actual_entry_invalid_geometry`.
     - continue backtest.
   - if actual-entry assessment returns Err due to invalid config/context/cost:
     - return Err.
   - if approved:
     - use the qty from actual-entry assessment.
     - simulate entry.
     - open position.

### Actual entry price calculation

Add a helper to `src/backtest/fill_model.rs`:

```rust
impl FillModel {
    pub fn adverse_entry_price(side: Side, open_price: f64, slippage_bps: f64) -> f64 {
        match side {
            Side::Long => open_price * (1.0 + slippage_bps / 10_000.0),
            Side::Short => open_price * (1.0 - slippage_bps / 10_000.0),
        }
    }
}
```

Use the same logic as `simulate_entry` so the adjusted signal entry price matches the actual fill price.

Then `simulate_entry` should use this helper internally to avoid divergence.

### Adjusted signal fields

Create helper:

```rust
fn adjusted_signal_for_actual_entry(signal: &Signal, actual_entry_price: f64) -> Signal
```

It should clone the signal and update:

```rust
entry_price = actual_entry_price
```

Also recalculate edge fields deterministically:

For long:

```text
expected_reward_bps = (take_profit - actual_entry_price) / actual_entry_price * 10000
```

For short:

```text
expected_reward_bps = (actual_entry_price - take_profit) / actual_entry_price * 10000
```

Then:

```text
expected_net_edge_bps = expected_reward_bps - estimated_cost_bps
```

Do not modify:

- signal_id
- symbol
- strategy_id
- side
- timeframes
- entry_time
- stop_loss
- take_profit
- confidence
- regime
- entry_reason
- filters_passed
- filters_failed

### Invalid actual-entry geometry

If actual entry makes geometry invalid, this should be a rejected signal, not a panic.

Examples:

Long invalid:

```text
actual_entry_price <= stop_loss
actual_entry_price >= take_profit
```

Short invalid:

```text
actual_entry_price >= stop_loss
actual_entry_price <= take_profit
```

Record reason:

```text
actual_entry_invalid_geometry
```

Do not open a position.

Do not count as a fatal backtest error.

But invalid risk config/context/cost should still return Err.

## Part 5 - Trade must reflect effective reward/risk

The current report shows `reward_risk`, but it may not represent actual entry reward/risk if entry moved.

After actual-entry re-risking, `Trade.reward_risk` should be based on actual entry price.

Currently `build_trade` already computes:

```rust
let risk = (pos.entry_price - pos.signal.stop_loss).abs();
let reward = (pos.signal.take_profit - pos.entry_price).abs();
let reward_risk = if risk > 0.0 { reward / risk } else { 0.0 };
```

This is correct if `pos.signal.entry_price` and `pos.entry_price` are both adjusted consistently.

Ensure:

- OpenSimPosition.signal contains the adjusted signal.
- pos.entry_price equals actual fill price from the same adjusted entry.
- Trade.reward_risk reflects actual effective reward/risk.

Optional but recommended:

Add explicit trade CSV field:

```text
effective_reward_risk
```

If adding this field requires changing Trade struct significantly, you may keep existing `reward_risk` as effective reward/risk and document that it is effective at actual fill.

Do not remove the existing `reward_risk` column.

## Part 6 - Write risk_rejections.csv

Add writer support.

Recommended location:

- `src/backtest/report.rs` or `src/report/attribution.rs`

Preferred:

- `src/backtest/report.rs` writes base backtest outputs including risk_rejections.csv and signal_flow_summary.json.
- Phase 7 manifest includes them.

CSV header:

```csv
signal_id,timestamp,side,regime,reason,equity,peak_equity,drawdown_pct,daily_realized_pnl,expected_reward_bps,expected_cost_bps,expected_net_edge_bps
```

Rules:

- Write header even if empty.
- Use stable ordering by occurrence in backtest.
- Escape CSV fields.
- All floats must be finite. If not finite, write 0 or return error; prefer returning error for invalid internal state.

## Part 7 - Write signal_flow_summary.json

JSON fields:

```json
{
  "signals_generated": 0,
  "signals_preapproved": 0,
  "signals_rejected_initial_risk": 0,
  "signals_rejected_actual_entry": 0,
  "trades_opened": 0,
  "trades_closed": 0,
  "risk_rejections": 0,
  "rejections_max_drawdown": 0,
  "rejections_daily_loss": 0,
  "rejections_reward_risk": 0,
  "rejections_expected_net_edge": 0,
  "rejections_other": 0
}
```

No serde required.

Manual JSON formatting is okay.

Use deterministic field order exactly like above.

## Part 8 - Update manifest

Update `src/report/manifest.rs`.

Manifest must include:

```text
reports/risk_rejections.csv
reports/signal_flow_summary.json
```

Row counts:

- risk_rejections.csv rows = risk_rejections.len()
- signal_flow_summary.json rows = 1

Manifest paths must remain relative and not absolute.

## Part 9 - Update research CLI output

Update `src/research/mod.rs`.

After backtest completes, print signal flow summary:

```text
Signal flow:
  generated:              X
  preapproved:            X
  rejected initial risk:  X
  rejected actual entry:  X
  trades opened:          X
  trades closed:          X
  risk rejection rows:    X
```

Also print top rejection reason counts:

```text
Risk rejections:
  max_drawdown:           X
  daily_loss:             X
  reward_risk:            X
  expected_net_edge:      X
  other:                  X
```

When writing reports, include:

```text
reports/risk_rejections.csv
reports/signal_flow_summary.json
```

Keep existing output.

## Part 10 - Reduce false audit warnings

Currently audit produces warnings for every trade because `filters_failed` is empty.

For an approved trade, empty `filters_failed` is normal.

Change audit behavior:

- `filters_passed` empty: keep warning.
- `filters_failed` empty: do not warn by default.
- If you want to preserve visibility, add an Info issue instead of Warning.

The final BTCUSDT run should no longer show 18 warnings solely because `filters_failed` is empty.

Do not weaken errors.

Do not suppress real invalid trade errors.

## Part 11 - Documentation update

Update README.md and docs/DATA_DOWNLOAD.md briefly.

Add a short section explaining:

- `risk_rejections.csv`
- `signal_flow_summary.json`
- actual-entry risk revalidation
- `reward_risk` in trades.csv is effective reward/risk at simulated entry fill
- release mode remains recommended for large datasets

Do not rewrite the whole README.

Do not remove existing content.

## Required tests

Add tests for the new behavior.

### Actual entry adjustment tests

- adjusted_signal_for_actual_entry_long_recalculates_expected_reward
- adjusted_signal_for_actual_entry_short_recalculates_expected_reward
- actual_entry_invalid_long_geometry_is_rejected_not_fatal
- actual_entry_invalid_short_geometry_is_rejected_not_fatal
- fill_model_adverse_entry_price_matches_simulate_entry_long
- fill_model_adverse_entry_price_matches_simulate_entry_short

### Risk rejection tests

- initial_risk_rejection_is_recorded
- actual_entry_risk_rejection_is_recorded
- risk_rejection_reason_counts_are_stable
- signal_flow_counts_generated_preapproved_rejected_opened_closed

### Report tests

- writes_risk_rejections_csv_with_header
- writes_empty_risk_rejections_csv_with_header
- writes_signal_flow_summary_json
- manifest_includes_risk_rejections_and_signal_flow
- risk_rejections_csv_escapes_fields

### Audit tests

- audit_does_not_warn_when_filters_failed_empty_for_approved_trade
- audit_still_warns_when_filters_passed_empty
- audit_still_errors_on_invalid_trade_id
- audit_still_errors_on_negative_fee

Existing tests must continue to pass.

## Required commands

Run:

```bash
cargo fmt
cargo build
cargo test
cargo run -- help
```

If `data/historical/BTCUSDT.csv` exists, also run:

```bash
cargo run --release -- research --config config/research.toml
```

Expected for real BTCUSDT 2024 data:

- reports are generated successfully.
- new files exist:
  - reports/risk_rejections.csv
  - reports/signal_flow_summary.json
- CLI prints signal flow.
- audit warnings are reduced if they were only caused by empty filters_failed.
- no paper/live/exchange/LLM behavior is added.

## Expected final result

At the end of this patch:

- The engine reports generated, approved, rejected, opened, and closed signal flow.
- Risk rejection reasons are exported.
- Backtest no longer hides why trading stopped after drawdown guard.
- Position sizing is based on actual next-open entry price, not stale signal close price.
- Effective reward/risk uses actual simulated entry price.
- Audit warnings are less noisy.
- Existing Phase 1-7 reports still work.
- No strategy tuning is introduced.

## Commit message suggestion

research: add risk rejection attribution
