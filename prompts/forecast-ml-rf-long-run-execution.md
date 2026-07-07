# Forecast ML Random Forest Long-Run Execution Prompt

## Phase

```text
F7B — Full Random Forest Long-Run Execution For Horizon Comparison
```

## References

Read these first:

```text
docs/forecast-ml-roadmap.md
prompts/forecast-ml-module-implementation.md
prompts/forecast-ml-completion-correctness-patch.md
prompts/forecast-ml-final-evaluation-quality-patch.md
prompts/forecast-ml-result-analysis.md
prompts/forecast-ml-horizon-comparison.md
docs/forecast-ml-implementation-report.md
docs/forecast-ml-result-analysis.md
docs/forecast-ml-horizon-comparison.md
config/forecast/btcusdt_1m_h15.toml
config/forecast/btcusdt_1m_h30.toml
config/forecast/btcusdt_1m_h1h.toml
config/forecast/btcusdt_1m_h4h.toml
```

F7 horizon comparison is currently incomplete because the sandbox environment could not complete full Random Forest runs for 30m, 1h, and 4h. Ridge completed for all horizons, and Random Forest full completed only for 15m. The official F7 decision is therefore:

```text
insufficient_completed_horizon_runs
```

This phase exists to finish the missing full-config Random Forest horizon runs in a compute environment that can sustain long-running jobs.

---

## Core Objective

Complete full forecast runs for:

```text
30m
1h
4h
```

using the full configs:

```text
config/forecast/btcusdt_1m_h30.toml
config/forecast/btcusdt_1m_h1h.toml
config/forecast/btcusdt_1m_h4h.toml
```

Then update:

```text
docs/forecast-ml-horizon-comparison.md
```

so the horizon comparison is based on four completed full-config runs, not smoke Random Forest runs.

---

## Non-Negotiable Boundaries

Do not modify production strategy behavior.
Do not add an ML-backed production strategy.
Do not integrate ForecastScorer.
Do not connect forecast output to `BacktestEngine`.
Do not make `strategy` depend on `forecast`.
Do not change risk sizing, fill simulation, accounting, indicators, or existing research backtest behavior.
Do not enable paper mode or live mode.
Do not add exchange/API/network/LLM calls.
Do not require Python.
Do not commit generated files under `reports/`.
Do not claim profitability.

This task is execution, validation, and documentation only.

Allowed changes:

```text
- update docs/forecast-ml-horizon-comparison.md with completed full RF results
- optionally add docs/forecast-ml-rf-long-run-execution.md as an execution log
- optionally add a shell script under scripts/ to run the long jobs safely
- optionally add small Rust/CLI improvements only if needed for stable long-run execution, logging, or resume behavior
- optionally add tests for any new helper logic
```

Generated report files must remain untracked.

---

## Environment Requirement

Run this in a real local machine or server environment, not a short-lived chat sandbox.

Minimum practical requirement:

```text
- enough disk for target/ and report output
- access to full BTCUSDT 1m historical files 2020-2025
- ability to keep process alive for at least 2-4 hours
- preferably inside tmux/screen/systemd-run/nohup
- release build, not debug build
```

Recommended:

```bash
cargo build --release
```

Use `tmux` or `screen` if available:

```bash
tmux new -s forecast-rf
```

Or use `nohup`:

```bash
nohup cargo run --release -- forecast --config config/forecast/btcusdt_1m_h30.toml > /tmp/forecast_h30.log 2>&1 &
```

Do not rely on Codex/chat sandbox background tasks if they are known to be killed after a few minutes.

---

## Required Full Commands

Run these full configs exactly:

```bash
cargo run --release -- forecast --config config/forecast/btcusdt_1m_h30.toml
cargo run --release -- forecast --config config/forecast/btcusdt_1m_h1h.toml
cargo run --release -- forecast --config config/forecast/btcusdt_1m_h4h.toml
```

Do not reduce:

```text
trees
max_depth
min_samples_leaf
feature_subsample_ratio
data years
walk-forward windows
```

Do not use smoke configs for final decision.

15m does not need to be re-run unless reports are missing/stale, because it already completed full config in F6/F7. If re-running 15m is easier for consistency, that is allowed.

---

## Required Reports Per Horizon

After each full run, verify the report directory contains:

```text
dataset_summary.json
feature_summary.csv
label_summary.json
walk_forward_windows.csv
ridge_summary.json
ridge_prediction_buckets.csv
ridge_walk_forward.csv
random_forest_summary.json
random_forest_prediction_buckets.csv
random_forest_walk_forward.csv
random_forest_feature_importance.csv
model_comparison.json
forecast_run_manifest.json
```

Expected directories:

```text
reports/forecast/btcusdt_1m_h30
reports/forecast/btcusdt_1m_h1h
reports/forecast/btcusdt_1m_h4h
```

Do not commit these report files.

---

## Long-Run Execution Logging

Create or update a documentation log:

```text
docs/forecast-ml-rf-long-run-execution.md
```

Required fields per horizon:

```text
horizon
config_path
start_time
end_time
wall_clock_duration
machine/environment description
command used
exit status
reports_dir
required reports present: yes/no
notes/errors
```

If a run fails, document exact failure:

```text
killed by OS
out of memory
timeout
missing historical data
panic
other error
```

Do not hide failures.

