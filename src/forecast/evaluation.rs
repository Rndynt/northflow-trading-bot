use super::metrics::{Prediction, RegressionMetrics};

#[derive(Debug, Clone)]
pub struct PredictionBucket {
    pub bucket_id: usize,
    pub min_prediction_bps: f64,
    pub max_prediction_bps: f64,
    pub row_count: usize,
    pub avg_prediction_bps: f64,
    pub avg_actual_bps: f64,
    pub avg_actual_after_cost_bps: f64,
    pub avg_effective_actual_bps: f64,
    pub hit_rate_after_cost: f64,
    pub hit_rate_effective_target: f64,
}

pub fn prediction_buckets(p: &[Prediction]) -> Vec<PredictionBucket> {
    let mut v = p.to_vec();
    v.sort_by(|a, b| a.predicted_bps.total_cmp(&b.predicted_bps));
    if v.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    for b in 0..10 {
        let s = b * v.len() / 10;
        let e = ((b + 1) * v.len() / 10).min(v.len());
        if s >= e {
            continue;
        }
        let sl = &v[s..e];
        let n = sl.len() as f64;
        out.push(PredictionBucket {
            bucket_id: b + 1,
            min_prediction_bps: sl.first().unwrap().predicted_bps,
            max_prediction_bps: sl.last().unwrap().predicted_bps,
            row_count: sl.len(),
            avg_prediction_bps: sl.iter().map(|x| x.predicted_bps).sum::<f64>() / n,
            avg_actual_bps: sl.iter().map(|x| x.actual_bps).sum::<f64>() / n,
            avg_actual_after_cost_bps: sl.iter().map(|x| x.actual_after_cost_bps).sum::<f64>() / n,
            avg_effective_actual_bps: sl.iter().map(|x| x.effective_actual_bps).sum::<f64>() / n,
            hit_rate_after_cost: sl.iter().filter(|x| x.actual_after_cost_bps > 0.0).count() as f64
                / n,
            hit_rate_effective_target: sl.iter().filter(|x| x.effective_actual_bps > 0.0).count()
                as f64
                / n,
        });
    }
    out
}

/// Aggregated per-model evaluation results collected by the forecast runner,
/// used to build a real (non-placeholder) model comparison report.
#[derive(Debug, Clone)]
pub struct ModelEvaluationResult {
    pub model_name: String,
    pub metrics: RegressionMetrics,
    pub buckets: Vec<PredictionBucket>,
    pub prediction_count: usize,
    pub window_count: usize,
}

