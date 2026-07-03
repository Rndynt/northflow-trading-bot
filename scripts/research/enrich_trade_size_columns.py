#!/usr/bin/env python3
"""Add USD position-size/notional audit columns to report trades.csv files.

The engine stores `qty` in base asset units, for example BTC quantity on BTCUSDT.
For fee and sizing audit, reports also need USD notional columns:

  position_size_usd      = entry_price * qty
  entry_notional_usd     = entry_price * qty
  exit_notional_usd      = exit_price * qty
  round_trip_notional_usd = entry_notional_usd + exit_notional_usd
  fee_bps_round_trip     = fee / round_trip_notional_usd * 10_000

Usage:
  python3 scripts/research/enrich_trade_size_columns.py reports/<run_dir>
  python3 scripts/research/enrich_trade_size_columns.py reports/run_a reports/run_b
"""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

DERIVED_FIELDS = [
    "position_size_usd",
    "entry_notional_usd",
    "exit_notional_usd",
    "avg_notional_usd",
    "round_trip_notional_usd",
    "fee_bps_round_trip",
]


def f(row: dict[str, str], key: str) -> float:
    value = row.get(key, "")
    try:
        return float(value)
    except ValueError:
        return 0.0


def enrich_file(path: Path) -> None:
    with path.open(newline="") as fh:
        reader = csv.DictReader(fh)
        rows = list(reader)
        source_fields = reader.fieldnames or []

    if not source_fields:
        raise SystemExit(f"empty CSV header: {path}")

    # Keep original order, remove old derived fields if script is re-run, then
    # insert audit fields immediately after `qty`.
    fields = [field for field in source_fields if field not in DERIVED_FIELDS]
    insert_at = fields.index("qty") + 1 if "qty" in fields else len(fields)
    fields = fields[:insert_at] + DERIVED_FIELDS + fields[insert_at:]

    for row in rows:
        entry_price = f(row, "entry_price")
        exit_price = f(row, "exit_price")
        qty = f(row, "qty")
        fee = f(row, "fee")

        entry_notional = entry_price * qty
        exit_notional = exit_price * qty
        avg_notional = (entry_notional + exit_notional) / 2.0
        round_trip_notional = entry_notional + exit_notional
        fee_bps_round_trip = (
            fee / round_trip_notional * 10_000.0 if round_trip_notional > 0.0 else 0.0
        )

        row["position_size_usd"] = f"{entry_notional:.6f}"
        row["entry_notional_usd"] = f"{entry_notional:.6f}"
        row["exit_notional_usd"] = f"{exit_notional:.6f}"
        row["avg_notional_usd"] = f"{avg_notional:.6f}"
        row["round_trip_notional_usd"] = f"{round_trip_notional:.6f}"
        row["fee_bps_round_trip"] = f"{fee_bps_round_trip:.6f}"

    with path.open("w", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)

    print(f"enriched {path}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report_dirs", nargs="+", type=Path)
    args = parser.parse_args()

    for report_dir in args.report_dirs:
        trades_csv = report_dir / "trades.csv"
        if not trades_csv.exists():
            raise SystemExit(f"missing trades.csv: {trades_csv}")
        enrich_file(trades_csv)


if __name__ == "__main__":
    main()
