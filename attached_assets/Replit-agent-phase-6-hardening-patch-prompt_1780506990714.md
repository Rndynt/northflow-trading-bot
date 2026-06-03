# Northflow Phase 6 Mini Hardening Prompt

You are working on this repository:

https://github.com/Rndynt/northflow-crypto-trading-bot

Your task is to perform a small Phase 6 hardening patch only.

Phase 6 backtest engine is already implemented. This patch must fix several correctness and documentation issues before moving to Phase 7.

Do not implement Phase 7.

Do not implement report attribution beyond existing Phase 6 report files.

Do not implement live trading.

Do not implement paper trading.

Do not implement exchange APIs.

Do not implement dashboard, Telegram, or LLM trading decisions.

## Files to read first

Read these files before changing anything:

- AGENTS.md
- docs/ROADMAP.md
- README.md
- config/research.toml
- src/main.rs
- src/research/mod.rs
- src/backtest/mod.rs
- src/backtest/engine.rs
- src/backtest/fill_model.rs
- src/backtest/metrics.rs
- src/backtest/report.rs
- src/backtest/walk_forward.rs
- src/risk/guard.rs
- src/core/trade.rs
- src/core/signal.rs

## Current issues

Phase 6 is implemented, but the review found these issues:

1. README still says Phase 5 and marks Phase 6 as pending.
2. CLI help in src/main.rs still says Phase 5 and says no backtest execution.
3. BacktestEngine currently swallows RiskEngine errors.
4. After a pending entry is filled at the next candle open, the engine skips exit checks on that same entry candle.
5. Engine tests need stronger coverage for risk error propagation and same-candle exit after entry.

This patch must fix only those issues.

## Required fix 1 - Update README to Phase 6

Update README.md so it correctly states:

- Current phase is Phase 6 - Backtest Engine.
- Phase 1 core domain is complete.
- Phase 2 market data is complete.
- Phase 3 indicators are complete.
- Phase 4 strategy engine is complete.
- Phase 5 risk and cost model is complete.
- Phase 6 backtest engine is implemented.
- Phase 7 reports and attribution is still pending.

Add or update a Phase 6 section explaining:

- Backtest is historical simulation only.
- Research command writes:
  - reports/backtest_summary.json
  - reports/trades.csv
  - reports/equity_curve.csv
- Conservative intrabar rule:
  - if stop-loss and take-profit are both touched in the same candle, assume stop-loss was hit first.
- Entry is simulated at the next 1m candle open after signal.
- No-lookahead rule is enforced for 5m and 15m candles.
- Paper and live modes remain disabled.
- No exchange calls.
- No LLM trading decisions.
- Phase 7 attribution is still pending.

Do not mark Phase 7 complete.

Do not claim profitability.

Do not give trading advice.

## Required fix 2 - Update CLI help text to Phase 6

In src/main.rs, update the top file comments and print_help() output.

The help text currently says Phase 5 and no backtest execution.

It must say Phase 6.

Expected help wording:

Northflow Crypto Trading Bot

Usage:
  northflow research [--config config/research.toml]
  northflow paper   # disabled — research engine not yet validated for paper
  northflow live    # disabled — paper/live parity not yet proven

Phase 6: deterministic backtest engine ready.
         Backtest output: simulated Trade records only.
         Reports: reports/backtest_summary.json, reports/trades.csv, reports/equity_curve.csv
         No live orders, no paper trading, no exchange calls.
         Place 1m CSV data in data/historical/<SYMBOL>.csv
         Columns: timestamp,open,high,low,close,volume
         Alternative timestamp column: open_time

Keep paper and live disabled.

Do not change command parsing behavior.

## Required fix 3 - Propagate RiskEngine errors

In src/backtest/engine.rs, BacktestEngine currently handles RiskEngine::assess() like this:

Ok(assessment) if assessment.approved => { ... }
Ok(_) => {}
Err(_) => {} // risk error — skip signal

This is wrong.

Risk rejection is normal and may be skipped.

Risk error means invalid signal, invalid config, invalid context, or invalid cost calculation. It must not be silently swallowed.

Change it to:

Ok(assessment) if assessment.approved => { ... }
Ok(_) => {}
Err(e) => return Err(e)

Required behavior:

- Risk rejection returns Ok(RiskAssessment approved=false) and should skip entry.
- Risk error returns Err and should stop the backtest.
- Do not convert risk errors to Ok(None).
- Do not hide risk errors in logs only.

## Required fix 4 - Check same-candle exit after entry at candle open

In src/backtest/engine.rs, pending entries are filled at the next 1m candle open.

Currently, after filling pending_entry, the engine does:

continue;

This skips SL/TP checks on the entry candle.

That is unrealistic because after entering at the candle open, the candle high/low can hit stop-loss or take-profit during that same candle.

Fix this behavior.

Required behavior:

1. Signal generated on candle i.
2. Entry occurs on candle i+1 open.
3. After entry is created, check SL/TP on that same entry candle.
4. Conservative intrabar rule still applies:
   - if SL and TP are both touched in the entry candle, assume SL first.
