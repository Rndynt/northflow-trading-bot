#!/usr/bin/env python3
"""Apply one global [cost] profile to one or more research config files.

Usage:
  python3 scripts/research/apply_cost_profile.py \
    --profile config/cost/binance_futures_taker.toml \
    config/research_*.toml

This deliberately keeps exchange execution cost separate from strategy logic.
Strategy config files define strategy parameters. Cost profile files define the
exchange fee/slippage assumption used for a research run.
"""

from __future__ import annotations

import argparse
from pathlib import Path

COST_KEYS = {
    "taker_fee_bps",
    "slippage_bps",
    "spread_bps",
    "market_impact_bps",
    "stop_slippage_bps",
}


def read_cost_block(path: Path) -> list[str]:
    lines = path.read_text().splitlines()
    block: list[str] = []
    in_cost = False

    for line in lines:
        stripped = line.strip()
        if stripped == "[cost]":
            in_cost = True
            block = ["[cost]"]
            continue
        if in_cost and stripped.startswith("[") and stripped.endswith("]"):
            break
        if in_cost:
            if not stripped or stripped.startswith("#"):
                block.append(line)
                continue
            key = stripped.split("=", 1)[0].strip()
            if key in COST_KEYS:
                block.append(line)

    if not block or block == ["[cost]"]:
        raise SystemExit(f"No usable [cost] block found in {path}")

    return block


def apply_cost_block(config_path: Path, cost_block: list[str]) -> None:
    lines = config_path.read_text().splitlines()
    out: list[str] = []
    in_cost = False
    replaced = False

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        if stripped == "[cost]":
            if not replaced:
                out.extend(cost_block)
                replaced = True
            in_cost = True
            i += 1
            continue

        if in_cost:
            if stripped.startswith("[") and stripped.endswith("]"):
                in_cost = False
                out.append(line)
            i += 1
            continue

        out.append(line)
        i += 1

    if not replaced:
        out.append("")
        out.extend(cost_block)

    config_path.write_text("\n".join(out).rstrip() + "\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("configs", nargs="+", type=Path)
    args = parser.parse_args()

    cost_block = read_cost_block(args.profile)
    for cfg in args.configs:
        apply_cost_block(cfg, cost_block)
        print(f"applied {args.profile} -> {cfg}")


if __name__ == "__main__":
    main()
