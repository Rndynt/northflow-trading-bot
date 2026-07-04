use super::{
    config::ForecastConfig,
    dataset::{self, ForecastRow},
    evaluation, metrics, models, reports, split,
};
use crate::market::OhlcvLoader;
pub fn run_forecast(cfg: &ForecastConfig) -> Result<(), String> {
    print_plan(cfg);
    reports::ensure_dir(&cfg.reports_dir)?;
    let target = target_selector(cfg);
    for symbol in &cfg.symbols {
        let paths = cfg.historical_paths_for(symbol);
        let missing: Vec<_> = paths.iter().filter(|p| !p.exists()).collect();
        if !missing.is_empty() {
            let mut msg = format!("missing historical data files for {symbol}:\n");
            for p in missing {
                msg.push_str(&format!("  - {}\n", p.display()));
            }
            msg.push_str("Download or place the files listed in config/forecast.toml, or update [historical_files].");
            return Err(msg);
        }
        let loaded = OhlcvLoader::load_files(&paths).map_err(|e| format!("{e}"))?;
        let ds = dataset::build_dataset(symbol, &loaded.candles, cfg);
        reports::write_dataset_reports(&cfg.reports_dir, &ds)?;
        let windows = split::build_windows(&ds.rows, &cfg.walk_forward);
        reports::write_windows(&cfg.reports_dir, &windows)?;
        if windows.is_empty() {
            continue;
        }
        for model in &cfg.enabled_models {
            let mut preds = Vec::new();
            for w in &windows {
                let train = &ds.rows[w.train_start_idx..=w.train_end_idx];
                let test = &ds.rows[w.test_start_idx..=w.test_end_idx];
                let mut p = match model.as_str() {
                    "ridge" => models::ridge::evaluate(
                        train,
                        test,
                        cfg.ridge.alpha,
                        cfg.ridge.standardize,
                        target,
                    ),
                    "random_forest" => models::random_forest::evaluate(
                        train,
                        test,
                        cfg.random_forest.trees,
                        cfg.random_forest.max_depth,
                        cfg.random_forest.min_samples_leaf,
                        cfg.random_forest.feature_subsample_ratio,
                        target,
                    ),
                    _ => vec![],
                };
                preds.append(&mut p);
            }
            let m = metrics::regression_metrics(&preds);
            let b = evaluation::prediction_buckets(&preds);
            reports::write_model(&cfg.reports_dir, model, &m, &b, &preds)?;
            if model == "random_forest" {
                reports::write_random_forest_zero_importance(
                    &cfg.reports_dir,
                    &cfg.enabled_features,
                )?;
            }
        }
    }
    println!(
        "\nForecast research complete: wrote reports to {}",
        cfg.reports_dir
    );
    reports::write_comparison_and_manifest(&cfg.reports_dir, cfg)?;
    Ok(())
}
fn target_selector(cfg: &ForecastConfig) -> fn(&ForecastRow) -> f64 {
    match cfg.effective_target_name() {
        "future_return_bps" => |r: &ForecastRow| r.future_return_bps,
        _ => |r: &ForecastRow| r.future_return_after_cost_bps,
    }
}
fn print_plan(c: &ForecastConfig) {
    println!("Northflow Forecast Research");
    println!("===========================");
    println!("Run Plan");
    println!("--------");
    println!("Mode             : forecast");
    println!("Symbols          : {}", c.symbols.join(", "));
    println!("Source TF        : {}", c.source_timeframe);
    println!("Entry TF         : {}", c.entry_timeframe);
    println!("Forecast Horizon : {}", c.forecast_horizon);
    println!("Models           : {}", c.enabled_models.join(", "));
    println!("Reports Dir      : {}", c.reports_dir);
}
