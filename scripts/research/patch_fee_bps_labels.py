#!/usr/bin/env python3
"""Rename misleading fee bps report column and add one-way equivalent fee bps.

Old column:
  fee_bps_round_trip

Problem:
  It was calculated as fee / (entry_notional + exit_notional) * 10_000,
  so the value is the fee rate on combined notional, not the round-trip fee
  measured against one-way position size.

New columns:
  fee_bps_on_combined_notional
  fee_bps_on_entry_notional

Run from repo root:
  python3 scripts/research/patch_fee_bps_labels.py
  cargo fmt
  cargo test
"""

from __future__ import annotations

from pathlib import Path

PATH = Path("src/backtest/report.rs")


def replace_all(src: str, old: str, new: str) -> str:
    if old not in src:
        raise SystemExit(f"pattern not found: {old}")
    return src.replace(old, new)


def main() -> None:
    src = PATH.read_text()

    src = replace_all(src, "fee_bps_round_trip", "fee_bps_on_combined_notional")

    src = replace_all(
        src,
        "round_trip_notional_usd,fee_bps_on_combined_notional,equity_at_entry_usd,\\\n",
        "round_trip_notional_usd,fee_bps_on_combined_notional,fee_bps_on_entry_notional,equity_at_entry_usd,\\\n",
    )

    src = replace_all(
        src,
        "{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{},{:.6},{:.6}",
        "{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{},{:.6},{:.6}",
    )

    src = replace_all(
        src,
        "notional.fee_bps_on_combined_notional,\n                notional.equity_at_entry_usd,",
        "notional.fee_bps_on_combined_notional,\n                notional.fee_bps_on_entry_notional,\n                notional.equity_at_entry_usd,",
    )

    src = replace_all(
        src,
        "fee_bps_on_combined_notional: f64,\n    equity_at_entry_usd: f64,",
        "fee_bps_on_combined_notional: f64,\n    fee_bps_on_entry_notional: f64,\n    equity_at_entry_usd: f64,",
    )

    src = replace_all(
        src,
        "let fee_bps_on_combined_notional = if round_trip_notional_usd > 0.0 {\n        t.fee / round_trip_notional_usd * 10_000.0\n    } else {\n        0.0\n    };",
        "let fee_bps_on_combined_notional = if round_trip_notional_usd > 0.0 {\n        t.fee / round_trip_notional_usd * 10_000.0\n    } else {\n        0.0\n    };\n    let fee_bps_on_entry_notional = if entry_notional_usd > 0.0 {\n        t.fee / entry_notional_usd * 10_000.0\n    } else {\n        0.0\n    };",
    )

    src = replace_all(
        src,
        "fee_bps_on_combined_notional,\n        equity_at_entry_usd,",
        "fee_bps_on_combined_notional,\n        fee_bps_on_entry_notional,\n        equity_at_entry_usd,",
    )

    if '"fee_bps_on_entry_notional",' not in src:
        src = replace_all(
            src,
            '"fee_bps_on_combined_notional",\n',
            '"fee_bps_on_combined_notional",\n            "fee_bps_on_entry_notional",\n',
        )

    src = src.replace(
        "assert!((audit.fee_bps_on_combined_notional - 5.0).abs() < 1e-9);",
        "assert!((audit.fee_bps_on_combined_notional - 5.0).abs() < 1e-9);\n        assert!((audit.fee_bps_on_entry_notional - 10.1).abs() < 1e-9);",
        1,
    )

    PATH.write_text(src)
    print("patched fee bps labels in src/backtest/report.rs")


if __name__ == "__main__":
    main()
