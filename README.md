# Northflow — Deterministic Crypto Trading Research Core

A pure Rust CLI and library for deterministic, research-first crypto strategy backtesting.

## Current phase: Phase 1 — Core Domain Foundation ✓

Phase 1 is complete. The core trading domain types are implemented with full unit test coverage.
Phase 2 (market data loader + timeframe builder) is next.

See `docs/ROADMAP.md` for the full roadmap.

## Key rules

### Signal ID is mandatory

Every `Signal` must have a `signal_id`. All downstream objects trace back to it:

```
signal_id → order_id → fill_id → position_id → exit_order_id → trade_id
```

ID examples:
```
SIG-BT-00000001
ORD-SIG-BT-00000001-ENTRY
ORD-SIG-BT-00000001-SL
ORD-SIG-BT-00000001-TP
TRD-SIG-BT-00000001
```

### Timeframe roles are explicit

Timeframe roles are declared explicitly in config — never inferred from array order:

```toml
entry_timeframe        = "1m"   # entry and execution signals
screening_timeframe    = "15m"  # market regime / bias filter
confirmation_timeframe = "5m"   # intermediate confirmation layer
```

### Paper and live modes are disabled

```
northflow paper   # exits with error — research engine not yet validated
northflow live    # exits with error — paper/live parity not yet proven
```

Paper and live modes will be enabled only after the research engine produces
validated, truthful backtest results.

### Legacy code is reference-only

Previous code under `legacy/aria/` is preserved for reference only.
The active `src/` tree must never import from `legacy/`.
See `legacy/README.md` for details.

## Design principles

- Research and validation before any live or paper trading
- Zero external dependencies — pure Rust std only
- Deterministic simulation: same config + same data = same result, always
- `signal_id` mandatory on every signal for full attribution chain
- Explicit timeframe roles — never infer from array order

## Project structure

```
northflow-crypto-trading-bot/
├── src/
│   ├── lib.rs              — public module exports
│   ├── main.rs             — CLI entry point
│   ├── core/               — Phase 1: core trading domain types
│   │   ├── candle.rs       — Candle (OHLCV + validation)
│   │   ├── side.rs         — Side::Long / Side::Short
│   │   ├── symbol.rs       — Symbol (validated ticker)
│   │   ├── timeframe.rs    — Timeframe (1m/5m/15m/1h + parsing)
│   │   ├── signal.rs       — Signal (mandatory signal_id, 3 TF roles)
│   │   ├── order.rs        — Order, OrderType, OrderStatus
│   │   ├── fill.rs         — Fill (executed order record)
│   │   ├── position.rs     — Position + unrealized PnL
│   │   ├── trade.rs        — Trade (final closed result)
│   │   └── error.rs        — NorthflowError
│   ├── config/             — ResearchConfig (parsed from TOML, no serde)
│   ├── data/               — CSV OHLCV loader (flexible header detection)
│   ├── indicators/         — EMA, ATR, VWAP (streaming structs)
│   ├── strategy/           — Phase 4 placeholder (screened_vwap_scalp)
│   ├── risk/               — Phase 5 placeholder (sizing + drawdown guards)
│   ├── execution/          — Phase 6 placeholder (SimExecutor)
│   ├── research/           — Phase 2+ orchestrator
│   ├── report/             — Phase 7 placeholder (JSON + CSV writers)
│   ├── journal/            — placeholder (not active)
│   └── advisor/            — placeholder (not active)
├── config/
│   └── research.toml       — default research config
├── data/
│   └── historical/         — place OHLCV CSV files here: <SYMBOL>.csv
├── legacy/
│   ├── README.md           — legacy boundary rules
│   └── aria/               — previous ARIA/crypto-scalper code (reference only)
└── reports/                — output (Phase 7): summary.json, trades.csv, equity_curve.csv
```

## Quick start

```bash
# Build
cargo build --release

# Phase 1 status (prints core domain ready message)
cargo run -- research --config config/research.toml

# Print help
cargo run -- help

# Run all tests (61 tests, Phase 1)
cargo test
```

## CSV data format

Header must include: `timestamp`, `open`, `high`, `low`, `close`, `volume`

```
timestamp,open,high,low,close,volume
1704067200000,42150.0,42800.0,41900.0,42600.0,1234.5
```

## Config reference (`config/research.toml`)

| Key | Section | Description |
|-----|---------|-------------|
| `symbols` | `[pairs]` | List of symbols, e.g. `["BTCUSDT"]` |
| `entry_timeframe` | `[pairs]` | Entry/execution timeframe (must be "1m") |
| `screening_timeframe` | `[pairs]` | Regime bias timeframe (must be "15m") |
| `confirmation_timeframe` | `[pairs]` | Confirmation timeframe (must be "5m") |
| `data_dir` | `[backtest]` | Directory containing CSV files |
| `reports_dir` | `[backtest]` | Output directory for reports |
| `initial_equity_usd` | `[risk]` | Starting capital |
| `risk_per_trade_pct` | `[risk]` | % of equity risked per trade |
| `max_open_positions` | `[risk]` | Max simultaneous positions |
| `max_leverage` | `[risk]` | Max notional leverage |
| `min_reward_risk` | `[risk]` | Minimum R:R ratio to take a trade |
| `max_daily_loss_pct` | `[risk]` | Daily loss circuit breaker |
| `max_drawdown_pct` | `[risk]` | Total drawdown circuit breaker |
| `taker_fee_bps` | `[cost]` | Taker fee in basis points |
| `slippage_bps` | `[cost]` | Slippage estimate in bps |
| `spread_bps` | `[cost]` | Spread cost in bps |
| `market_impact_bps` | `[cost]` | Market impact estimate in bps |
| `conservative_intrabar` | `[backtest]` | Use worst-case intrabar fill |
| `min_confidence` | `[strategy]` | Minimum signal confidence (0–100) |

## Strictly forbidden (current phase and beyond)

- React app, TypeScript app, dashboard, web UI
- Telegram integration
- LLM trading decision
- Manager agent, learning agent, survival agent, orchestrator
- Live exchange order placement
- Paper trading loop (until research validated)
- Multi-strategy router
- Portfolio optimizer
- 100x leverage logic

## Push to GitHub

```bash
git remote set-url origin https://github.com/Rndynt/northflow-crypto-trading-bot.git
git push -u origin main
```

Use a GitHub PAT with `repo` scope when prompted for a password.
