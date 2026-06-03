# Northflow — Deterministic Crypto Trading Research Core

A pure Rust CLI and library for deterministic, research-first crypto strategy backtesting.

## Current phase: Phase 7 — Reports and Attribution ✓

| Phase | Status |
|---|---|
| Phase 1 — Core Domain (Candle, Signal, Order, Trade …) | ✅ Complete |
| Phase 2 — Market Data (OHLCV loader, timeframe builder, data quality) | ✅ Complete |
| Phase 3 — Indicators (EMA 8/21/50/200, ATR 14, VWAP, Volume SMA 20) | ✅ Complete |
| Phase 4 — Strategy Engine (screened_vwap_scalp) | ✅ Complete |
| Phase 5 — Risk & Cost model | ✅ Complete |
| Phase 6 — Backtest engine | ✅ Complete |
| Phase 7 — Reports & Attribution | ✅ Implemented |

See `docs/ROADMAP.md` for full roadmap and architecture decisions.

---

## Phase 7 — Reports and Attribution

Phase 7 is the final research-core phase. Every trade is now explainable and auditable back to its original signal.

**Backtest output: simulated `Trade` records only.**  
No live orders. No paper trading. No exchange calls. No LLM trading decisions.

### Traceability chain

Every trade traces back through the full ID chain:

```
signal_id → order_id → fill_id → position_id → exit_order_id → trade_id
```

Deterministic format: `SIG-BT-00000001`, `TRD-SIG-BT-00000001`, `POS-SIG-BT-00000001`, …  
No random IDs. No UUID dependency. No system time.

### Report files

After `cargo run -- research --config config/research.toml` with valid historical CSV data:

| File | Description |
|---|---|
| `reports/backtest_summary.json` | Aggregate backtest metrics (win rate, net PnL, drawdown, …) |
| `reports/trades.csv` | Every simulated trade including signal_id, filters, edge bps |
| `reports/equity_curve.csv` | Equity at each closed trade |
| `reports/attribution_summary.json` | Overall attribution statistics (avg edge, realization, filter counts) |
| `reports/attribution_by_regime.csv` | Win rate, PnL, edge per market regime (bullish/bearish/neutral) |
| `reports/attribution_by_exit_reason.csv` | Win rate, PnL, edge grouped by exit reason (stop_loss/take_profit/time_exit/…) |
| `reports/attribution_by_side.csv` | Win rate, PnL, edge grouped by trade direction (long/short) |
| `reports/attribution_by_filter.csv` | Win rate, PnL, edge per passed or failed strategy filter |
| `reports/audit_report.json` | Audit validation result — errors, warnings, and per-trade issues |
| `reports/report_manifest.json` | Deterministic list of all generated files with row counts |

Attribution CSV files share a stable header:

```
key,trades,wins,losses,win_rate,net_pnl,gross_pnl,total_fee,total_slippage,avg_net_pnl,avg_expected_edge_bps,avg_actual_edge_bps,avg_bars_held
```

Filter bucket keys:
- `passed:<filter_name>` — strategy filters that approved the signal
- `failed:<filter_name>` — strategy filters that rejected the signal

### Audit validation

Every trade is validated by `ReportAuditor`:

- **Errors** — broken attribution or invalid trade fields; `audit_report.passed = false`.  
  Examples: empty `trade_id`, `trade_id` does not embed `signal_id`, negative fee, non-finite PnL, duplicate signal IDs.

- **Warnings** — incomplete but non-fatal explainability; audit still passes.  
  Examples: `filters_passed` is empty, `expected_edge_bps <= 0`, `bars_held == 0`.

If errors are found, `audit_report.json` is still written and attribution files are still generated. Only file I/O failures stop report writing.

### Paper and live modes remain disabled

Later phases may add paper trading, live trading, and an AI advisor mode.  
**AI must not decide entries, SL/TP, or position size.**  
Those decisions must remain deterministic and rule-based.

```
northflow paper   # exits with error — research engine not yet validated for paper
northflow live    # exits with error — paper/live parity not yet proven
```

---

## Phase 6 — Backtest Engine

Phase 6 is a deterministic historical simulation only.

### Execution rules

- Entry is simulated at the **next 1m candle open** after signal generation.
- **No-lookahead rule**: 5m and 15m candles are only used once they are fully closed and their close time is at or before the current 1m candle's signal time.
- **Conservative intrabar rule**: if stop-loss and take-profit are both touched in the same candle, stop-loss is assumed to have been hit first.
- After entry at the next candle open, SL/TP checks run on that same entry candle.
- No new strategy signal is evaluated on the candle where an entry was just opened.

---

## Phase 5 — Risk and Cost Model

Phase 5 validates a `Signal` against risk limits and calculates a safe theoretical quantity.

**Risk model output: `RiskAssessment` only.**  
No orders. No fills. No positions. No backtest execution.

### Position sizing

```
risk_amount         = equity × risk_per_trade_pct / 100
qty_by_risk         = risk_amount / |entry − stop_loss|
max_qty_by_leverage = equity × max_leverage / entry
qty                 = min(qty_by_risk, max_qty_by_leverage)
```

### Cost model components

| Component | Formula |
|---|---|
| Entry fee | `entry_notional × taker_fee_bps / 10000` |
| Exit fee | `exit_notional × taker_fee_bps / 10000` |
| Spread | `avg_notional × spread_bps / 10000` |
| Slippage | `avg_notional × slippage_bps / 10000 × 2` |
| Market impact | `avg_notional × market_impact_bps / 10000` |
| Stop slippage | `avg_notional × stop_slippage_bps / 10000` |

### Risk guards

| Guard | Reject condition |
|---|---|
| Max open positions | `open_positions >= max_open_positions` |
| Daily loss | `abs(min(daily_pnl, 0)) / equity × 100 >= max_daily_loss_pct` |
| Max drawdown | `(peak − equity) / peak × 100 >= max_drawdown_pct` |
| Min reward/risk | `signal.reward_risk() < min_reward_risk` |
| Net edge | `expected_reward_bps − total_adverse_cost_bps <= 0` |

---

## Phase 4 — Strategy Engine

The first active strategy is `screened_vwap_scalp`.

**Strategy output: `Signal` only.**

### Timeframe roles (explicit — never inferred from order)

| Role | Timeframe | Purpose |
|---|---|---|
| `entry_timeframe` | 1m | Entry and execution signal |
| `confirmation_timeframe` | 5m | Intermediate confirmation |
| `screening_timeframe` | 15m | Market regime / bias filter |

### screened_vwap_scalp rules

**Regime classification (15m / 5m):**