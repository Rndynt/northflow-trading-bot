# Dynamic Config & Engine Independence Report

## Baseline inspection

- The previous parser read symbols/timeframe roles from top-level or `[timeframes]`, while the active preset stores them under `[pairs]`.
- Single strategy mode printed readiness text and returned without looping over configured symbols.
- `BacktestEngine` previously owned orchestration concerns: global research config, data file lookup/loading, candle-store construction, and strategy registry resolution.
- The source data timeframe was implicitly treated as 1m.
- Runtime output contained fixed role descriptions such as `1m`, `5m`, `15m`, and `no lookahead across 5m / 15m`.

## Completed cleanup

- `ResearchConfig` now parses the preset sections used by `config/research.toml`: `[pairs]`, `[data]`, `[strategy]`, `[risk]`, `[cost]`, `[backtest]`, and `[historical_files]`.
- `[data].source_timeframe` is explicit and currently validates only `1m`, because higher timeframe candles are built from 1m source candles.
- Single mode resolves the selected strategy and attempts each configured symbol through the normal symbol runner.
- `BacktestEngine` now accepts `BacktestRunInput` with prepared `CandleStore`, timeframe roles, risk/cost/backtest config, and a provided `dyn Strategy`.
- Strategy registry lookup, historical file loading, data quality validation, candle-store construction, and report writing are owned by the research orchestrator.
- Runtime output now prints configured entry, confirmation, screening, and source timeframes.
- Strategy ID is centralized at `src/strategy/ids.rs`.

## Remaining limitations

- Source data currently supports only `1m`.
- The engine remains single-position by policy.
- Only `basic_sample_strategy` is active intentionally.
- Paper and live modes remain disabled.
