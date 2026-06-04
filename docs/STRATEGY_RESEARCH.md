# Strategy Research Guide

Northflow supports deterministic, configurable strategy variants for controlled research comparison.

> **Disclaimer:** All backtest results are historical simulation only.
> They are not financial advice and do not guarantee future profitability.
> This is a research/diagnostic tool only. Paper and live trading are disabled.

---

## Strategy Run Modes

Set `strategy_run_mode` in the `[backtest]` section of your TOML config.

| Mode | Description |
|---|---|
| `single` | Run one strategy, write reports to `reports_dir` (default, fully backwards-compatible) |
| `comparison` | Run multiple strategies independently, each in its own subfolder; write aggregate summary |
| `multi` | Reserved — returns a config error; use `comparison` instead |

### Single mode (default)

```toml
[backtest]
reports_dir = "reports/v2_run"
strategy_run_mode = "single"
strategies = ["screened_vwap_scalp_v2"]
# or leave strategies empty — falls back to [strategy].strategy_id
```

Single mode is identical to the legacy behaviour when `strategy_run_mode` and `strategies` are omitted.

### Comparison mode

```toml
[backtest]
reports_dir = "reports/comparison"
strategy_run_mode = "comparison"
strategies = ["screened_vwap_scalp", "screened_vwap_scalp_v2"]
```

Each strategy gets its own isolated subfolder and independent equity/risk state:

```
reports/comparison/screened_vwap_scalp/
  backtest_summary.json
  trades.csv
  equity_curve.csv
  ...all Phase 7 files...

reports/comparison/screened_vwap_scalp_v2/
  backtest_summary.json
  trades.csv
  ...

reports/comparison/comparison_summary.csv    ← one row per strategy
reports/comparison/comparison_summary.json   ← same data in JSON
```

For **multi-symbol** comparison the layout nests symbol under base dir:

```
reports/comparison/BTCUSDT/screened_vwap_scalp/
reports/comparison/BTCUSDT/screened_vwap_scalp_v2/
reports/comparison/ETHUSDT/screened_vwap_scalp/
...
reports/comparison/comparison_summary.csv
reports/comparison/comparison_summary.json
```

---

## Comparison Summary Files

### comparison_summary.csv

One row per symbol × strategy run, plus a header row.

| Column | Description |
|---|---|
| `symbol` | Trading pair (e.g. `BTCUSDT`) |
| `strategy_id` | Strategy that produced this row |
| `reports_dir` | Relative path to the per-strategy report folder |
| `status` | `ok` or `error` |
| `error` | Error message if status = `error`, empty otherwise |
| `total_trades` | Number of closed trades |
| `win_rate` | Percentage of winning trades (0–100) |
| `net_pnl` | Net profit/loss after all costs |
| `gross_pnl` | Gross profit/loss before costs |
| `total_fee` | Total taker fees paid |
| `total_slippage` | Total slippage paid |
| `total_cost` | Total cost: fee + slippage + spread + market impact |
| `profit_factor` | Gross profit / gross loss (0 when undefined) |
| `expectancy` | Expected PnL per trade |
| `max_drawdown` | Maximum equity drawdown (%) |
| `max_consecutive_losses` | Worst consecutive losing streak |
| `avg_expected_edge_bps` | Average expected edge in basis points |
| `avg_actual_edge_bps` | Average realised edge in basis points |
| `avg_edge_realization_bps` | actual − expected edge (bps) |
| `avg_total_cost_bps` | Average all-in cost per trade (bps) |
| `signals_generated` | Total signals produced by the strategy |
| `signals_preapproved` | Signals that passed initial risk check |
| `signals_rejected_initial_risk` | Signals blocked at signal-close price |
| `signals_rejected_actual_entry` | Signals blocked at actual next-bar open |
| `trades_opened` | Positions opened |
| `trades_closed` | Positions closed |
| `risk_rejections` | Total RiskRejection rows written |
| `dominant_rejection_reason` | Most common rejection reason string |
| `dominant_rejection_count` | Count for dominant rejection reason |

### comparison_summary.json

Same data as the CSV in JSON array form, wrapped in:

```json
{
  "mode": "comparison",
  "runs": [ ... ]
}
```

---

## Isolation Guarantee

Each strategy in comparison mode runs with **independent state**:

- Separate `initial_equity` counter
- Separate drawdown and daily-loss accumulators
- Separate risk rejection log
- Separate signal ID sequence (restarted from `SIG-BT-00000001`)

Runs are sequential, not concurrent. Results are fully deterministic.

