# Phase: Multi-File Historical Data Loading

## Goal

Support historical backtests that read multiple yearly CSV files per symbol from config, instead of relying on one overwritten file such as `data/historical/BTCUSDT.csv`.

The engine must remain timeframe-dynamic. Do not hardcode 1m, 5m, 15m, BTCUSDT, or specific years in code.

## Current Problem

The existing downloader writes the final converted CSV to the same output path:

```text
data/historical/<SYMBOL>.csv
```

If the user runs the downloader in a loop for 2020, 2021, 2022, 2023, 2024, and 2025, each run may overwrite the same final CSV. The last year remains, not the full multi-year dataset.

## Required Data Layout

Use yearly files:

```text
data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv
data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv
data/historical/BTCUSDT/1m/BTCUSDT-1m-2022.csv
data/historical/BTCUSDT/1m/BTCUSDT-1m-2023.csv
data/historical/BTCUSDT/1m/BTCUSDT-1m-2024.csv
data/historical/BTCUSDT/1m/BTCUSDT-1m-2025.csv
```

Keep the existing single-file path as backward compatible:

```text
data/historical/BTCUSDT.csv
```

## Config Design

Add config support for per-symbol historical files.

Preferred TOML:

```toml
[historical_files]
BTCUSDT = [
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2022.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2023.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2024.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2025.csv",
]
```

Rules:

- If `historical_files.<SYMBOL>` exists and is non-empty, use those files in the declared order.
- If not configured, fall back to the current behavior: `data_dir/<SYMBOL>.csv`.
- Keep old configs working.
- Do not hardcode symbol names or years.

## Loader Behavior

Add a loader path that can load multiple CSV files:

```rust
OhlcvLoader::load_files(paths: &[PathBuf]) -> Result<LoadedOhlcv, OhlcvError>
```

Behavior:

1. Load each CSV with the existing CSV parser/validation rules.
2. Merge candles in file order.
3. Ensure final candle stream is strictly chronological.
4. Reject duplicate timestamps after merging.
5. Reject out-of-order timestamps after merging.
6. Preserve existing data-quality behavior.
7. Keep header handling compatible with the existing downloader output.

Do not silently sort away bad data unless the existing quality model explicitly supports it. Prefer deterministic validation errors for duplicates or out-of-order rows.

## Engine Integration

File:

```text
src/backtest/engine.rs
```

Replace direct single-file load selection with symbol-aware resolution:

```rust
let data_paths = cfg.historical_paths_for(symbol);
```

Then:

- If multiple paths are configured, call `OhlcvLoader::load_files(&data_paths)`.
- If no multi-file config exists, keep old single-file behavior.
- `BacktestEngine::run(cfg, symbol)` remains the entrypoint.

## Downloader Usage Documentation

Update docs to explain that the downloader overwrites the final `data/historical/<SYMBOL>.csv` path, so yearly export must copy that file after each yearly run.

Example local workflow:

```bash
mkdir -p data/historical/BTCUSDT/1m

for y in 2020 2021 2022 2023 2024 2025; do
  ./scripts/download_binance_klines.sh BTCUSDT "$y" 01 12 um 1m
  cp data/historical/BTCUSDT.csv "data/historical/BTCUSDT/1m/BTCUSDT-1m-$y.csv"
done

git checkout -- data/historical/BTCUSDT.csv
```

The last command restores the legacy single-file CSV so old configs are not accidentally changed by the last yearly download.

## Validation

Run:

```bash
cargo fmt
cargo test
cargo run --release -- research --config config/research_vwap_reclaim_mid_edge_cd0.toml
```

Add a new config example, for example:

```text
config/research_btcusdt_2020_2025.toml
```

It should use `historical_files.BTCUSDT` and otherwise keep strategy settings explicit.

## Expected Commit

```bash
git add src config docs
git commit -m "data: support multi-file historical inputs"
```

## Expected Summary

Report:

1. Config fields added.
2. Loader function added.
3. Backward compatibility behavior.
4. Validation for duplicate/out-of-order candles across files.
5. Example config added.
6. Confirmation that timeframe roles are still read from config only.
