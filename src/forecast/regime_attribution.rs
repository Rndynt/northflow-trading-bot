//! Lightweight forecast-only helpers for post-run regime attribution.
//!
//! This module is intentionally limited to analysis data structures. It does
//! not emit trading signals, size risk, place orders, or integrate forecast
//! output with the backtest engine.

use super::{dataset::ForecastRow, metrics::Prediction};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RegimeJoinedRow {
    pub timestamp: i64,
    pub predicted_bps: f64,
    pub effective_actual_bps: f64,
    pub features: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegimeAttributionRow {
    pub regime_name: String,
    pub subset: String,
    pub row_count: usize,
    pub top_decile_avg_effective_actual_bps: f64,
    pub top_decile_hit_rate_effective_target: f64,
    pub correlation: f64,
    pub passes_candidate_gate: bool,
}

pub fn join_predictions_with_features(
    dataset_rows: &[ForecastRow],
    predictions: &[Prediction],
) -> Vec<RegimeJoinedRow> {
    let by_timestamp: HashMap<i64, &ForecastRow> =
        dataset_rows.iter().map(|r| (r.timestamp, r)).collect();
    predictions
        .iter()
        .filter_map(|p| {
            let row = by_timestamp.get(&p.timestamp)?;
            Some(RegimeJoinedRow {
                timestamp: p.timestamp,
                predicted_bps: p.predicted_bps,
                effective_actual_bps: p.effective_actual_bps,
                features: row.features.clone(),
            })
        })
        .collect()
}

pub fn summarize_subset(
    regime_name: &str,
    subset: &str,
    rows: &[RegimeJoinedRow],
) -> RegimeAttributionRow {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|a, b| a.predicted_bps.total_cmp(&b.predicted_bps));
    let top_start = sorted.len() * 9 / 10;
    let top = if sorted.is_empty() {
        &sorted[0..0]
    } else {
        &sorted[top_start..]
    };
    let top_n = top.len() as f64;
    let avg = if top.is_empty() {
        0.0
    } else {
        top.iter().map(|r| r.effective_actual_bps).sum::<f64>() / top_n
    };
    let hit = if top.is_empty() {
        0.0
    } else {
        top.iter().filter(|r| r.effective_actual_bps > 0.0).count() as f64 / top_n
    };
    let corr = correlation(rows);
    RegimeAttributionRow {
        regime_name: regime_name.to_string(),
        subset: subset.to_string(),
        row_count: rows.len(),
        top_decile_avg_effective_actual_bps: avg,
        top_decile_hit_rate_effective_target: hit,
        correlation: corr,
        passes_candidate_gate: avg > 0.0 && corr > 0.0,
    }
}

fn correlation(rows: &[RegimeJoinedRow]) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    let n = rows.len() as f64;
    let ap = rows.iter().map(|r| r.predicted_bps).sum::<f64>() / n;
    let aa = rows.iter().map(|r| r.effective_actual_bps).sum::<f64>() / n;
    let cov = rows
        .iter()
        .map(|r| (r.predicted_bps - ap) * (r.effective_actual_bps - aa))
        .sum::<f64>();
    let vp = rows
        .iter()
        .map(|r| (r.predicted_bps - ap).powi(2))
        .sum::<f64>();
    let va = rows
        .iter()
        .map(|r| (r.effective_actual_bps - aa).powi(2))
        .sum::<f64>();
    if vp > 0.0 && va > 0.0 {
        cov / (vp.sqrt() * va.sqrt())
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_predictions_keeps_only_matching_timestamps() {
        let dataset_rows = vec![ForecastRow {
            timestamp: 10,
            close: 100.0,
            features: vec![1.0, 2.0],
            future_return_bps: 3.0,
            future_return_after_cost_bps: 1.0,
        }];
        let predictions = vec![
            Prediction {
                timestamp: 10,
                actual_bps: 3.0,
                actual_after_cost_bps: 1.0,
                effective_actual_bps: 1.0,
                predicted_bps: 2.0,
            },
            Prediction {
                timestamp: 11,
                actual_bps: 0.0,
                actual_after_cost_bps: 0.0,
                effective_actual_bps: 0.0,
                predicted_bps: 0.0,
            },
        ];
        let joined = join_predictions_with_features(&dataset_rows, &predictions);
        assert_eq!(joined.len(), 1);
        assert_eq!(joined[0].features, vec![1.0, 2.0]);
    }

    #[test]
    fn subset_summary_uses_highest_predictions_for_top_decile() {
        let rows = (0..20)
            .map(|i| RegimeJoinedRow {
                timestamp: i,
                predicted_bps: i as f64,
                effective_actual_bps: if i >= 18 { 5.0 } else { -1.0 },
                features: vec![],
            })
            .collect::<Vec<_>>();
        let summary = summarize_subset("trend_proxy", "positive", &rows);
        assert_eq!(summary.row_count, 20);
        assert_eq!(summary.top_decile_avg_effective_actual_bps, 5.0);
        assert_eq!(summary.top_decile_hit_rate_effective_target, 1.0);
        assert!(summary.passes_candidate_gate);
    }
}
