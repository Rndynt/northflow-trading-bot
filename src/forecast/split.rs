use super::{config::WalkForwardConfig, dataset::ForecastRow};
#[derive(Debug, Clone)]
pub struct WalkForwardWindow {
    pub window_id: usize,
    pub train_start: i64,
    pub train_end: i64,
    pub test_start: i64,
    pub test_end: i64,
    pub train_start_idx: usize,
    pub train_end_idx: usize,
    pub test_start_idx: usize,
    pub test_end_idx: usize,
    pub train_rows: usize,
    pub test_rows: usize,
    pub embargo_bars: usize,
}
pub fn build_windows(rows: &[ForecastRow], cfg: &WalkForwardConfig) -> Vec<WalkForwardWindow> {
    if rows.is_empty() {
        return vec![];
    }
    let bars_per_month = 30 * 24 * 60;
    let train = cfg.train_months * bars_per_month;
    let test = cfg.test_months * bars_per_month;
    let step = cfg.step_months * bars_per_month;
    let mut out = Vec::new();
    let mut start = 0;
    let mut id = 1;
    while start + train + cfg.embargo_bars + test <= rows.len() {
        let tr_s = start;
        let tr_e = start + train - 1;
        let te_s = tr_e + 1 + cfg.embargo_bars;
        let te_e = te_s + test - 1;
        out.push(WalkForwardWindow {
            window_id: id,
            train_start: rows[tr_s].timestamp,
            train_end: rows[tr_e].timestamp,
            test_start: rows[te_s].timestamp,
            test_end: rows[te_e].timestamp,
            train_start_idx: tr_s,
            train_end_idx: tr_e,
            test_start_idx: te_s,
            test_end_idx: te_e,
            train_rows: train,
            test_rows: test,
            embargo_bars: cfg.embargo_bars,
        });
        id += 1;
        start += step;
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    fn r(i: usize) -> ForecastRow {
        ForecastRow {
            timestamp: i as i64,
            close: 1.0,
            features: vec![1.0],
            future_return_bps: 0.0,
            future_return_after_cost_bps: 0.0,
        }
    }
    #[test]
    fn chronological_with_embargo() {
        let rows = (0..90000).map(r).collect::<Vec<_>>();
        let w = build_windows(
            &rows,
            &WalkForwardConfig {
                train_months: 1,
                test_months: 1,
                step_months: 1,
                embargo_bars: 15,
            },
        );
        assert!(w[0].train_end < w[0].test_start);
        assert_eq!(w[0].test_start_idx - w[0].train_end_idx - 1, 15);
    }
}
