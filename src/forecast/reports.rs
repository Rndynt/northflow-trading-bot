use super::{
    config::ForecastConfig,
    dataset::ForecastDataset,
    evaluation::{self, ModelEvaluationResult, PredictionBucket},
    metrics::{Prediction, RegressionMetrics},
    split::WalkForwardWindow,
};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

pub fn ensure_dir(p: &str) -> Result<(), String> {
    fs::create_dir_all(p).map_err(|e| format!("failed to create reports dir {p}: {e}"))
}

pub fn write_dataset_reports(dir: &str, d: &ForecastDataset) -> Result<(), String> {
    ensure_dir(dir)?;
    let s = &d.summary;
    write_file(Path::new(dir).join("dataset_summary.json"), &format!("{{\n  \"symbol\": \"{}\",\n  \"input_rows\": {},\n  \"output_rows\": {},\n  \"feature_count\": {},\n  \"skipped_missing_feature\": {},\n  \"skipped_invalid_feature\": {},\n  \"skipped_label_horizon\": {},\n  \"skipped_invalid_close\": {},\n  \"skipped_invalid_label\": {}\n}}\n", d.symbol, s.input_rows, s.output_rows, d.feature_names.len(), s.skipped_missing_feature, s.skipped_invalid_feature, s.skipped_label_horizon, s.skipped_invalid_close, s.skipped_invalid_label))?;
    let mut csv = "feature,rows\n".to_string();
    for f in &d.feature_names {
        csv.push_str(&format!("{},{}\n", f, d.rows.len()));
    }
    write_file(Path::new(dir).join("feature_summary.csv"), &csv)?;
    let avg_raw = avg(d.rows.iter().map(|r| r.future_return_bps));
    let avg_cost = avg(d.rows.iter().map(|r| r.future_return_after_cost_bps));
    write_file(Path::new(dir).join("label_summary.json"), &format!("{{\n  \"rows\": {},\n  \"avg_future_return_bps\": {:.8},\n  \"avg_future_return_after_cost_bps\": {:.8}\n}}\n", d.rows.len(), avg_raw, avg_cost))
}
fn avg<I: Iterator<Item = f64>>(it: I) -> f64 {
    let v: Vec<f64> = it.collect();
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

pub fn write_windows(dir: &str, w: &[WalkForwardWindow]) -> Result<(), String> {
    let mut csv =
        "window_id,train_start,train_end,test_start,test_end,train_rows,test_rows,embargo_bars\n"
            .to_string();
    for x in w {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            x.window_id,
            x.train_start,
            x.train_end,
            x.test_start,
            x.test_end,
            x.train_rows,
            x.test_rows,
            x.embargo_bars
        ));
    }
    write_file(Path::new(dir).join("walk_forward_windows.csv"), &csv)
}

