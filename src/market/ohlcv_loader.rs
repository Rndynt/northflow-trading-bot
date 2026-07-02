//! OhlcvLoader — deterministic 1m OHLCV CSV loader.
//!
//! Rules:
//!   - No async, no network, no exchange API calls.
//!   - Accepts headers: timestamp or open_time (case-insensitive, whitespace-tolerant).
//!   - Timestamps must be positive integers (Unix seconds or milliseconds).
//!     Decimal, NaN, inf, negative, and zero timestamps are rejected.
//!   - Normalises timestamps to milliseconds: < 10^12 → seconds × 1000.
//!   - Reports every rejected row in DataQualityReport; never panics on bad data.
//!   - Sorts output candles ascending by timestamp.
//!   - Detects non-monotonic input, duplicate timestamps (keep first),
//!     missing 1m gaps (delta is a positive exact multiple of 60_000 ms — warning), and
//!     irregular intervals (delta is not an exact multiple of 60_000 ms — error).
//!
//! Interval classification (after sort + dedup):
//!   delta == SOURCE_TIMEFRAME_MS               → Exact (valid)
//!   delta >  SOURCE_TIMEFRAME_MS
//!     && delta % SOURCE_TIMEFRAME_MS == 0      → MissingGap (warning)
//!   anything else (including 90_000, 150_000)  → Irregular (error)

use std::path::{Path, PathBuf};

use crate::core::{Candle, NorthflowError};
use crate::market::data_quality::{DataQualityIssueKind, DataQualityReport, MissingCandleGap};

/// Timestamps below this value are treated as Unix seconds and multiplied
/// by 1_000 to convert to milliseconds.  (~year 2001 in ms = 10^12)
const SECONDS_THRESHOLD: i64 = 1_000_000_000_000;

/// Expected interval between consecutive candles for the Phase 2 source timeframe (1m).
///
/// Future phases may make source_interval_ms configurable.
/// The generic rule is:
///   target_timeframe_ms must be divisible by source_timeframe_ms.
///   required_count = target_timeframe_ms / source_timeframe_ms.
/// For now, Phase 2 source data is fixed to 1m.
const SOURCE_TIMEFRAME_MS: i64 = 60_000;

// ── private interval classification ─────────────────────────────────────────

/// Result of classifying a delta between two consecutive sorted, deduped candles.
enum IntervalClassification {
    /// Delta equals exactly one source interval — correct 1m sequence.
    Exact,
    /// Delta is a positive exact multiple of the source interval — one or more
    /// candles are absent but the boundary timestamps are aligned.
    MissingGap { missing_count: u64 },
    /// Delta is anything else: sub-interval, non-multiple super-interval, or ≤ 0.
    /// Examples: 30_000, 90_000, 150_000.
    Irregular,
}

/// Classify the millisecond delta between two adjacent sorted candle timestamps.
///
/// `source_interval_ms` is the expected candle-to-candle interval for the source
/// data (60_000 for 1m).  The function is generic so future phases can pass a
/// different source interval without changing the calling code.
fn classify_interval_delta(delta: i64, source_interval_ms: i64) -> IntervalClassification {
    if delta == source_interval_ms {
        IntervalClassification::Exact
    } else if delta > source_interval_ms && delta % source_interval_ms == 0 {
        let missing_count = (delta / source_interval_ms) as u64 - 1;
        IntervalClassification::MissingGap { missing_count }
    } else {
        IntervalClassification::Irregular
    }
}

// ── public types ─────────────────────────────────────────────────────────────

pub struct OhlcvLoadResult {
    /// Sorted, deduplicated, validated 1m candles.
    pub candles: Vec<Candle>,
    /// Full data quality report including all issues and missing gaps.
    pub quality: DataQualityReport,
}

pub struct OhlcvLoader;

// ── private helpers ──────────────────────────────────────────────────────────

/// Parse a raw timestamp string into a millisecond Unix timestamp.
///
/// Rules:
///   - Must be a valid integer (no decimals, no NaN, no inf).
///   - Must be strictly positive (> 0).
///   - Values < SECONDS_THRESHOLD are treated as Unix seconds and multiplied by 1_000.
///   - Values >= SECONDS_THRESHOLD are kept as milliseconds.
fn parse_timestamp_ms(raw: &str) -> Result<i64, String> {
    let ts = raw
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("timestamp must be a positive integer, got '{raw}'"))?;

    if ts <= 0 {
        return Err(format!("timestamp must be > 0, got {ts}"));
    }

    if ts < SECONDS_THRESHOLD {
        Ok(ts * 1_000)
    } else {
        Ok(ts)
    }
}

