use super::{
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
    write_file(Path::new(dir).join("dataset_summary.json"), &format!("{{\n  \"symbol\": \"{}\",\n  \"input_rows\": {},\n  \"output_rows\": {},\n  \"skipped_missing_feature\": {},\n  \"skipped_invalid_feature\": {},\n  \"skipped_label_horizon\": {},\n  \"skipped_invalid_close\": {},\n  \"skipped_invalid_label\": {}\n}}\n",d.symbol,s.input_rows,s.output_rows,s.skipped_missing_feature,s.skipped_invalid_feature,s.skipped_label_horizon,s.skipped_invalid_close,s.skipped_invalid_label))?;
    let mut csv = "feature,rows\n".to_string();
    for f in &d.feature_names {
        csv.push_str(&format!("{},{}\n", f, d.rows.len()))
    }
    write_file(Path::new(dir).join("feature_summary.csv"), &csv)?;
    let avg = if d.rows.is_empty() {
        0.0
    } else {
        d.rows
            .iter()
            .map(|r| r.future_return_after_cost_bps)
            .sum::<f64>()
            / d.rows.len() as f64
    };
    write_file(Path::new(dir).join("label_summary.json"),&format!("{{\n  \"target\": \"future_return_after_cost_bps\",\n  \"rows\": {},\n  \"avg_after_cost_bps\": {:.8}\n}}\n",d.rows.len(),avg))
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
    write_file(Path::new(dir).join(format!("{name}_summary.json")),&format!("{{\n  \"mae\": {:.8},\n  \"rmse\": {:.8},\n  \"correlation\": {:.8},\n  \"directional_accuracy\": {:.8},\n  \"avg_predicted_bps\": {:.8},\n  \"avg_actual_bps\": {:.8},\n  \"avg_actual_after_cost_bps\": {:.8},\n  \"feature_importance_unavailable\": {}\n}}\n",m.mae,m.rmse,m.correlation,m.directional_accuracy,m.avg_predicted_bps,m.avg_actual_bps,m.avg_actual_after_cost_bps, if name=="random_forest"{"true"}else{"false"}))?;
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
    let mut pc = "timestamp,actual_bps,actual_after_cost_bps,predicted_bps\n".to_string();
    for x in p {
        pc.push_str(&format!(
            "{},{:.8},{:.8},{:.8}\n",
            x.timestamp, x.actual_bps, x.actual_after_cost_bps, x.predicted_bps
        ));
    }
    write_file(Path::new(dir).join(format!("{name}_walk_forward.csv")), &pc)
}
fn write_file(path: impl AsRef<Path>, body: &str) -> Result<(), String> {
    let mut f = File::create(path.as_ref())
        .map_err(|e| format!("failed to write {}: {e}", path.as_ref().display()))?;
    f.write_all(body.as_bytes())
        .map_err(|e| format!("failed to write {}: {e}", path.as_ref().display()))
}
