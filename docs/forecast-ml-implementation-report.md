# Forecast ML Implementation Report

## Roadmap reference

This report documents the completion patch for `docs/forecast-ml-roadmap.md` and `prompts/forecast-ml-module-implementation.md`.

## Completed phases

- F0 forecast CLI/config/module skeleton exists.
- F1 dataset builder writes dataset, feature, and label reports.
- F2 walk-forward splitter uses chronological fixed 30-day month windows with embargo.
- F3 Ridge is implemented as multivariate normal-equation Ridge with non-regularized intercept and train-only standardization.
- F4 Random Forest is deterministic regression, uses `feature_subsample_ratio`, and writes real split-count feature importance accumulated across every accepted split in every tree (zero-importance fallback only for the genuine no-split case).
- F5 model comparison is computed from real in-memory per-model evaluation results (metrics + prediction buckets), and the run manifest lists the reports actually written during the run.

## Module boundaries

Forecast remains independent under `src/forecast/*`. It does not emit production signals, place orders, call exchanges, call LLMs, mutate account state, or integrate with strategy/backtest/risk sizing/fill/accounting behavior.

## Config schema

`config/forecast.toml` includes mode, pairs/timeframes, data/historical files, features, label target/cost flag, cost assumptions, enabled models, Ridge/RF hyperparameters, walk-forward settings, and report directory.

## Features and labels

Features: return_1m, return_5m, return_15m, atr_bps, volume_ratio, vwap_distance_bps, ema_8_21_spread_bps, range_position, hour_of_day, day_of_week.

Regression labels: `future_return_bps`, `future_return_after_cost_bps`. `future_direction_after_cost` is rejected until a real classification evaluator exists.

## Target handling

Reports include configured and effective target in the manifest/comparison. `cost_adjusted = true` maps the effective target to `future_return_after_cost_bps`; otherwise the configured regression target is used.

## Walk-forward policy

Walk-forward windows are chronological. Month lengths are fixed at 30 days for the MVP. Embargo bars separate train and test regions.

## Ridge details

Ridge solves `(X^T X + alpha I)w = X^T y` with Gaussian elimination. The intercept is not regularized. Feature means/stds are computed on training rows only.

## Random Forest details

Random Forest is an in-repo deterministic regression implementation. `feature_subsample_ratio` is validated (`0.0 < ratio <= 1.0`) and controls candidate feature subset count.

## Metrics and buckets

MAE, RMSE, correlation, directional accuracy, averages, and prediction buckets compare predictions against the same effective target used for training. Reports still carry raw and after-cost actual columns.

## Reports

Successful runs write dataset_summary.json, feature_summary.csv, label_summary.json, walk_forward_windows.csv, model summaries/buckets/walk-forward CSVs, random_forest_feature_importance.csv, model_comparison.json, and forecast_run_manifest.json.

## Dependency decisions

No new dependency was added. The implementations are deterministic Rust code and require no Python or network service.

## Limitations

- Fixed 30-day months in walk-forward splitting.
- Classification target is explicitly rejected.
- Comparison report is conservative and does not claim profitability; a `candidate_for_backtest_filter_phase` recommendation only means the model may be tested later as a scorer/filter, not that it is production-ready.
- Random Forest training over the full ~3.15M-row / 20-window BTCUSDT walk-forward set is slow (tens of minutes) on a single CPU core; see command results below.

## Command results

- `cargo fmt --check`: **passed**. No diff produced; the patched files (`evaluation.rs`, `models/random_forest.rs`, `reports.rs`, `runner.rs`) are formatting-clean.
- `cargo test`: **passed**. Exact summary: `test result: ok. 435 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. This is up from the pre-patch baseline of 416 passing tests (19 new/updated tests added by this patch, covering model comparison, effective-target buckets, split-count feature importance, report-writer smoke behavior, manifest fields, and the missing-data formatter).
- `cargo run -- forecast --config config/forecast.toml`: **completed**, using the full 6-year BTCUSDT 1m historical set (2020-2025, ~3.15M input rows, 20 walk-forward windows, `trees = 100`, `max_depth = 8`). Built with `cargo build --release` and run to completion in the background so it could be polled without an artificial timeout. Ridge finished in well under a minute; Random Forest (100 trees × 20 windows, ~518k training rows per window) was the dominant cost and took roughly 41 minutes wall-clock in total on this sandbox's single CPU core. All expected reports were written: `dataset_summary.json`, `feature_summary.csv`, `label_summary.json`, `walk_forward_windows.csv`, `ridge_summary.json`, `ridge_prediction_buckets.csv`, `ridge_walk_forward.csv`, `random_forest_summary.json`, `random_forest_prediction_buckets.csv`, `random_forest_walk_forward.csv`, `random_forest_feature_importance.csv`, `model_comparison.json`, `forecast_run_manifest.json`. Observed results (not a claim of profitability): `random_forest_feature_importance.csv` shows real, non-zero split counts for all 10 enabled features (importance values sum to ~1.0); `model_comparison.json` shows real (non-null) `best_model_by_rmse`, `best_model_by_correlation`, and `best_model_by_top_decile_return` (all `"ridge"` for this run) with `recommendation = "reject_due_to_cost_adjusted_decay"`, since the top-decile average effective (cost-adjusted) return was negative for both models over this window set.
- `cargo run -- research --config config/research.toml`: **completed**, using the same full 6-year BTCUSDT 1m historical set. This is the pre-existing strategy/backtest pipeline (unrelated to the forecast module) and finished in well under a minute, confirming this patch made no observable change to research/backtest behavior. All expected research reports were written (`backtest_summary.json`, `trades.csv`, `equity_curve.csv`, `signal_diagnostics.csv`, attribution CSVs, `report_manifest.json`, etc.).

## Boundary confirmation

Existing strategy, backtest, risk sizing, fill simulation, and accounting behavior were not intentionally changed by this patch.

## Result analysis

The generated reports from the full BTCUSDT run above are analyzed in detail, with an explicit research decision, in [`docs/forecast-ml-result-analysis.md`](./forecast-ml-result-analysis.md).
