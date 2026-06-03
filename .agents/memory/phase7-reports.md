---
name: Phase 7 Reports and Attribution
description: Design decisions and gotchas for src/report/ — attribution, audit, manifest, validation layers.
---

## Structure

4 active files in `src/report/`:
- `attribution.rs` — `AttributionEngine::build(&[Trade]) -> AttributionReport`; uses BTreeMap for deterministic sorted buckets
- `audit.rs` — `ReportAuditor::audit_trades(&[Trade]) -> AuditReport`; passed=false only when error_count > 0
- `manifest.rs` — `ManifestWriter::build(...) -> ReportManifest`; BTreeMap for path-sorted file list
- `validation.rs` — `TradeValidator` (composable field helpers); delegates to ReportAuditor for full validation

`AttributionWriter::write_all()` is in `mod.rs` alongside the re-exports.

## Critical rules

### Audit severity split
- **Error** → broken attribution or invalid field → `passed = false`
- **Warning** → incomplete explainability (empty filters_passed, empty filters_failed, non-positive expected_edge, bars_held==0) → does NOT fail the audit
- Do NOT block report writing on warnings.

### Traceability checks
- `trade_id` must contain `signal_id` as a substring
- `position_id` must contain `signal_id` as a substring
- Both are **Errors** if violated

### Filter bucket keys
- `passed:<filter_name>` for filters in `trade.filters_passed`
- `failed:<filter_name>` for filters in `trade.filters_failed`
- Trades with no filters at all → no filter buckets created

### Manifest
- Uses BTreeMap for path-sorted deterministic output
- Paths use `reports_dir` prefix as-is (default "reports") → all paths are relative
- No system time, no random IDs

### Edition 2024 keyword
- `gen` is a reserved keyword in Rust Edition 2024 — use `generated_by` instead

## Integration (research/mod.rs)
After Phase 6 base reports are written:
1. `AttributionEngine::build(&result.trades)`
2. `ReportAuditor::audit_trades(&result.trades)`
3. `ManifestWriter::build(reports_dir, trades, equity_curve, &attribution)`
4. `AttributionWriter::write_all(reports_dir, &attribution, &audit, &manifest)`

If audit has errors: print warning + list errors, but still write all files and return Ok.
Only file I/O failure returns an error message.

**Why:** Non-fatal audit errors allow researchers to inspect exactly what went wrong per-trade without losing the attribution data.

## Test count: 360 (306 Phase 1-6 + 54 Phase 7)
