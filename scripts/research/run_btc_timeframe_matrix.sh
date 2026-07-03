#!/usr/bin/env bash
set -euo pipefail

configs=(
  "config/research_mean_revert_v1_btc_1m_2020_2025.toml"
  "config/research_mean_revert_v1_btc_1m_confirmed_2020_2025.toml"
  "config/research_mean_revert_v1_btc_5m_2020_2025.toml"
  "config/research_mean_revert_v1_btc_15m_2020_2025.toml"
  "config/research_svs2_momentum_btc_5m_2020_2025.toml"
)

cargo fmt
cargo test

for cfg in "${configs[@]}"; do
  echo
  echo "================================================================="
  echo "Running $cfg"
  echo "================================================================="
  reports_dir=$(awk -F '"' '/reports_dir/ {print $2}' "$cfg" | tail -n 1)
  if [[ -z "$reports_dir" ]]; then
    echo "Cannot resolve reports_dir for $cfg" >&2
    exit 1
  fi
  rm -rf "$reports_dir"
  cargo run --release -- research --config "$cfg"
  test -f "$reports_dir/backtest_summary.json"
  cat "$reports_dir/backtest_summary.json"
done