/// Writes only the per-model summary, prediction-bucket CSV, and walk-forward
/// CSV. Random Forest feature importance has exactly one dedicated writer:
/// `write_random_forest_importance`. This avoids duplicated/conflicting
/// writes of the same report file.
pub fn write_model(
    dir: &str,
    name: &str,
    m: &RegressionMetrics,
    b: &[PredictionBucket],
    p: &[Prediction],
) -> Result<(), String> {
    write_file(Path::new(dir).join(format!("{name}_summary.json")), &format!("{{\n  \"mae\": {:.8},\n  \"rmse\": {:.8},\n  \"correlation\": {:.8},\n  \"directional_accuracy\": {:.8},\n  \"avg_predicted_bps\": {:.8},\n  \"avg_actual_bps\": {:.8},\n  \"avg_actual_after_cost_bps\": {:.8},\n  \"prediction_count\": {}\n}}\n", m.mae,m.rmse,m.correlation,m.directional_accuracy,m.avg_predicted_bps,m.avg_actual_bps,m.avg_actual_after_cost_bps,p.len()))?;
    let mut bc = "bucket_id,min_prediction_bps,max_prediction_bps,row_count,avg_prediction_bps,avg_actual_bps,avg_actual_after_cost_bps,avg_effective_actual_bps,hit_rate_after_cost,hit_rate_effective_target\n".to_string();
    for x in b {
        bc.push_str(&format!(
            "{},{:.8},{:.8},{},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8}\n",
            x.bucket_id,
            x.min_prediction_bps,
            x.max_prediction_bps,
            x.row_count,
            x.avg_prediction_bps,
            x.avg_actual_bps,
            x.avg_actual_after_cost_bps,
            x.avg_effective_actual_bps,
            x.hit_rate_after_cost,
            x.hit_rate_effective_target
        ));
    }
    write_file(
        Path::new(dir).join(format!("{name}_prediction_buckets.csv")),
        &bc,
    )?;
    let mut pc = "timestamp,actual_bps,actual_after_cost_bps,effective_actual_bps,predicted_bps\n"
        .to_string();
    for x in p {
        pc.push_str(&format!(
            "{},{:.8},{:.8},{:.8},{:.8}\n",
            x.timestamp,
            x.actual_bps,
            x.actual_after_cost_bps,
            x.effective_actual_bps,
            x.predicted_bps
        ));
    }
    write_file(Path::new(dir).join(format!("{name}_walk_forward.csv")), &pc)
}

/// Dedicated Random Forest feature-importance writer using real accumulated
/// split counts. This is the *only* writer of
/// `random_forest_feature_importance.csv`. If no splits occurred (a genuine
/// no-split case, e.g. every tree stayed a single leaf), it writes zero
/// importance for every enabled feature with method `split_count_no_splits`.
pub fn write_random_forest_importance(
    dir: &str,
    feature_names: &[String],
    split_counts: &[usize],
) -> Result<(), String> {
    let total: usize = split_counts.iter().sum();
    let mut csv = "feature,split_count,importance,method\n".to_string();
    for (i, f) in feature_names.iter().enumerate() {
        let count = split_counts.get(i).copied().unwrap_or(0);
        if total > 0 {
            let importance = count as f64 / total as f64;
            csv.push_str(&format!("{f},{count},{importance:.8},split_count\n"));
        } else {
            csv.push_str(&format!("{f},0,0.00000000,split_count_no_splits\n"));
        }
    }
    write_file(
        Path::new(dir).join("random_forest_feature_importance.csv"),
        &csv,
    )
}

