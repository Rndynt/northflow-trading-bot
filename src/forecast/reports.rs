use super::{
    config::ForecastConfig,
    dataset::ForecastDataset,
    evaluation::PredictionBucket,
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

pub fn write_model(
    dir: &str,
    name: &str,
    m: &RegressionMetrics,
    b: &[PredictionBucket],
    p: &[Prediction],
) -> Result<(), String> {
    write_file(Path::new(dir).join(format!("{name}_summary.json")), &format!("{{\n  \"mae\": {:.8},\n  \"rmse\": {:.8},\n  \"correlation\": {:.8},\n  \"directional_accuracy\": {:.8},\n  \"avg_predicted_bps\": {:.8},\n  \"avg_actual_bps\": {:.8},\n  \"avg_actual_after_cost_bps\": {:.8},\n  \"prediction_count\": {}\n}}\n", m.mae,m.rmse,m.correlation,m.directional_accuracy,m.avg_predicted_bps,m.avg_actual_bps,m.avg_actual_after_cost_bps,p.len()))?;
    let mut bc="bucket_id,min_prediction_bps,max_prediction_bps,row_count,avg_prediction_bps,avg_actual_bps,avg_actual_after_cost_bps,hit_rate_after_cost\n".to_string();
    for x in b {
        bc.push_str(&format!(
            "{},{:.8},{:.8},{},{:.8},{:.8},{:.8},{:.8}\n",
            x.bucket_id,
            x.min_prediction_bps,
            x.max_prediction_bps,
            x.row_count,
            x.avg_prediction_bps,
            x.avg_actual_bps,
            x.avg_actual_after_cost_bps,
            x.hit_rate_after_cost
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
    write_file(Path::new(dir).join(format!("{name}_walk_forward.csv")), &pc)?;
    if name == "random_forest" {
        write_file(Path::new(dir).join("random_forest_feature_importance.csv"), "feature,split_count,importance,method\nunknown,0,0.00000000,split_count_no_splits_or_not_collected\n")?;
    }
    Ok(())
}

pub fn write_comparison_and_manifest(dir: &str, cfg: &ForecastConfig) -> Result<(), String> {
    let models = cfg.enabled_models.join("\", \"");
    let features = cfg.enabled_features.join("\", \"");
    let symbols = cfg.symbols.join("\", \"");
    write_file(Path::new(dir).join("model_comparison.json"), &format!("{{\n  \"configured_target\": \"{}\",\n  \"effective_target\": \"{}\",\n  \"horizon_bars\": {},\n  \"models_compared\": [\"{}\"],\n  \"best_model_by_rmse\": null,\n  \"best_model_by_correlation\": null,\n  \"best_model_by_top_decile_return\": null,\n  \"recommendation\": \"weak_signal_needs_more_validation\",\n  \"note\": \"Use per-model summaries for metrics; no profitability is claimed.\"\n}}\n", cfg.label_target, cfg.effective_target_name(), cfg.horizon_bars, models))?;
    write_file(Path::new(dir).join("forecast_run_manifest.json"), &format!("{{\n  \"run_mode\": \"forecast\",\n  \"symbols\": [\"{}\"],\n  \"source_timeframe\": \"{}\",\n  \"entry_timeframe\": \"{}\",\n  \"horizon_bars\": {},\n  \"configured_target\": \"{}\",\n  \"effective_target\": \"{}\",\n  \"cost_adjusted\": {},\n  \"features\": [\"{}\"],\n  \"models\": [\"{}\"],\n  \"walk_forward_fixed_30_day_month_policy\": true,\n  \"limitations\": [\"classification target not implemented\", \"fixed 30-day walk-forward months\"]\n}}\n", symbols, cfg.source_timeframe, cfg.entry_timeframe, cfg.horizon_bars, cfg.label_target, cfg.effective_target_name(), cfg.cost_adjusted, features, models))
}
fn write_file(path: impl AsRef<Path>, body: &str) -> Result<(), String> {
    let mut f = File::create(path.as_ref())
        .map_err(|e| format!("failed to write {}: {e}", path.as_ref().display()))?;
    f.write_all(body.as_bytes())
        .map_err(|e| format!("failed to write {}: {e}", path.as_ref().display()))
}

pub fn write_random_forest_zero_importance(dir: &str, features: &[String]) -> Result<(), String> {
    let mut csv = "feature,split_count,importance,method\n".to_string();
    for f in features {
        csv.push_str(&format!(
            "{},0,0.00000000,split_count_no_splits_or_not_collected\n",
            f
        ));
    }
    write_file(
        Path::new(dir).join("random_forest_feature_importance.csv"),
        &csv,
    )
}