---

## Resume / Re-Run Policy

The current forecast runner may not have per-window checkpoint/resume. If the process dies before writing `model_comparison.json` and `forecast_run_manifest.json`, treat that horizon as incomplete and re-run the full horizon.

If adding resume/checkpoint support is necessary, keep it strictly inside `src/forecast/*` and reports. It must not affect strategy/backtest/risk/accounting.

A minimal acceptable checkpoint design, if implemented:

```text
reports/forecast/<horizon>/partial/
  ridge_predictions.csv
  random_forest_window_<window_id>_predictions.csv
  random_forest_window_<window_id>_split_counts.csv
```

Then a final aggregation step can combine completed windows into the normal reports.

But do not implement checkpointing unless needed. Prefer simply running full jobs in a stable environment first.

---

## Required Horizon Comparison Update

After all full runs complete, update:

```text
docs/forecast-ml-horizon-comparison.md
```

Remove or clearly archive the old incomplete-run caveat. The updated document must state that all four full horizon runs completed if true.

Update these sections:

```text
1. Executive Decision
2. Run Matrix
3. Dataset And Label Comparison
4. Ridge Horizon Comparison
5. Random Forest Horizon Comparison
6. Cross-Horizon Ranking Power
7. Decision
8. Next Phase Recommendation
```

The final decision must be one of:

```text
candidate_horizon_found
iterate_regime_attribution_next
iterate_label_design_next
iterate_cost_sensitivity_next
insufficient_completed_horizon_runs
```

Use `insufficient_completed_horizon_runs` only if one or more full runs still did not complete.

---

## Required Metrics To Extract

For each horizon and model, extract from reports:

```text
prediction_count
MAE
RMSE
correlation
directional_accuracy
avg_predicted_bps
avg_actual_after_cost_bps
top_decile_avg_effective_actual_bps
top_decile_hit_rate_effective_target
monotonicity_ratio
top_minus_bottom_spread_bps
```

For Random Forest, also extract top 5 feature importance rows:

```text
feature
split_count
importance
```

Compute:

```text
monotonic_steps = number of adjacent bucket pairs where avg_effective_actual_bps increases
max_steps = bucket_count - 1
monotonicity_ratio = monotonic_steps / max_steps

top_minus_bottom_spread_bps = top_bucket.avg_effective_actual_bps - bottom_bucket.avg_effective_actual_bps
```

---

## Decision Rules

Use conservative rules.

```text
if fewer than 4 full horizon runs completed:
  insufficient_completed_horizon_runs

else if one or more horizons have:
  top_decile_avg_effective_actual_bps > 0
  correlation > 0
  monotonicity_ratio >= 0.60
  top_minus_bottom_spread_bps > 0
then:
  candidate_horizon_found

else if longer horizons improve top_decile_avg_effective_actual_bps and spread but remain negative:
  iterate_cost_sensitivity_next or iterate_regime_attribution_next

else if all horizons remain noisy/negative with weak ranking:
  iterate_label_design_next
```

If `candidate_horizon_found`, document:

```text
candidate_horizon
candidate_model
why it passed
why this is not a profitability claim
what must still be validated before strategy integration
```

Do not recommend ForecastScorer unless a horizon actually survives cost with full-config results.

---

## Required Validation Commands

Run:

```bash
cargo fmt --check
cargo test
```

If code changed, tests must pass.

If only docs changed after report generation, still run at least:

```bash
cargo fmt --check
cargo test
```

Document exact results in:

```text
docs/forecast-ml-rf-long-run-execution.md
```

---

## Git Hygiene

Before commit:

```bash
git status --short
```

Must not include:

```text
reports/
target/
large CSV outputs
/tmp logs
local smoke configs
```

Allowed to commit:

```text
docs/forecast-ml-horizon-comparison.md
docs/forecast-ml-rf-long-run-execution.md
optional scripts/forecast-rf-long-run.sh
optional src/forecast helper only if necessary
optional tests only if helper code was added
```

---

## Optional Script

If useful, create:

```text
scripts/run-forecast-rf-long-horizons.sh
```

Requirements:

```bash
#!/usr/bin/env bash
set -euo pipefail
```

It should:

```text
- build release binary
- run 30m, 1h, 4h configs sequentially
- write logs to local ignored path, e.g. .local/forecast-logs/
- print start/end timestamps
- verify required report files exist after each run
- never add reports to git
```

If `.local/` is used, ensure it is gitignored or not committed.

---

## Acceptance Criteria

This phase is complete only when:

```text
- full RF 30m completed or failure is honestly documented
- full RF 1h completed or failure is honestly documented
- full RF 4h completed or failure is honestly documented
- docs/forecast-ml-rf-long-run-execution.md exists
- docs/forecast-ml-horizon-comparison.md is updated from real full results, or still explicitly says incomplete
- no smoke RF result is used as final decision basis
- generated reports are not committed
- no strategy/backtest/risk/fill/accounting behavior changes
- cargo fmt --check passes
- cargo test passes
```

---

## Out Of Scope

Do not implement:

```text
ForecastScorer
ML-backed strategy
backtest integration
new model family
hyperparameter optimization
new indicators
paper trading
live trading
exchange integration
Python analysis pipeline
UI/dashboard
```

This phase is long-run execution and documentation only.