/// Writes `model_comparison.json` computed from real in-memory evaluation
/// results (not inferred from config alone), and `forecast_run_manifest.json`
/// listing the reports actually written during the run.
pub fn write_comparison_and_manifest(
    dir: &str,
    cfg: &ForecastConfig,
    results: &[ModelEvaluationResult],
    reports_written: &[String],
) -> Result<(), String> {
    let cmp = evaluation::compare_models(results);

    let models_compared = json_str_array(&cfg.enabled_models);
    let models_json = results
        .iter()
        .map(|r| {
            format!(
                "    {{\n      \"model_name\": \"{}\",\n      \"prediction_count\": {},\n      \"window_count\": {},\n      \"mae\": {:.8},\n      \"rmse\": {:.8},\n      \"correlation\": {:.8},\n      \"directional_accuracy\": {:.8},\n      \"avg_predicted_bps\": {:.8},\n      \"avg_actual_bps\": {:.8},\n      \"avg_actual_after_cost_bps\": {:.8},\n      \"top_decile_avg_effective_actual_bps\": {:.8},\n      \"top_decile_hit_rate_after_cost\": {:.8}\n    }}",
                r.model_name,
                r.prediction_count,
                r.window_count,
                r.metrics.mae,
                r.metrics.rmse,
                r.metrics.correlation,
                r.metrics.directional_accuracy,
                r.metrics.avg_predicted_bps,
                r.metrics.avg_actual_bps,
                r.metrics.avg_actual_after_cost_bps,
                r.top_decile_avg_effective_actual_bps().unwrap_or(0.0),
                r.top_decile_hit_rate_after_cost().unwrap_or(0.0),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    write_file(
        Path::new(dir).join("model_comparison.json"),
        &format!(
            "{{\n  \"configured_target\": \"{}\",\n  \"effective_target\": \"{}\",\n  \"horizon_bars\": {},\n  \"models_compared\": {},\n  \"best_model_by_rmse\": {},\n  \"best_model_by_correlation\": {},\n  \"best_model_by_top_decile_return\": {},\n  \"recommendation\": \"{}\",\n  \"models\": [\n{}\n  ]\n}}\n",
            cfg.label_target,
            cfg.effective_target_name(),
            cfg.horizon_bars,
            models_compared,
            opt_json_str(&cmp.best_model_by_rmse),
            opt_json_str(&cmp.best_model_by_correlation),
            opt_json_str(&cmp.best_model_by_top_decile_return),
            cmp.recommendation,
            models_json,
        ),
    )?;

    let symbols = json_str_array(&cfg.symbols);
    let features = json_str_array(&cfg.enabled_features);
    let models = json_str_array(&cfg.enabled_models);
    let reports = json_str_array(reports_written);
    let limitations = json_str_array(&[
        "source timeframe currently supports only 1m".to_string(),
        "walk-forward months use fixed 30-day approximation".to_string(),
        "classification target is not implemented".to_string(),
        "forecast module does not emit production trading signals".to_string(),
        "paper/live remain disabled".to_string(),
    ]);

    write_file(
        Path::new(dir).join("forecast_run_manifest.json"),
        &format!(
            "{{\n  \"run_mode\": \"forecast\",\n  \"symbols\": {},\n  \"source_timeframe\": \"{}\",\n  \"entry_timeframe\": \"{}\",\n  \"forecast_horizon\": \"{}\",\n  \"horizon_bars\": {},\n  \"configured_target\": \"{}\",\n  \"effective_target\": \"{}\",\n  \"cost_adjusted\": {},\n  \"enabled_features\": {},\n  \"enabled_models\": {},\n  \"walk_forward\": {{\n    \"train_months\": {},\n    \"test_months\": {},\n    \"step_months\": {},\n    \"embargo_bars\": {},\n    \"month_model\": \"fixed_30_day_months\"\n  }},\n  \"reports_written\": {},\n  \"limitations\": {}\n}}\n",
            symbols,
            cfg.source_timeframe,
            cfg.entry_timeframe,
            cfg.forecast_horizon,
            cfg.horizon_bars,
            cfg.label_target,
            cfg.effective_target_name(),
            cfg.cost_adjusted,
            features,
            models,
            cfg.walk_forward.train_months,
            cfg.walk_forward.test_months,
            cfg.walk_forward.step_months,
            cfg.walk_forward.embargo_bars,
            reports,
            limitations,
        ),
    )
}

fn json_str_array(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let inner = items
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{inner}]")
}

fn opt_json_str(v: &Option<String>) -> String {
    match v {
        Some(s) => format!("\"{s}\""),
        None => "null".to_string(),
    }
}

fn write_file(path: impl AsRef<Path>, body: &str) -> Result<(), String> {
    let mut f = File::create(path.as_ref())
        .map_err(|e| format!("failed to write {}: {e}", path.as_ref().display()))?;
    f.write_all(body.as_bytes())
        .map_err(|e| format!("failed to write {}: {e}", path.as_ref().display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast::evaluation::PredictionBucket;
    use crate::forecast::metrics::RegressionMetrics;
    use std::fs;

    fn tmp_dir(name: &str) -> String {
        let dir = format!("/tmp/forecast_reports_test_{name}");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn random_forest_importance_uses_real_split_counts_and_sums_to_one() {
        let dir = tmp_dir("rf_importance");
        let features = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let split_counts = vec![3usize, 1usize, 0usize];
        write_random_forest_importance(&dir, &features, &split_counts).unwrap();
        let content =
            fs::read_to_string(format!("{dir}/random_forest_feature_importance.csv")).unwrap();
        assert!(content.contains("a,3,0.75000000,split_count"));
        assert!(content.contains("b,1,0.25000000,split_count"));
        assert!(content.contains("c,0,0.00000000,split_count"));
        assert!(!content.contains("split_count_no_splits"));
    }

    #[test]
    fn random_forest_importance_no_split_fallback_lists_all_features() {
        let dir = tmp_dir("rf_no_split");
        let features = vec!["a".to_string(), "b".to_string()];
        let split_counts = vec![0usize, 0usize];
        write_random_forest_importance(&dir, &features, &split_counts).unwrap();
        let content =
            fs::read_to_string(format!("{dir}/random_forest_feature_importance.csv")).unwrap();
        assert!(content.contains("a,0,0.00000000,split_count_no_splits"));
        assert!(content.contains("b,0,0.00000000,split_count_no_splits"));
    }

    #[test]
    fn write_model_does_not_write_random_forest_importance_file() {
        let dir = tmp_dir("write_model_no_dup");
        let m = RegressionMetrics::default();
        write_model(&dir, "random_forest", &m, &[], &[]).unwrap();
        assert!(!Path::new(&dir)
            .join("random_forest_feature_importance.csv")
            .exists());
    }

    #[test]
    fn manifest_contains_required_fields() {
        let dir = tmp_dir("manifest_fields");
        let cfg = ForecastConfig::default();
        write_comparison_and_manifest(&dir, &cfg, &[], &["dataset_summary.json".to_string()])
            .unwrap();
        let content = fs::read_to_string(format!("{dir}/forecast_run_manifest.json")).unwrap();
        for field in [
            "run_mode",
            "symbols",
            "source_timeframe",
            "entry_timeframe",
            "forecast_horizon",
            "horizon_bars",
            "configured_target",
            "effective_target",
            "cost_adjusted",
            "enabled_features",
            "enabled_models",
            "walk_forward",
            "train_months",
            "test_months",
            "step_months",
            "embargo_bars",
            "month_model",
            "fixed_30_day_months",
            "reports_written",
            "limitations",
        ] {
            assert!(content.contains(field), "missing field: {field}");
        }
    }

    #[test]
    fn model_comparison_json_handles_empty_results_cleanly() {
        let dir = tmp_dir("comparison_empty");
        let cfg = ForecastConfig::default();
        write_comparison_and_manifest(&dir, &cfg, &[], &[]).unwrap();
        let content = fs::read_to_string(format!("{dir}/model_comparison.json")).unwrap();
        assert!(content.contains("\"best_model_by_rmse\": null"));
        assert!(content.contains("no_predictive_signal_detected"));
    }

    #[test]
    fn model_comparison_json_uses_real_metrics_when_present() {
        let dir = tmp_dir("comparison_real");
        let cfg = ForecastConfig::default();
        let bucket = PredictionBucket {
            bucket_id: 10,
            min_prediction_bps: 1.0,
            max_prediction_bps: 5.0,
            row_count: 10,
            avg_prediction_bps: 3.0,
            avg_actual_bps: 2.0,
            avg_actual_after_cost_bps: 2.0,
            avg_effective_actual_bps: 2.0,
            hit_rate_after_cost: 0.6,
            hit_rate_effective_target: 0.6,
        };
        let result = ModelEvaluationResult {
            model_name: "ridge".to_string(),
            metrics: RegressionMetrics {
                mae: 1.0,
                rmse: 2.0,
                correlation: 0.05,
                directional_accuracy: 0.55,
                avg_predicted_bps: 1.0,
                avg_actual_bps: 1.0,
                avg_actual_after_cost_bps: 1.0,
            },
            buckets: vec![bucket],
            prediction_count: 10,
            window_count: 1,
        };
        write_comparison_and_manifest(&dir, &cfg, &[result], &[]).unwrap();
        let content = fs::read_to_string(format!("{dir}/model_comparison.json")).unwrap();
        assert!(content.contains("\"best_model_by_rmse\": \"ridge\""));
        assert!(!content.contains("\"best_model_by_rmse\": null"));
    }
}
