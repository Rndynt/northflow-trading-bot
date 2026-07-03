#!/usr/bin/env python3
"""Patch ReportWriter to emit native position-sizing audit columns.

This is a deterministic source patch for src/backtest/report.rs.
It adds native trades.csv columns for:
  equity_at_entry_usd
  risk_amount_usd
  risk_per_unit_usd
  stop_distance_bps
  risk_pct_of_equity
  leverage_used

Run from repo root:
  python3 scripts/research/patch_native_position_sizing_audit.py
  cargo fmt
  cargo test
"""

from __future__ import annotations

from pathlib import Path

PATH = Path("src/backtest/report.rs")


def replace_once(src: str, old: str, new: str) -> str:
    if old not in src:
        raise SystemExit(f"pattern not found:\n{old}")
    return src.replace(old, new, 1)


def find_fn_bounds(src: str, name: str) -> tuple[int, int]:
    start = src.index(name)
    brace = src.index("{", start)
    depth = 0
    for i in range(brace, len(src)):
        ch = src[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return start, i + 1
    raise SystemExit(f"cannot find end of {name}")


def main() -> None:
    src = PATH.read_text()

    src = replace_once(
        src,
        "Self::write_trades_csv(dir, trades)?;",
        "Self::write_trades_csv(dir, trades, equity_curve)?;",
    )

    old_fn_start, old_fn_end = find_fn_bounds(src, "fn write_trades_csv")
    new_fn = r'''fn write_trades_csv(
        dir: &Path,
        trades: &[Trade],
        equity_curve: &[EquityPoint],
    ) -> Result<(), NorthflowError> {
        let path = dir.join("trades.csv");
        let mut rows: Vec<String> = Vec::with_capacity(trades.len() + 1);
        rows.push(
            "trade_id,signal_id,symbol,strategy_id,regime,side,\
             entry_time,exit_time,entry_price,exit_price,stop_loss,take_profit,qty,\
             position_size_usd,entry_notional_usd,exit_notional_usd,avg_notional_usd,\
             round_trip_notional_usd,fee_bps_round_trip,equity_at_entry_usd,\
             risk_amount_usd,risk_per_unit_usd,stop_distance_bps,risk_pct_of_equity,leverage_used,\
             gross_pnl,fee,slippage,net_pnl,reward_risk,bars_held,exit_reason,\
             entry_reason,filters_passed,filters_failed,expected_edge_bps,actual_edge_bps"
                .to_string(),
        );

        let mut equity_at_entry = initial_equity_from_curve(equity_curve);

        for t in trades {
            let filters_passed = t.filters_passed.join("|");
            let filters_failed = t.filters_failed.join("|");
            let notional = trade_notional_audit(t, equity_at_entry);
            let row = format!(
                "{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.8},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{},{:.6},{:.6}",
                csv_escape(t.trade_id.as_str()),
                csv_escape(t.signal_id.as_str()),
                csv_escape(t.symbol.as_str()),
                csv_escape(t.strategy_id.as_str()),
                csv_escape(&t.regime),
                csv_escape(t.side.as_str()),
                t.entry_time,
                t.exit_time,
                t.entry_price,
                t.exit_price,
                t.stop_loss,
                t.take_profit,
                t.quantity,
                notional.position_size_usd,
                notional.entry_notional_usd,
                notional.exit_notional_usd,
                notional.avg_notional_usd,
                notional.round_trip_notional_usd,
                notional.fee_bps_round_trip,
                notional.equity_at_entry_usd,
                notional.risk_amount_usd,
                notional.risk_per_unit_usd,
                notional.stop_distance_bps,
                notional.risk_pct_of_equity,
                notional.leverage_used,
                t.gross_pnl,
                t.fee,
                t.slippage,
                t.net_pnl,
                t.reward_risk,
                t.bars_held,
                csv_escape(t.exit_reason.as_str()),
                csv_escape(&t.entry_reason),
                csv_escape(&filters_passed),
                csv_escape(&filters_failed),
                t.expected_edge_bps,
                t.actual_edge_bps,
            );
            rows.push(row);

            if equity_at_entry > 0.0 {
                equity_at_entry += t.net_pnl;
            }
        }

        let content = rows.join("\n") + "\n";
        fs::write(&path, content)
            .map_err(|e| NorthflowError::DataError(format!("cannot write {}: {e}", path.display())))
    }'''
    src = src[:old_fn_start] + new_fn + src[old_fn_end:]

    old_struct_start, old_struct_end = find_fn_bounds(src, "struct TradeNotionalAudit")
    # find_fn_bounds starts at the struct name and captures the struct body, but not derive lines.
    derive_start = src.rfind("#[derive", 0, old_struct_start)
    helper_end = src.index("// ── CSV helpers", old_struct_end)
    new_helpers = r'''#[derive(Debug, Clone, Copy)]
struct TradeNotionalAudit {
    position_size_usd: f64,
    entry_notional_usd: f64,
    exit_notional_usd: f64,
    avg_notional_usd: f64,
    round_trip_notional_usd: f64,
    fee_bps_round_trip: f64,
    equity_at_entry_usd: f64,
    risk_amount_usd: f64,
    risk_per_unit_usd: f64,
    stop_distance_bps: f64,
    risk_pct_of_equity: f64,
    leverage_used: f64,
}

fn trade_notional_audit(t: &Trade, equity_at_entry_usd: f64) -> TradeNotionalAudit {
    // `quantity` is base asset quantity, e.g. BTC quantity for BTCUSDT.
    // Position size / notional must be shown in quote currency (USDT/USD).
    let entry_notional_usd = t.entry_price * t.quantity;
    let exit_notional_usd = t.exit_price * t.quantity;
    let avg_notional_usd = (entry_notional_usd + exit_notional_usd) / 2.0;
    let round_trip_notional_usd = entry_notional_usd + exit_notional_usd;
    let fee_bps_round_trip = if round_trip_notional_usd > 0.0 {
        t.fee / round_trip_notional_usd * 10_000.0
    } else {
        0.0
    };

    let risk_per_unit_usd = (t.entry_price - t.stop_loss).abs();
    let risk_amount_usd = risk_per_unit_usd * t.quantity;
    let stop_distance_bps = if t.entry_price > 0.0 {
        risk_per_unit_usd / t.entry_price * 10_000.0
    } else {
        0.0
    };
    let risk_pct_of_equity = if equity_at_entry_usd > 0.0 {
        risk_amount_usd / equity_at_entry_usd * 100.0
    } else {
        0.0
    };
    let leverage_used = if equity_at_entry_usd > 0.0 {
        entry_notional_usd / equity_at_entry_usd
    } else {
        0.0
    };

    TradeNotionalAudit {
        position_size_usd: entry_notional_usd,
        entry_notional_usd,
        exit_notional_usd,
        avg_notional_usd,
        round_trip_notional_usd,
        fee_bps_round_trip,
        equity_at_entry_usd,
        risk_amount_usd,
        risk_per_unit_usd,
        stop_distance_bps,
        risk_pct_of_equity,
        leverage_used,
    }
}

fn initial_equity_from_curve(equity_curve: &[EquityPoint]) -> f64 {
    equity_curve.first().map(|p| p.equity).unwrap_or(0.0)
}

'''
    src = src[:derive_start] + new_helpers + src[helper_end:]

    src = replace_once(
        src,
        "let audit = trade_notional_audit(&t);",
        "let audit = trade_notional_audit(&t, 5000.0);",
    )

    header_fields = [
        '"fee_bps_round_trip",',
        '"equity_at_entry_usd",',
        '"risk_amount_usd",',
        '"risk_per_unit_usd",',
        '"stop_distance_bps",',
        '"risk_pct_of_equity",',
        '"leverage_used",',
    ]
    if '"equity_at_entry_usd",' not in src:
        anchor = '"fee_bps_round_trip",\n'
        insert = ''.join(f'            {field}\n' for field in header_fields[1:])
        src = replace_once(src, anchor, anchor + insert)

    # Strengthen the existing audit test when present.
    src = src.replace(
        "assert!((audit.fee_bps_round_trip - 5.0).abs() < 1e-9);",
        "assert!((audit.fee_bps_round_trip - 5.0).abs() < 1e-9);\n        assert!((audit.risk_per_unit_usd - 300.0).abs() < 1e-9);\n        assert!((audit.risk_amount_usd - 30.0).abs() < 1e-9);\n        assert!((audit.stop_distance_bps - 100.0).abs() < 1e-9);\n        assert!((audit.risk_pct_of_equity - 0.6).abs() < 1e-9);\n        assert!((audit.leverage_used - 0.6).abs() < 1e-9);",
        1,
    )

    PATH.write_text(src)
    print("patched src/backtest/report.rs with native position sizing audit columns")


if __name__ == "__main__":
    main()