impl ModelEvaluationResult {
    /// Top decile is the bucket with the highest `bucket_id` (buckets are built
    /// from predictions sorted ascending, so the last bucket holds the highest
    /// predicted values).
    pub fn top_decile_avg_effective_actual_bps(&self) -> Option<f64> {
        self.buckets
            .iter()
            .max_by_key(|b| b.bucket_id)
            .map(|b| b.avg_effective_actual_bps)
    }
    pub fn top_decile_hit_rate_after_cost(&self) -> Option<f64> {
        self.buckets
            .iter()
            .max_by_key(|b| b.bucket_id)
            .map(|b| b.hit_rate_after_cost)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModelComparison {
    pub best_model_by_rmse: Option<String>,
    pub best_model_by_correlation: Option<String>,
    pub best_model_by_top_decile_return: Option<String>,
    pub recommendation: String,
}

const REC_NO_SIGNAL: &str = "no_predictive_signal_detected";
const REC_WEAK_SIGNAL: &str = "weak_signal_needs_more_validation";
const REC_CANDIDATE: &str = "candidate_for_backtest_filter_phase";
const REC_COST_DECAY: &str = "reject_due_to_cost_adjusted_decay";

/// Compute a real, deterministic model comparison from in-memory per-model
/// evaluation results. Never claims profitability; only ever returns one of
/// the allowed conservative recommendation values.
pub fn compare_models(results: &[ModelEvaluationResult]) -> ModelComparison {
    let with_preds: Vec<&ModelEvaluationResult> =
        results.iter().filter(|r| r.prediction_count > 0).collect();

    if with_preds.is_empty() {
        return ModelComparison {
            best_model_by_rmse: None,
            best_model_by_correlation: None,
            best_model_by_top_decile_return: None,
            recommendation: REC_NO_SIGNAL.to_string(),
        };
    }

    let best_rmse = with_preds
        .iter()
        .min_by(|a, b| a.metrics.rmse.total_cmp(&b.metrics.rmse))
        .unwrap();
    let best_corr = with_preds
        .iter()
        .max_by(|a, b| a.metrics.correlation.total_cmp(&b.metrics.correlation))
        .unwrap();
    let best_topdecile = with_preds
        .iter()
        .max_by(|a, b| {
            let av = a
                .top_decile_avg_effective_actual_bps()
                .unwrap_or(f64::NEG_INFINITY);
            let bv = b
                .top_decile_avg_effective_actual_bps()
                .unwrap_or(f64::NEG_INFINITY);
            av.total_cmp(&bv)
        })
        .unwrap();

    let best_topdecile_val = best_topdecile
        .top_decile_avg_effective_actual_bps()
        .unwrap_or(f64::NEG_INFINITY);
    let best_corr_val = best_corr.metrics.correlation;
    let best_da_val = with_preds
        .iter()
        .map(|r| r.metrics.directional_accuracy)
        .fold(f64::MIN, f64::max);

    let recommendation = if best_topdecile_val <= 0.0 {
        REC_COST_DECAY
    } else if best_corr_val <= 0.0 && best_da_val <= 0.50 {
        REC_NO_SIGNAL
    } else if best_topdecile_val > 0.0 && best_corr_val < 0.02 {
        REC_WEAK_SIGNAL
    } else {
        REC_CANDIDATE
    }
    .to_string();

    ModelComparison {
        best_model_by_rmse: Some(best_rmse.model_name.clone()),
        best_model_by_correlation: Some(best_corr.model_name.clone()),
        best_model_by_top_decile_return: Some(best_topdecile.model_name.clone()),
        recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pred(predicted: f64, effective: f64) -> Prediction {
        Prediction {
            timestamp: 0,
            actual_bps: effective,
            actual_after_cost_bps: effective,
            effective_actual_bps: effective,
            predicted_bps: predicted,
        }
    }

    fn metrics_with(rmse: f64, correlation: f64, directional_accuracy: f64) -> RegressionMetrics {
        RegressionMetrics {
            mae: 0.0,
            rmse,
            correlation,
            directional_accuracy,
            avg_predicted_bps: 0.0,
            avg_actual_bps: 0.0,
            avg_actual_after_cost_bps: 0.0,
        }
    }

    fn result(
        name: &str,
        rmse: f64,
        correlation: f64,
        directional_accuracy: f64,
        top_decile_bps: f64,
        prediction_count: usize,
    ) -> ModelEvaluationResult {
        let buckets = if prediction_count == 0 {
            vec![]
        } else {
            vec![
                PredictionBucket {
                    bucket_id: 1,
                    min_prediction_bps: -10.0,
                    max_prediction_bps: -1.0,
                    row_count: 1,
                    avg_prediction_bps: -5.0,
                    avg_actual_bps: -5.0,
                    avg_actual_after_cost_bps: -5.0,
                    avg_effective_actual_bps: -5.0,
                    hit_rate_after_cost: 0.0,
                    hit_rate_effective_target: 0.0,
                },
                PredictionBucket {
                    bucket_id: 10,
                    min_prediction_bps: 1.0,
                    max_prediction_bps: 10.0,
                    row_count: 1,
                    avg_prediction_bps: 5.0,
                    avg_actual_bps: top_decile_bps,
                    avg_actual_after_cost_bps: top_decile_bps,
                    avg_effective_actual_bps: top_decile_bps,
                    hit_rate_after_cost: if top_decile_bps > 0.0 { 1.0 } else { 0.0 },
                    hit_rate_effective_target: if top_decile_bps > 0.0 { 1.0 } else { 0.0 },
                },
            ]
        };
        ModelEvaluationResult {
            model_name: name.to_string(),
            metrics: metrics_with(rmse, correlation, directional_accuracy),
            buckets,
            prediction_count,
            window_count: if prediction_count > 0 { 1 } else { 0 },
        }
    }

    #[test]
    fn prediction_bucket_uses_effective_target_average() {
        let preds = vec![pred(1.0, 2.0), pred(2.0, 4.0), pred(3.0, -6.0)];
        let buckets = prediction_buckets(&preds);
        assert!(!buckets.is_empty());
        for (b, p) in buckets.iter().zip(preds.iter()) {
            assert!((b.avg_effective_actual_bps - p.effective_actual_bps).abs() < 1e-9);
        }
    }

    #[test]
    fn model_comparison_empty_predictions_returns_null_and_no_signal() {
        let results = vec![
            result("ridge", 0.0, 0.0, 0.0, 0.0, 0),
            result("random_forest", 0.0, 0.0, 0.0, 0.0, 0),
        ];
        let cmp = compare_models(&results);
        assert!(cmp.best_model_by_rmse.is_none());
        assert!(cmp.best_model_by_correlation.is_none());
        assert!(cmp.best_model_by_top_decile_return.is_none());
        assert_eq!(cmp.recommendation, REC_NO_SIGNAL);
    }

    #[test]
    fn model_comparison_picks_best_rmse() {
        let results = vec![
            result("ridge", 14.0, 0.05, 0.55, 3.0, 100),
            result("random_forest", 9.0, 0.01, 0.52, 1.0, 100),
        ];
        let cmp = compare_models(&results);
        assert_eq!(cmp.best_model_by_rmse.as_deref(), Some("random_forest"));
    }

    #[test]
    fn model_comparison_picks_best_correlation() {
        let results = vec![
            result("ridge", 14.0, 0.03, 0.55, 3.0, 100),
            result("random_forest", 9.0, 0.08, 0.52, 1.0, 100),
        ];
        let cmp = compare_models(&results);
        assert_eq!(
            cmp.best_model_by_correlation.as_deref(),
            Some("random_forest")
        );
    }

    #[test]
    fn model_comparison_picks_best_top_decile_effective_return() {
        let results = vec![
            result("ridge", 14.0, 0.03, 0.55, 2.0, 100),
            result("random_forest", 9.0, 0.08, 0.52, 6.0, 100),
        ];
        let cmp = compare_models(&results);
        assert_eq!(
            cmp.best_model_by_top_decile_return.as_deref(),
            Some("random_forest")
        );
    }

    #[test]
    fn model_comparison_rejects_due_to_cost_adjusted_decay() {
        let results = vec![result("ridge", 14.0, 0.05, 0.55, -1.0, 100)];
        let cmp = compare_models(&results);
        assert_eq!(cmp.recommendation, REC_COST_DECAY);
    }

    #[test]
    fn model_comparison_no_signal_when_correlation_and_directional_accuracy_are_weak() {
        let results = vec![result("ridge", 14.0, -0.01, 0.49, 0.5, 100)];
        let cmp = compare_models(&results);
        assert_eq!(cmp.recommendation, REC_NO_SIGNAL);
    }

    #[test]
    fn model_comparison_weak_signal_needs_more_validation() {
        let results = vec![result("ridge", 14.0, 0.01, 0.55, 2.0, 100)];
        let cmp = compare_models(&results);
        assert_eq!(cmp.recommendation, REC_WEAK_SIGNAL);
    }

    #[test]
    fn model_comparison_candidate_for_backtest_filter_phase() {
        let results = vec![result("ridge", 14.0, 0.05, 0.55, 2.0, 100)];
        let cmp = compare_models(&results);
        assert_eq!(cmp.recommendation, REC_CANDIDATE);
    }
}
