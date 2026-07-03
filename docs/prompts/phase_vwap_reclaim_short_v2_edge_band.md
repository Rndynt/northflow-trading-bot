# Phase: vwap_reclaim_short_v2 edge-band probe

## Goal

Add optional upper-bound filtering for expected net edge in vwap_reclaim_short_v2.

Current edge30 report shows:
- edge_30_50 is small positive
- edge_gte_50 is negative

Current config only supports minimum expected net edge, so it cannot isolate only 30–50 bps.

## Required changes

Modify:
- src/config/mod.rs
- src/strategy/vwap_reclaim_short_v2.rs

Add config key:
- vrs2_max_expected_net_edge_bps

Rules:
- Existing configs must keep working.
- If the new key is absent, behavior must remain unchanged.
- If present, validate it is finite, non-negative, and not below vrs2_min_expected_net_edge_bps.

Strategy logic:
- After expected_net_edge_bps is calculated, keep existing min check.
- Add max check:
  reject signal when expected_net_edge_bps > vrs2_max_expected_net_edge_bps.
- Add filters_passed tag:
  expected_net_edge_band_ok

Create config:
- config/research_vwap_reclaim_short_v2_probe_edge30_50_2020_2025.toml

Base it on:
- config/research_vwap_reclaim_short_v2_probe_edge30_2020_2025.toml

Change:
- reports_dir = "reports/vwap_reclaim_short_v2_probe_edge30_50_2020_2025"
- vrs2_min_expected_net_edge_bps = 30.0
- vrs2_max_expected_net_edge_bps = 50.0

Keep other edge30 settings unchanged.

Run:
cargo fmt
cargo test
cargo run --release -- research --config config/research_vwap_reclaim_short_v2_probe_edge30_50_2020_2025.toml

Commit:
git add src/config/mod.rs src/strategy/vwap_reclaim_short_v2.rs config/research_vwap_reclaim_short_v2_probe_edge30_50_2020_2025.toml reports/vwap_reclaim_short_v2_probe_edge30_50_2020_2025 docs/prompts/phase_vwap_reclaim_short_v2_edge_band.md
git commit -m "research: add vwap reclaim short v2 edge band probe"
