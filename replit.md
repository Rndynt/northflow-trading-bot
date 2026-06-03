# Northflow Crypto Trading Bot

A deterministic, research-first crypto trading engine written in pure Rust. Its goal is to provide a truthful, verifiable backtesting and research environment for crypto strategies.

## Architecture

- **Language:** Rust (Edition 2024, requires v1.85+)
- **Dependencies:** None — pure Rust `std` library only, for full determinism
- **Mode:** CLI research tool only. Paper and live trading modes are intentionally disabled.

## Project Structure

- `src/core/` — Domain types (Candle, Signal, Order, Fill, Position, Trade)
- `src/market/` — Data loading (CSV), validation, timeframe aggregation
- `src/indicators/` — Deterministic indicators (EMA, ATR, VWAP, Volume SMA)
- `src/strategy/` — Strategy logic (`screened_vwap_scalp`)
- `src/risk/` — Position sizing, cost models (fees, slippage, spread), risk guards
- `src/backtest/` — Replay engine and fill simulation
- `src/research/` — CLI orchestration and report generation
- `config/research.toml` — Primary configuration (symbols, timeframes, risk limits)
- `data/historical/` — Place 1m OHLCV CSV files here (format: `<SYMBOL>.csv`)
- `reports/` — Output JSON/CSV reports generated after a backtest run

## Running a Backtest

1. Place 1-minute OHLCV CSV data in `data/historical/<SYMBOL>.csv`
   - Required columns: `timestamp,open,high,low,close,volume` (or `open_time` for timestamp)
2. Edit `config/research.toml` to set symbols and parameters
3. Run:
   ```
   cargo run -- research --config config/research.toml
   ```

## Build

```
cargo build
```

Reports are written to the `reports/` directory after a successful run.

## User Preferences

- Keep the project pure Rust with no external crates unless explicitly requested.
- Maintain determinism as a core design principle.
