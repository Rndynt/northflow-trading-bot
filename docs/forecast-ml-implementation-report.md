# Forecast ML Implementation Report

## Roadmap reference

This report documents the completion patch for `docs/forecast-ml-roadmap.md` and `prompts/forecast-ml-module-implementation.md`.

## Completed phases

- F0 forecast CLI/config/module skeleton exists.
- F1 dataset builder writes dataset, feature, and label reports.
- F2 walk-forward splitter uses chronological fixed 30-day month windows with embargo.
- F3 Ridge is implemented as multivariate normal-equation Ridge with non-regularized intercept and train-only standardization.
- F4 Random Forest is deterministic regression, uses `feature_subsample_ratio`, and writes a feature-importance CSV placeholder when split counts are not collected.
- F5 model comparison and run manifest reports are written.

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
- Random Forest feature importance currently writes a clear zero-split placeholder instead of accumulated split counts.
- Classification target is explicitly rejected.
- Comparison report is conservative and does not claim profitability.

## Command results

See final agent response for current command output.

## Boundary confirmation

Existing strategy, backtest, risk sizing, fill simulation, and accounting behavior were not intentionally changed by this patch.
