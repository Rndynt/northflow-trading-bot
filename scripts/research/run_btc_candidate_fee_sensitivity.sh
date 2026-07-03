#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUT_CONFIG_DIR="config/generated/btc_candidate_fee_sensitivity"
OUT_REPORT_DIR="reports/btc_candidate_fee_sensitivity"
mkdir -p "$OUT_CONFIG_DIR" "$OUT_REPORT_DIR"

# Only near-survivors from btc_edge_filter_retune.
# Do not add random strategies here. This is a research gate:
# if a candidate only works with lower fees, it needs maker/limit execution modelling.
candidates=(
  "ema_trend_pullback_v1|15m|edge120_rr20_cd12|edge120"
  "ema_trend_pullback_v1|5m|edge120_rr20_cd12|edge120"
  "screened_vwap_scalp_v2|15m|edge80_rr18_cd8|edge80"
)

fees=(
  "taker5|5.0"
  "maker2|2.0"
)

write_config() {
  local strategy="$1"
  local tf="$2"
  local profile="$3"
  local fee_name="$4"
  local fee_bps="$5"
  local conf screen max_bars lookback min_reward min_net rr cooldown tp sl min_vol strict

  case "$tf" in
    1m) conf="5m"; screen="15m"; max_bars=60; lookback=240 ;;
    5m) conf="15m"; screen="1h"; max_bars=36; lookback=160 ;;
    15m) conf="1h"; screen="4h"; max_bars=24; lookback=120 ;;
    *) echo "unknown tf: $tf" >&2; exit 1 ;;
  esac

  case "$profile" in
    edge80) min_reward=80.0; min_net=50.0; rr=1.8; cooldown=8; tp=1.8; sl=1.0; min_vol=1.1; strict=true ;;
    edge120) min_reward=120.0; min_net=80.0; rr=2.0; cooldown=12; tp=2.2; sl=1.0; min_vol=1.2; strict=true ;;
    *) echo "unknown profile: $profile" >&2; exit 1 ;;
  esac

  local name="${strategy}_${tf}_${profile}_${fee_name}"
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
min_confidence = 75
v2_require_strict_confirmation = $strict
v2_require_ema_ribbon_alignment = $strict
v2_allow_neutral_confirmation = false
v2_min_expected_reward_bps = $min_reward
v2_min_expected_net_edge_bps = $min_net
v2_min_atr_bps = 8.0
v2_max_atr_bps = 250.0
v2_tp_atr_multiple = $tp
v2_sl_atr_multiple = $sl
v2_min_volume_ratio = $min_vol
v2_vwap_distance_atr_min = 0.8
v2_vwap_distance_atr_max = 3.5
v2_cooldown_bars = $cooldown
v2_enable_long = true
v2_enable_short = true

etp_require_strict_screening_trend = $strict
etp_require_strict_confirmation_trend = $strict
etp_require_entry_ema_alignment = $strict
etp_allow_long = true
etp_allow_short = true
etp_pullback_to = "ema21_or_vwap"
etp_max_pullback_distance_atr = 1.25
etp_min_pullback_distance_atr = 0.10
etp_reclaim_mode = "close_reclaim"
etp_min_body_ratio = 0.30
etp_min_wick_rejection_ratio = 0.20
etp_sl_atr_multiple = $sl
etp_tp_atr_multiple = $tp
etp_min_reward_risk = $rr
etp_min_atr_bps = 8.0
etp_max_atr_bps = 250.0
etp_min_expected_reward_bps = $min_reward
etp_min_expected_net_edge_bps = $min_net
etp_min_volume_ratio = $min_vol
etp_cooldown_bars = $cooldown

[risk]
initial_equity_usd = 5000.0
risk_per_trade_pct = 0.15
max_open_positions = 1
min_reward_risk = $rr
max_daily_loss_pct = 3.0
max_drawdown_pct = 100.0

[cost]
taker_fee_bps = $fee_bps
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

summary="$OUT_REPORT_DIR/fee_sensitivity_summary.csv"
echo "strategy_id,timeframe,profile,fee_profile,fee_bps,total_trades,win_rate,net_pnl,gross_pnl,total_fee,total_slippage,profit_factor,expectancy,max_drawdown,max_consecutive_losses,reports_dir" > "$summary"

for item in "${candidates[@]}"; do
  IFS='|' read -r strategy tf source_profile profile <<< "$item"
  for fee_item in "${fees[@]}"; do
    IFS='|' read -r fee_name fee_bps <<< "$fee_item"
    cfg="$(write_config "$strategy" "$tf" "$profile" "$fee_name" "$fee_bps")"
    report="$(awk -F '"' '/reports_dir/ {print $2}' "$cfg" | tail -n 1)"
    echo
    echo "================================================================="
    echo "Candidate $strategy $tf $source_profile $fee_name"
    echo "================================================================="
    rm -rf "$report"
    cargo run --release -- research --config "$cfg"
    test -f "$report/backtest_summary.json"
    python3 - "$strategy" "$tf" "$source_profile" "$fee_name" "$fee_bps" "$report" "$summary" <<'PY'
import csv, json, sys
strategy, tf, profile, fee_name, fee_bps, report, summary = sys.argv[1:8]
with open(f"{report}/backtest_summary.json") as f:
    s = json.load(f)
row = {
    "strategy_id": strategy,
    "timeframe": tf,
    "profile": profile,
    "fee_profile": fee_name,
    "fee_bps": fee_bps,
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
with open(summary, "a", newline="") as f:
    csv.DictWriter(f, fieldnames=list(row.keys())).writerow(row)
print(json.dumps(row, indent=2))
PY
  done
done

echo
echo "Fee sensitivity summary: $summary"
cat "$summary"
