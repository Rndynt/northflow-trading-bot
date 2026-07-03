use super::metrics::Prediction;
#[derive(Debug, Clone)]
pub struct PredictionBucket {
    pub bucket_id: usize,
    pub min_prediction_bps: f64,
    pub max_prediction_bps: f64,
    pub row_count: usize,
    pub avg_prediction_bps: f64,
    pub avg_actual_bps: f64,
    pub avg_actual_after_cost_bps: f64,
    pub hit_rate_after_cost: f64,
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
            hit_rate_after_cost: sl.iter().filter(|x| x.actual_after_cost_bps > 0.0).count() as f64
                / n,
        });
    }
    out
}