5. Do not evaluate a new strategy signal on the same candle where an entry was just opened.
6. If the trade closes on the same candle, update equity and equity curve normally.
7. If the trade remains open, continue normal lifecycle on following candles.

Important:

- Do not create more than one position at a time.
- Do not evaluate a fresh signal on the same candle as entry.
- Do not skip exit checks after entry.
- Do not introduce lookahead.
- Do not enter and exit before the open. Entry occurs first at open, then SL/TP check uses that candle high/low.

Suggested approach:

Replace the immediate continue after pending entry with a flag:

let entered_this_bar = false;

When pending_entry is filled:

entered_this_bar = true;

Then allow the existing exit-check block to run.

After exit-check, skip strategy evaluation if entered_this_bar is true:

if entered_this_bar {
    continue;
}

If you need to restructure the loop, keep it simple and deterministic.

## Required fix 5 - Add tests for same-candle exit after entry

Add or improve tests in src/backtest/engine.rs and/or src/backtest/fill_model.rs.

At minimum, add tests that prove the fill model and engine behavior are correct.

Required tests:

- fill_model_can_exit_on_entry_candle_long_stop_loss
- fill_model_can_exit_on_entry_candle_long_take_profit
- fill_model_can_exit_on_entry_candle_short_stop_loss
- fill_model_can_exit_on_entry_candle_short_take_profit
- fill_model_entry_candle_both_sl_tp_assumes_stop_first
- engine_does_not_skip_exit_check_on_entry_candle

The fill model tests can use OpenSimPosition with bars_held = 0 or 1 and a candle whose high/low touches SL/TP.

For engine_does_not_skip_exit_check_on_entry_candle:

If creating a full strategy-triggering CSV is too complex, expose a small private helper or unit-testable function in engine.rs that handles:

- an existing OpenSimPosition
- an entry candle
- FillModel::check_exit
- equity update through build_trade

Keep the helper private if possible.

Do not weaken the test into only checking that the engine does not crash.

The test must actually verify that a same-candle SL/TP can close a trade.

## Required fix 6 - Add test for risk error propagation

Add a test proving RiskEngine errors are propagated by the backtest path.

Preferred target:

- create a small internal helper around the risk assess handling branch, or
- refactor the risk-assessment-to-pending-entry logic into a small private function that is unit-testable.

Required test name:

engine_propagates_risk_engine_error

Test intent:

- invalid risk config or invalid signal makes RiskEngine::assess() return Err.
- BacktestEngine logic must return Err.
- It must not silently skip the signal.

Avoid huge brittle CSV fixtures if possible.

Do not add public API unless necessary.

Do not bypass existing RiskEngine behavior.

## Required fix 7 - Ensure reports and research still work

Do not break src/research/mod.rs.

Research command must still:

- print Phase 6 title.
- load data when CSV exists.
- run BacktestEngine.
- write:
  - reports/backtest_summary.json
  - reports/trades.csv
  - reports/equity_curve.csv
- print friendly missing CSV message when no data exists.

If no CSV exists, research must not panic.

## Required fix 8 - Keep scope limited

Do not change strategy rules.

Do not retune screened_vwap_scalp.

Do not modify risk sizing formulas.

Do not modify indicator formulas.

Do not add parameter optimization.

Do not add walk-forward optimization.

Do not implement Phase 7.

Do not add new external dependencies unless absolutely necessary.

## Forbidden changes

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
- strategy optimizer
- portfolio optimizer
- 100x leverage logic
- synthetic candles
- interpolated candles
- exchange API integration
- websocket feed
- database requirement

Do not implement:

- live trading
- paper trading
- exchange adapters
- parameter optimization
- AI signal generation
- adaptive strategy tuning
- external broker integration
- notification systems
- Phase 7 attribution expansion

This is only a Phase 6 hardening patch.

## Required commands

Run:

cargo fmt
cargo build
cargo test
cargo run -- research --config config/research.toml
cargo run -- help

If valid CSV data exists, research must generate:

reports/backtest_summary.json
reports/trades.csv
reports/equity_curve.csv

If no CSV data exists, research must not panic and must print the friendly missing-data message.

Do not leave failing tests.

Do not leave TODO stubs.

## Expected final result

At the end of this patch:

- README says Phase 6 is current and implemented.
- README still marks Phase 7 as pending.
- CLI help says Phase 6.
- CLI help no longer says no backtest execution.
- RiskEngine Err is propagated from BacktestEngine.
- Risk rejection still skips normally.
- Entry at next candle open can exit on the same candle.
- Conservative SL-first rule still applies if SL and TP both touched.
- No new signal is evaluated on the entry candle.
- Tests cover same-candle entry/exit behavior.
- Tests cover risk error propagation.
- cargo fmt passes.
- cargo build passes.
- cargo test passes.
- cargo run -- research --config config/research.toml works.
- cargo run -- help works.

## Commit message suggestion

phase6: harden backtest execution flow
