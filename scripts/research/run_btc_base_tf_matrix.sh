#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUT_CONFIG_DIR="config/generated/btc_base_tf_matrix"
OUT_REPORT_DIR="reports/btc_base_tf_matrix"
mkdir -p "$OUT_CONFIG_DIR" "$OUT_REPORT_DIR"

strategies=(
  screened_vwap_scalp
  screened_vwap_scalp_v2
  ema_trend_pullback_v1
  vwap_reclaim_short_v1
  vwap_reclaim_short_v2
  mean_revert_v1
)

timeframes=(1m 5m 15m)

write_config() {
  local strategy="$1"
  local tf="$2"
  local conf screen max_bars lookback

  case "$tf" in
    1m) conf="5m"; screen="15m"; max_bars=60; lookback=240 ;;
    5m) conf="15m"; screen="1h"; max_bars=36; lookback=160 ;;
    15m) conf="1h"; screen="4h"; max_bars=24; lookback=120 ;;
    *) echo "unknown tf: $tf" >&2; exit 1 ;;
  esac

  local name="${strategy}_${tf}"
  local cfg="$OUT_CONFIG_DIR/${name}.toml"
  local report="$OUT_REPORT_DIR/${name}"

  cat > "$cfg" <<TOML
[mode]
run_mode = "research"
dry_run = true

[pairs]
symbols = ["BTCUSDT"]
entry_timeframe = "$tf"
screening_timeframe = "$screen"
confirmation_timeframe = "$conf"

[historical_files]
BTCUSDT = [
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2022.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2023.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2024.csv",
  "data/historical/BTCUSDT/1m/BTCUSDT-1m-2025.csv",
]

[strategy]
strategy_id = "$strategy"
min_confidence = 65
v2_require_strict_confirmation = false
v2_require_ema_ribbon_alignment = false
v2_allow_neutral_confirmation = true
v2_min_expected_reward_bps = 20.0
v2_min_expected_net_edge_bps = 0.0
v2_min_atr_bps = 5.0
v2_max_atr_bps = 350.0
v2_tp_atr_multiple = 1.2
v2_sl_atr_multiple = 1.0
v2_min_volume_ratio = 0.80
v2_vwap_distance_atr_min = 0.0
v2_vwap_distance_atr_max = 4.0
v2_cooldown_bars = 0
v2_enable_long = true
v2_enable_short = true

[risk]
initial_equity_usd = 5000.0
risk_per_trade_pct = 0.15
max_open_positions = 1
min_reward_risk = 1.0
max_daily_loss_pct = 3.0
max_drawdown_pct = 100.0

[cost]
taker_fee_bps = 5.0
slippage_bps = 0.0
spread_bps = 0.0
market_impact_bps = 0.0
stop_slippage_bps = 0.0

[backtest]
data_dir = "data/historical"
reports_dir = "$report"
conservative_intrabar = true
max_bars_held = $max_bars
entry_geometry_mode = "reanchor_to_actual_entry"
entry_lookback_bars = $lookback
strategy_run_mode = "single"
strategies = ["$strategy"]
TOML

  echo "$cfg"
}

cargo fmt
cargo test

summary="$OUT_REPORT_DIR/matrix_summary.csv"
echo "strategy_id,timeframe,total_trades,win_rate,net_pnl,gross_pnl,total_fee,total_slippage,profit_factor,expectancy,max_drawdown,max_consecutive_losses,reports_dir" > "$summary"

for strategy in "${strategies[@]}"; do
  for tf in "${timeframes[@]}"; do
    cfg="$(write_config "$strategy" "$tf")"
    report="$(awk -F '"' '/reports_dir/ {print $2}' "$cfg" | tail -n 1)"
    echo
    echo "================================================================="
    echo "Running $strategy $tf"
    echo "================================================================="
    rm -rf "$report"
    cargo run --release -- research --config "$cfg"
    test -f "$report/backtest_summary.json"
    python3 - "$strategy" "$tf" "$report" "$summary" <<'PY'
import csv, json, sys
strategy, tf, report, summary = sys.argv[1:5]
with open(f"{report}/backtest_summary.json") as f:
    s = json.load(f)
row = {
    "strategy_id": strategy,
    "timeframe": tf,
    "total_trades": s.get("total_trades", 0),
    "win_rate": s.get("win_rate", 0),
    "net_pnl": s.get("net_pnl", 0),
    "gross_pnl": s.get("gross_pnl", 0),
    "total_fee": s.get("total_fee", 0),
    "total_slippage": s.get("total_slippage", 0),
    "profit_factor": s.get("profit_factor", 0),
    "expectancy": s.get("expectancy", 0),
    "max_drawdown": s.get("max_drawdown", 0),
    "max_consecutive_losses": s.get("max_consecutive_losses", 0),
    "reports_dir": report,
}
fields = list(row.keys())
with open(summary, "a", newline="") as f:
    csv.DictWriter(f, fieldnames=fields).writerow(row)
print(json.dumps(row, indent=2))
PY
  done
done

echo
echo "Matrix summary: $summary"
cat "$summary"