// ── OhlcvLoader impl ─────────────────────────────────────────────────────────

impl OhlcvLoader {
    /// Load a 1m OHLCV CSV file from disk.
    ///
    /// Returns `Err` only on OS-level failure (file not found, permission denied).
    /// All CSV parsing and candle validation issues are captured in the
    /// returned `OhlcvLoadResult.quality` report.
    pub fn load_file(path: &Path) -> Result<OhlcvLoadResult, NorthflowError> {
        let source = path.display().to_string();
        let raw = std::fs::read_to_string(path)
            .map_err(|e| NorthflowError::DataError(format!("failed to read '{source}': {e}")))?;
        Ok(Self::load_csv(&source, &raw))
    }

    /// Load multiple 1m OHLCV CSV files from disk and merge candles in the
    /// caller-declared file order.
    ///
    /// Each file is parsed with the same validation rules as `load_file`.
    /// The merged stream is then validated without sorting: duplicate or
    /// out-of-order timestamps across file boundaries are reported as data
    /// quality errors so bad yearly inputs are not silently hidden.
    pub fn load_files(paths: &[PathBuf]) -> Result<OhlcvLoadResult, NorthflowError> {
        let source = paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let mut quality = DataQualityReport::new(format!("multi-file:{source}"));
        let mut candles: Vec<Candle> = Vec::new();

        if paths.is_empty() {
            quality.push_issue(
                DataQualityIssueKind::EmptyFile,
                None,
                None,
                "no historical files configured",
            );
            return Ok(OhlcvLoadResult { candles, quality });
        }

        for path in paths {
            let previous_last = candles.last().copied();
            let file_result = Self::load_file(path)?;
            let current_first = file_result.candles.first().copied();

            quality.total_rows += file_result.quality.total_rows;
            quality.rejected_rows += file_result.quality.rejected_rows;
            quality.issues.extend(file_result.quality.issues);
            quality
                .missing_gaps
                .extend(file_result.quality.missing_gaps);

            if let (Some(prev), Some(curr)) = (previous_last, current_first) {
                let prev_ts = prev.timestamp;
                let curr_ts = curr.timestamp;
                let delta = curr_ts - prev_ts;

                if curr_ts == prev_ts {
                    quality.push_issue(
                        DataQualityIssueKind::DuplicateTimestamp,
                        None,
                        Some(curr_ts),
                        format!("duplicate timestamp {curr_ts} after merging historical files"),
                    );
                    quality.rejected_rows += 1;
                } else if curr_ts < prev_ts {
                    quality.push_issue(
                        DataQualityIssueKind::NonMonotonicTimestamp,
                        None,
                        Some(curr_ts),
                        format!("out-of-order timestamp after merging historical files: prev={prev_ts} current={curr_ts}"),
                    );
                } else {
                    match classify_interval_delta(delta, SOURCE_TIMEFRAME_MS) {
                        IntervalClassification::Exact => {}
                        IntervalClassification::MissingGap { missing_count } => {
                            let expected_next = prev_ts + SOURCE_TIMEFRAME_MS;
                            quality.missing_gaps.push(MissingCandleGap {
                                from_timestamp: prev_ts,
                                to_timestamp: curr_ts,
                                expected_next_timestamp: expected_next,
                                missing_count,
                            });
                            quality.push_issue(
                                DataQualityIssueKind::MissingCandleGap,
                                None,
                                Some(expected_next),
                                format!("missing {missing_count} candle(s) between ts={prev_ts} and ts={curr_ts}"),
                            );
                        }
                        IntervalClassification::Irregular => {
                            quality.push_issue(
                                DataQualityIssueKind::IrregularInterval,
                                None,
                                Some(curr_ts),
                                format!("irregular 1m interval after merging historical files: prev={prev_ts} current={curr_ts} delta={delta} expected={SOURCE_TIMEFRAME_MS}"),
                            );
                        }
                    }
                }
            }

            candles.extend(file_result.candles);
        }

        quality.valid_candles = candles.len();
        Ok(OhlcvLoadResult { candles, quality })
    }

