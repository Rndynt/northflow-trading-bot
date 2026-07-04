#[derive(Debug, Clone, Default)]
pub struct RegressionMetrics {
    pub mae: f64,
    pub rmse: f64,
    pub correlation: f64,
    pub directional_accuracy: f64,
    pub avg_predicted_bps: f64,
    pub avg_actual_bps: f64,
    pub avg_actual_after_cost_bps: f64,
}
#[derive(Debug, Clone)]
pub struct Prediction {
    pub timestamp: i64,
    pub actual_bps: f64,
    pub actual_after_cost_bps: f64,
    pub effective_actual_bps: f64,
    pub predicted_bps: f64,
}
pub fn regression_metrics(p: &[Prediction]) -> RegressionMetrics {
    if p.is_empty() {
        return RegressionMetrics::default();
    }
    let n = p.len() as f64;
    let mae = p
        .iter()
        .map(|x| (x.predicted_bps - x.effective_actual_bps).abs())
        .sum::<f64>()
        / n;
    let rmse = (p
        .iter()
        .map(|x| (x.predicted_bps - x.effective_actual_bps).powi(2))
        .sum::<f64>()
        / n)
        .sqrt();
    let ap = p.iter().map(|x| x.predicted_bps).sum::<f64>() / n;
    let aa = p.iter().map(|x| x.effective_actual_bps).sum::<f64>() / n;
    let ac = p.iter().map(|x| x.actual_after_cost_bps).sum::<f64>() / n;
    let cov = p
        .iter()
        .map(|x| (x.predicted_bps - ap) * (x.effective_actual_bps - aa))
        .sum::<f64>();
    let vp = p
        .iter()
        .map(|x| (x.predicted_bps - ap).powi(2))
        .sum::<f64>();
    let va = p
        .iter()
        .map(|x| (x.effective_actual_bps - aa).powi(2))
        .sum::<f64>();
    let corr = if vp > 0.0 && va > 0.0 {
        cov / (vp.sqrt() * va.sqrt())
    } else {
        0.0
    };
    let da = p
        .iter()
        .filter(|x| (x.predicted_bps >= 0.0) == (x.effective_actual_bps >= 0.0))
        .count() as f64
        / n;
    RegressionMetrics {
        mae,
        rmse,
        correlation: corr,
        directional_accuracy: da,
        avg_predicted_bps: ap,
        avg_actual_bps: aa,
        avg_actual_after_cost_bps: ac,
    }
}