---

## Validation Rules

`validate_strategy_runner_config()` is called before any backtest runs and enforces:

1. `strategy_run_mode` must be `"single"`, `"comparison"`, or `"multi"` (reserved).
2. `"multi"` returns a clear error pointing to `"comparison"`.
3. No duplicate strategy IDs in `strategies`.
4. Each entry in `strategies` must be a known strategy ID.
5. `"single"` mode: `strategies` can have 0 or 1 items; 2+ is rejected with a clear suggestion to use `"comparison"`.
6. `"comparison"` mode: `strategies` must have at least 1 item.

---

## Strategy Variants

| `strategy_id` | Description |
|---|---|
| `screened_vwap_scalp` | Original deterministic multi-timeframe scalp strategy. |
| `screened_vwap_scalp_v2` | Stricter, cost-aware research variant with configurable filters. |

V2 adds: strict MTF confirmation, EMA ribbon alignment, ATR bps range, VWAP/EMA21 distance, minimum expected reward bps, minimum expected net edge bps, TP/SL ATR multipliers, volume ratio, and cooldown bars.

V2 is a diagnostic/research variant only. It is not a profitability claim and is not an optimizer.

---

## Switching Strategy (single mode)

Edit `config/research.toml`:

```toml
[strategy]
strategy_id = "screened_vwap_scalp"   # V1
# strategy_id = "screened_vwap_scalp_v2"  # V2
```

Then run:

```bash
cargo run -- research --config config/research.toml
```

---

## Comparing V1 vs V2 with the comparison runner

Set `strategy_run_mode = "comparison"` and list both strategies:

```toml
[strategy]
strategy_id = "screened_vwap_scalp_v2"   # used as fallback if strategies is empty

[backtest]
data_dir = "data/historical"
reports_dir = "reports/comparison"
strategy_run_mode = "comparison"
strategies = ["screened_vwap_scalp", "screened_vwap_scalp_v2"]
```

Run:

```bash
cargo run -- research --config config/research.toml
```

Reports written to:

```
reports/comparison/screened_vwap_scalp/
reports/comparison/screened_vwap_scalp_v2/
reports/comparison/comparison_summary.csv
reports/comparison/comparison_summary.json
```

---

## Comparing V1 vs V2 with separate configs (legacy approach)

Run V1:

```toml
[strategy]
strategy_id = "screened_vwap_scalp"

[backtest]
data_dir = "data/historical"
reports_dir = "reports/v1"
```

```bash
cargo run -- research --config config/v1.toml
```

Run V2:

```toml
[strategy]
strategy_id = "screened_vwap_scalp_v2"

[backtest]
data_dir = "data/historical"
reports_dir = "reports/v2"
```

```bash
cargo run -- research --config config/v2.toml
```

Compare `reports/v1/backtest_summary.json` vs `reports/v2/backtest_summary.json`.

---

## V2 Config Keys

All `v2_*` keys live under `[strategy]` in the TOML:

| Key | Default | Description |
|---|---|---|
| `v2_require_strict_confirmation` | `true` | Require 5m close to agree with 15m bias |
| `v2_require_ema_ribbon_alignment` | `true` | Require EMA 8 > 21 > 50 (long) or reversed (short) |
| `v2_allow_neutral_confirmation` | `false` | Accept neutral 5m candle as confirmation |
| `v2_min_expected_reward_bps` | `20.0` | Minimum expected TP reward in bps |
| `v2_min_expected_net_edge_bps` | `5.0` | Minimum expected edge net of cost in bps |
| `v2_min_atr_bps` | `5.0` | Minimum ATR in bps (low-volatility filter) |
| `v2_max_atr_bps` | `150.0` | Maximum ATR in bps (high-volatility filter) |
| `v2_tp_atr_multiple` | `2.0` | Take-profit = entry ± TP multiple × ATR |
| `v2_sl_atr_multiple` | `1.0` | Stop-loss = entry ∓ SL multiple × ATR |
| `v2_min_volume_ratio` | `1.0` | Signal-bar volume / 20-bar volume SMA ratio |
| `v2_vwap_distance_atr_min` | `0.0` | Minimum distance from VWAP in ATR multiples |
| `v2_vwap_distance_atr_max` | `2.0` | Maximum distance from VWAP in ATR multiples |
| `v2_cooldown_bars` | `0` | Bars to skip after a signal (prevents clustering) |
| `v2_enable_long` | `true` | Allow long signals |
| `v2_enable_short` | `true` | Allow short signals |