    /// Parse raw CSV text into validated candles plus a data quality report.
    ///
    /// This function never panics — all errors are recorded in the report.
    pub fn load_csv(source: &str, raw: &str) -> OhlcvLoadResult {
        let mut quality = DataQualityReport::new(source);
        let mut lines = raw.lines().enumerate();

        // ── locate header ────────────────────────────────────────────────────
        let header_line = loop {
            match lines.next() {
                None => {
                    quality.push_issue(
                        DataQualityIssueKind::EmptyFile,
                        None,
                        None,
                        "file is empty",
                    );
                    return OhlcvLoadResult {
                        candles: Vec::new(),
                        quality,
                    };
                }
                Some((_, line)) if line.trim().is_empty() => continue,
                Some((_, line)) => break line,
            }
        };

        // ── parse column indices (case-insensitive, whitespace-tolerant) ─────
        let cols: Vec<String> = header_line
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();

        let find = |names: &[&str]| -> Option<usize> {
            cols.iter().position(|c| names.contains(&c.as_str()))
        };

        let ts_i = find(&["timestamp", "open_time"]);
        let open_i = find(&["open"]);
        let high_i = find(&["high"]);
        let low_i = find(&["low"]);
        let close_i = find(&["close"]);
        let vol_i = find(&["volume"]);

        let mut missing: Vec<&str> = Vec::new();
        if ts_i.is_none() {
            missing.push("timestamp/open_time");
        }
        if open_i.is_none() {
            missing.push("open");
        }
        if high_i.is_none() {
            missing.push("high");
        }
        if low_i.is_none() {
            missing.push("low");
        }
        if close_i.is_none() {
            missing.push("close");
        }
        if vol_i.is_none() {
            missing.push("volume");
        }

        if !missing.is_empty() {
            quality.push_issue(
                DataQualityIssueKind::MissingRequiredColumn,
                None,
                None,
                format!("missing required columns: {}", missing.join(", ")),
            );
            return OhlcvLoadResult {
                candles: Vec::new(),
                quality,
            };
        }

        let (ts_i, open_i, high_i, low_i, close_i, vol_i) = (
            ts_i.unwrap(),
            open_i.unwrap(),
            high_i.unwrap(),
            low_i.unwrap(),
            close_i.unwrap(),
            vol_i.unwrap(),
        );
        let min_fields = [ts_i, open_i, high_i, low_i, close_i, vol_i]
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            + 1;

        // ── parse data rows ──────────────────────────────────────────────────
        let mut candidates: Vec<Candle> = Vec::new();

        for (line_no, line) in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            quality.total_rows += 1;

            let fields: Vec<&str> = line.split(',').collect();

            if fields.len() < min_fields {
                quality.push_issue(
                    DataQualityIssueKind::MalformedRow,
                    Some(line_no + 1),
                    None,
                    format!(
                        "expected ≥{min_fields} fields, got {} in row '{line}'",
                        fields.len()
                    ),
                );
                quality.rejected_rows += 1;
                continue;
            }

            // Parse timestamp — strictly as positive integer only.
            let ts_str = fields[ts_i].trim();
            let ts_ms = match parse_timestamp_ms(ts_str) {
                Ok(v) => v,
                Err(msg) => {
                    quality.push_issue(
                        DataQualityIssueKind::InvalidTimestamp,
                        Some(line_no + 1),
                        None,
                        format!("cannot parse timestamp '{ts_str}': {msg}"),
                    );
                    quality.rejected_rows += 1;
                    continue;
                }
            };

            // Parse OHLCV — macro avoids repeating error-handling boilerplate.
            macro_rules! parse_f {
                ($idx:expr, $label:expr) => {{
                    match fields[$idx].trim().parse::<f64>() {
                        Ok(v) => v,
                        Err(_) => {
                            quality.push_issue(
                                DataQualityIssueKind::InvalidNumber,
                                Some(line_no + 1),
                                Some(ts_ms),
                                format!("cannot parse {} '{}'", $label, fields[$idx].trim()),
                            );
                            quality.rejected_rows += 1;
                            continue;
                        }
                    }
                }};
            }

            let open = parse_f!(open_i, "open");
            let high = parse_f!(high_i, "high");
            let low = parse_f!(low_i, "low");
            let close = parse_f!(close_i, "close");
            let volume = parse_f!(vol_i, "volume");

            // Validate candle geometry and value ranges.
            let candle = Candle {
                timestamp: ts_ms,
                open,
                high,
                low,
                close,
                volume,
            };
            if let Err(e) = candle.validate() {
                quality.push_issue(
                    DataQualityIssueKind::InvalidCandle,
                    Some(line_no + 1),
                    Some(ts_ms),
                    e.to_string(),
                );
                quality.rejected_rows += 1;
                continue;
            }

            candidates.push(candle);
        }

        // Header-only file (no data rows at all).
        if quality.total_rows == 0 {
            quality.push_issue(
                DataQualityIssueKind::EmptyFile,
                None,
                None,
                "no data rows found (header only)",
            );
            return OhlcvLoadResult {
                candles: Vec::new(),
                quality,
            };
        }

        // ── detect non-monotonic input (report before sorting) ───────────────
        let already_sorted = candidates
            .windows(2)
            .all(|w| w[0].timestamp <= w[1].timestamp);
        if !already_sorted {
            quality.push_issue(
                DataQualityIssueKind::NonMonotonicTimestamp,
                None,
                None,
                "input rows are not ordered by timestamp; sorted automatically",
            );
        }

        // ── sort ascending ───────────────────────────────────────────────────
        candidates.sort_by_key(|c| c.timestamp);

        // ── dedup: keep first occurrence, reject subsequent duplicates ────────
        let mut deduped: Vec<Candle> = Vec::with_capacity(candidates.len());
        for candle in candidates {
            if let Some(last) = deduped.last() {
                if last.timestamp == candle.timestamp {
                    quality.push_issue(
                        DataQualityIssueKind::DuplicateTimestamp,
                        None,
                        Some(candle.timestamp),
                        format!(
                            "duplicate timestamp {}; first occurrence kept",
                            candle.timestamp
                        ),
                    );
                    quality.rejected_rows += 1;
                    continue;
                }
            }
            deduped.push(candle);
        }

        // ── classify every consecutive-candle interval ────────────────────────
        //
        // Valid 1m source data has delta == SOURCE_TIMEFRAME_MS (60_000 ms) between
        // every pair.  Any other delta is either a clean aligned gap or irregular
        // source data:
        //
        //   delta == 60_000               → Exact       (OK)
        //   delta >  60_000, multiple     → MissingGap  (warning; e.g. 120_000, 180_000)
        //   delta >  60_000, non-multiple → Irregular   (error;   e.g. 90_000, 150_000)
        //   delta <  60_000               → Irregular   (error;   e.g. 30_000)
        //   delta <= 0 (defensive)        → Irregular   (error)
        for i in 1..deduped.len() {
            let prev_ts = deduped[i - 1].timestamp;
            let curr_ts = deduped[i].timestamp;
            let delta = curr_ts - prev_ts;

            match classify_interval_delta(delta, SOURCE_TIMEFRAME_MS) {
                IntervalClassification::Exact => {
                    // Correct 1m step — nothing to report.
                }
                IntervalClassification::MissingGap { missing_count } => {
                    let expected_next = prev_ts + SOURCE_TIMEFRAME_MS;
                    quality.missing_gaps.push(MissingCandleGap {
                        from_timestamp: prev_ts,
                        to_timestamp: curr_ts,
                        expected_next_timestamp: expected_next,
                        missing_count,
                    });
                    quality.push_issue(
                        DataQualityIssueKind::MissingCandleGap,
                        None,
                        Some(expected_next),
                        format!(
                            "missing {missing_count} candle(s) between \
                             ts={prev_ts} and ts={curr_ts}"
                        ),
                    );
                }
                IntervalClassification::Irregular => {
                    quality.push_issue(
                        DataQualityIssueKind::IrregularInterval,
                        None,
                        Some(curr_ts),
                        format!(
                            "irregular 1m interval: prev={prev_ts} current={curr_ts} \
                             delta={delta} expected={SOURCE_TIMEFRAME_MS}"
                        ),
                    );
                }
            }
        }

        quality.valid_candles = deduped.len();
        OhlcvLoadResult {
            candles: deduped,
            quality,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HDR: &str = "timestamp,open,high,low,close,volume";
    const HDR_OT: &str = "open_time,open,high,low,close,volume";

    fn row_ms(ts: i64) -> String {
        format!("{ts},100.0,110.0,90.0,105.0,10.0")
    }
    fn row_s(ts_s: i64) -> String {
        format!("{ts_s},100.0,110.0,90.0,105.0,10.0")
    }

    // ── basic load ───────────────────────────────────────────────────────────

    #[test]
    fn loads_valid_csv_with_timestamp_column() {
        let csv = format!("{HDR}\n{}\n", row_ms(1_700_000_000_000));
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.candles.len(), 1);
        assert!(!r.quality.has_errors());
    }

    #[test]
    fn loads_valid_csv_with_open_time_column() {
        let csv = format!("{HDR_OT}\n{}\n", row_ms(1_700_000_000_000));
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.candles.len(), 1);
        assert!(!r.quality.has_errors());
    }

    // ── timestamp normalisation ───────────────────────────────────────────────

    #[test]
    fn normalises_seconds_timestamp_to_milliseconds() {
        let csv = format!("{HDR}\n{}\n", row_s(1_700_000_000));
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.candles.len(), 1);
        assert_eq!(r.candles[0].timestamp, 1_700_000_000_000);
    }

    #[test]
    fn keeps_milliseconds_timestamp_unchanged() {
        let ts = 1_700_000_060_000_i64;
        let csv = format!("{HDR}\n{}\n", row_ms(ts));
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.candles[0].timestamp, ts);
    }

    #[test]
    fn normalises_positive_seconds_timestamp_to_milliseconds() {
        let csv = format!("{HDR}\n{}\n", row_s(1_700_000_000));
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.candles.len(), 1);
        assert_eq!(r.candles[0].timestamp, 1_700_000_000_000);
        assert!(!r.quality.has_errors());
    }

    #[test]
    fn keeps_positive_milliseconds_timestamp_unchanged() {
        let ts = 1_700_000_060_000_i64;
        let csv = format!("{HDR}\n{}\n", row_ms(ts));
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.candles.len(), 1);
        assert_eq!(r.candles[0].timestamp, ts);
        assert!(!r.quality.has_errors());
    }

    // ── strict timestamp rejection ────────────────────────────────────────────

    #[test]
    fn rejects_decimal_timestamp() {
        let csv = format!("{HDR}\n1700000000.5,100.0,110.0,90.0,105.0,10.0\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r.candles.is_empty());
        assert_eq!(r.quality.rejected_rows, 1);
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::InvalidTimestamp));
    }

    #[test]
    fn rejects_nan_timestamp() {
        let csv = format!("{HDR}\nNaN,100.0,110.0,90.0,105.0,10.0\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r.candles.is_empty());
        assert_eq!(r.quality.rejected_rows, 1);
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::InvalidTimestamp));
    }

    #[test]
    fn rejects_infinite_timestamp() {
        for bad in &["inf", "-INF", "Inf", "infinity"] {
            let csv = format!("{HDR}\n{bad},100.0,110.0,90.0,105.0,10.0\n");
            let r = OhlcvLoader::load_csv("test", &csv);
            assert!(r.candles.is_empty(), "expected empty candles for '{bad}'");
            assert!(
                r.quality
                    .issues
                    .iter()
                    .any(|i| i.kind == DataQualityIssueKind::InvalidTimestamp),
                "expected InvalidTimestamp for '{bad}'"
            );
        }
    }

    #[test]
    fn rejects_negative_timestamp() {
        let csv = format!("{HDR}\n-1700000000,100.0,110.0,90.0,105.0,10.0\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r.candles.is_empty());
        assert_eq!(r.quality.rejected_rows, 1);
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::InvalidTimestamp));
    }

    #[test]
    fn rejects_zero_timestamp() {
        let csv = format!("{HDR}\n0,100.0,110.0,90.0,105.0,10.0\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r.candles.is_empty());
        assert_eq!(r.quality.rejected_rows, 1);
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::InvalidTimestamp));
    }

    // ── rejection cases ───────────────────────────────────────────────────────

    #[test]
    fn rejects_missing_required_columns() {
        let csv = "open,high,low,close,volume\n100.0,110.0,90.0,105.0,10.0\n";
        let r = OhlcvLoader::load_csv("test", csv);
        assert!(r.candles.is_empty());
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::MissingRequiredColumn));
    }

    #[test]
    fn rejects_invalid_number() {
        let csv = format!("{HDR}\n1700000000000,notanumber,110.0,90.0,105.0,10.0\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r.candles.is_empty());
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::InvalidNumber));
    }

    #[test]
    fn rejects_invalid_timestamp() {
        let csv = format!("{HDR}\nabc,100.0,110.0,90.0,105.0,10.0\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r.candles.is_empty());
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::InvalidTimestamp));
    }

    #[test]
    fn rejects_invalid_candle_geometry() {
        // high(85) < low(90) → invalid
        let csv = format!("{HDR}\n1700000000000,100.0,85.0,90.0,100.0,10.0\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r.candles.is_empty());
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::InvalidCandle));
    }

    // ── sorting & dedup ───────────────────────────────────────────────────────

    #[test]
    fn sorts_output_candles_ascending() {
        let csv = format!(
            "{HDR}\n{}\n{}\n",
            row_ms(1_700_000_060_000),
            row_ms(1_700_000_000_000),
        );
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.candles.len(), 2);
        assert!(r.candles[0].timestamp < r.candles[1].timestamp);
    }

    #[test]
    fn detects_non_monotonic_input() {
        let csv = format!(
            "{HDR}\n{}\n{}\n",
            row_ms(1_700_000_060_000),
            row_ms(1_700_000_000_000),
        );
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::NonMonotonicTimestamp));
    }

    #[test]
    fn detects_duplicate_timestamp() {
        let ts = 1_700_000_000_000_i64;
        let csv = format!("{HDR}\n{}\n{}\n", row_ms(ts), row_ms(ts));
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.candles.len(), 1);
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::DuplicateTimestamp));
    }

    // ── missing candle gap detection ─────────────────────────────────────────

    #[test]
    fn no_missing_gap_for_continuous_1m_candles() {
        let base = 1_700_000_000_000_i64;
        let rows: String = (0..5)
            .map(|i| row_ms(base + i * SOURCE_TIMEFRAME_MS))
            .collect::<Vec<_>>()
            .join("\n");
        let csv = format!("{HDR}\n{rows}\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r.quality.missing_gaps.is_empty());
        assert_eq!(r.quality.warning_count(), 0);
    }

    #[test]
    fn detects_one_missing_candle() {
        let base = 1_700_000_000_000_i64;
        // jump 2 minutes → 1 candle missing
        let csv = format!(
            "{HDR}\n{}\n{}\n",
            row_ms(base),
            row_ms(base + 2 * SOURCE_TIMEFRAME_MS),
        );
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.quality.missing_gaps.len(), 1);
        assert_eq!(r.quality.missing_gaps[0].missing_count, 1);
    }

    #[test]
    fn detects_multiple_missing_candles() {
        let base = 1_700_000_000_000_i64;
        // jump 5 minutes → 4 candles missing
        let csv = format!(
            "{HDR}\n{}\n{}\n",
            row_ms(base),
            row_ms(base + 5 * SOURCE_TIMEFRAME_MS),
        );
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.quality.missing_gaps.len(), 1);
        assert_eq!(r.quality.missing_gaps[0].missing_count, 4);
        assert_eq!(
            r.quality.missing_gaps[0].expected_next_timestamp,
            base + SOURCE_TIMEFRAME_MS
        );
    }

    // ── irregular interval detection ─────────────────────────────────────────

    #[test]
    fn detects_irregular_sub_minute_interval() {
        // Three candles: t, t+30s, t+2m — delta between first two is 30_000 ms.
        let base = 1_700_000_000_000_i64;
        let csv = format!(
            "{HDR}\n{}\n{}\n{}\n",
            row_ms(base),
            row_ms(base + 30_000), // +30 seconds — irregular
            row_ms(base + 2 * SOURCE_TIMEFRAME_MS),
        );
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::IrregularInterval));
        assert!(r.quality.has_errors());
    }

    #[test]
    fn detects_irregular_90_second_interval() {
        // delta = 90_000 ms — greater than 60_000 but NOT a multiple of 60_000.
        let base = 1_700_000_000_000_i64;
        let csv = format!("{HDR}\n{}\n{}\n", row_ms(base), row_ms(base + 90_000),);
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(
            r.quality
                .issues
                .iter()
                .any(|i| i.kind == DataQualityIssueKind::IrregularInterval),
            "90_000 ms delta must be IrregularInterval"
        );
        assert!(
            r.quality.missing_gaps.is_empty(),
            "90_000 ms delta must not create a MissingCandleGap"
        );
        assert!(r.quality.has_errors());
    }

    #[test]
    fn detects_irregular_150_second_interval() {
        // delta = 150_000 ms — greater than 60_000 but NOT a multiple of 60_000.
        let base = 1_700_000_000_000_i64;
        let csv = format!("{HDR}\n{}\n{}\n", row_ms(base), row_ms(base + 150_000),);
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(
            r.quality
                .issues
                .iter()
                .any(|i| i.kind == DataQualityIssueKind::IrregularInterval),
            "150_000 ms delta must be IrregularInterval"
        );
        assert!(
            r.quality.missing_gaps.is_empty(),
            "150_000 ms delta must not create a MissingCandleGap"
        );
        assert!(r.quality.has_errors());
    }

    #[test]
    fn does_not_create_missing_gap_with_zero_missing_count() {
        // delta = 90_000 ms would give missing_count = 90_000 / 60_000 - 1 = 0
        // if the old non-modulo path were used.  The new path must not do that.
        let base = 1_700_000_000_000_i64;
        let csv = format!("{HDR}\n{}\n{}\n", row_ms(base), row_ms(base + 90_000),);
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(
            r.quality.missing_gaps.is_empty(),
            "no MissingCandleGap must be created for a non-multiple delta"
        );
        assert!(
            !r.quality.missing_gaps.iter().any(|g| g.missing_count == 0),
            "no MissingCandleGap may have missing_count == 0"
        );
    }

    #[test]
    fn still_detects_clean_2m_gap_as_missing_count_1() {
        // delta = 120_000 ms — exact multiple of 60_000 → MissingCandleGap, missing_count=1.
        let base = 1_700_000_000_000_i64;
        let csv = format!("{HDR}\n{}\n{}\n", row_ms(base), row_ms(base + 120_000),);
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.quality.missing_gaps.len(), 1, "expected one missing gap");
        assert_eq!(
            r.quality.missing_gaps[0].missing_count, 1,
            "missing_count must be 1 for a 2m gap"
        );
        assert!(
            r.quality
                .issues
                .iter()
                .any(|i| i.kind == DataQualityIssueKind::MissingCandleGap),
            "MissingCandleGap issue must be recorded"
        );
        assert!(
            !r.quality
                .issues
                .iter()
                .any(|i| i.kind == DataQualityIssueKind::IrregularInterval),
            "IrregularInterval must not be recorded for a clean 2m gap"
        );
    }

    #[test]
    fn still_detects_clean_3m_gap_as_missing_count_2() {
        // delta = 180_000 ms — exact multiple of 60_000 → MissingCandleGap, missing_count=2.
        let base = 1_700_000_000_000_i64;
        let csv = format!("{HDR}\n{}\n{}\n", row_ms(base), row_ms(base + 180_000),);
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.quality.missing_gaps.len(), 1, "expected one missing gap");
        assert_eq!(
            r.quality.missing_gaps[0].missing_count, 2,
            "missing_count must be 2 for a 3m gap"
        );
        assert!(
            r.quality
                .issues
                .iter()
                .any(|i| i.kind == DataQualityIssueKind::MissingCandleGap),
            "MissingCandleGap issue must be recorded"
        );
        assert!(
            !r.quality
                .issues
                .iter()
                .any(|i| i.kind == DataQualityIssueKind::IrregularInterval),
            "IrregularInterval must not be recorded for a clean 3m gap"
        );
    }

    #[test]
    fn classifies_exact_60_second_interval_as_valid() {
        // delta = 60_000 ms — exact source interval → no issue at all.
        let base = 1_700_000_000_000_i64;
        let csv = format!(
            "{HDR}\n{}\n{}\n",
            row_ms(base),
            row_ms(base + SOURCE_TIMEFRAME_MS),
        );
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(
            r.quality.missing_gaps.is_empty(),
            "exact 1m interval must produce no missing gap"
        );
        assert!(
            !r.quality
                .issues
                .iter()
                .any(|i| i.kind == DataQualityIssueKind::IrregularInterval),
            "exact 1m interval must produce no IrregularInterval"
        );
        assert!(
            !r.quality.has_errors(),
            "exact 1m interval must produce no errors"
        );
    }

    #[test]
    fn does_not_flag_irregular_interval_for_valid_1m_sequence() {
        let base = 1_700_000_000_000_i64;
        let rows: String = (0..5)
            .map(|i| row_ms(base + i * SOURCE_TIMEFRAME_MS))
            .collect::<Vec<_>>()
            .join("\n");
        let csv = format!("{HDR}\n{rows}\n");
        let r = OhlcvLoader::load_csv("test", &csv);
        assert!(!r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::IrregularInterval));
    }

    #[test]
    fn still_detects_missing_gap_for_delta_above_60000() {
        let base = 1_700_000_000_000_i64;
        let csv = format!(
            "{HDR}\n{}\n{}\n",
            row_ms(base),
            row_ms(base + 3 * SOURCE_TIMEFRAME_MS), // 3m gap → 2 candles missing
        );
        let r = OhlcvLoader::load_csv("test", &csv);
        assert_eq!(r.quality.missing_gaps.len(), 1);
        assert_eq!(r.quality.missing_gaps[0].missing_count, 2);
        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::MissingCandleGap));
        assert!(!r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::IrregularInterval));
    }

    fn write_temp_csv(name: &str, body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "northflow-{name}-{}-{}.csv",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn load_files_merges_declared_order() {
        let base = 1_700_000_000_000_i64;
        let f1 = write_temp_csv(
            "merge-a",
            &format!(
                "{HDR}\n{}\n{}\n",
                row_ms(base),
                row_ms(base + SOURCE_TIMEFRAME_MS)
            ),
        );
        let f2 = write_temp_csv(
            "merge-b",
            &format!(
                "{HDR}\n{}\n{}\n",
                row_ms(base + 2 * SOURCE_TIMEFRAME_MS),
                row_ms(base + 3 * SOURCE_TIMEFRAME_MS)
            ),
        );

        let r = OhlcvLoader::load_files(&[f1.clone(), f2.clone()]).unwrap();
        let _ = std::fs::remove_file(f1);
        let _ = std::fs::remove_file(f2);

        assert_eq!(r.candles.len(), 4);
        assert_eq!(r.candles[0].timestamp, base);
        assert_eq!(r.candles[3].timestamp, base + 3 * SOURCE_TIMEFRAME_MS);
        assert!(!r.quality.has_errors());
    }

    #[test]
    fn load_files_rejects_duplicate_timestamp_across_files() {
        let base = 1_700_000_000_000_i64;
        let f1 = write_temp_csv("dup-a", &format!("{HDR}\n{}\n", row_ms(base)));
        let f2 = write_temp_csv("dup-b", &format!("{HDR}\n{}\n", row_ms(base)));

        let r = OhlcvLoader::load_files(&[f1.clone(), f2.clone()]).unwrap();
        let _ = std::fs::remove_file(f1);
        let _ = std::fs::remove_file(f2);

        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::DuplicateTimestamp));
        assert!(r.quality.has_errors());
    }

    #[test]
    fn load_files_rejects_out_of_order_file_sequence() {
        let base = 1_700_000_000_000_i64;
        let f1 = write_temp_csv(
            "order-a",
            &format!("{HDR}\n{}\n", row_ms(base + SOURCE_TIMEFRAME_MS)),
        );
        let f2 = write_temp_csv("order-b", &format!("{HDR}\n{}\n", row_ms(base)));

        let r = OhlcvLoader::load_files(&[f1.clone(), f2.clone()]).unwrap();
        let _ = std::fs::remove_file(f1);
        let _ = std::fs::remove_file(f2);

        assert!(r
            .quality
            .issues
            .iter()
            .any(|i| i.kind == DataQualityIssueKind::NonMonotonicTimestamp));
        assert!(r.quality.has_errors());
    }
}
