use super::{
    config::ForecastConfig,
    dataset::{self, ForecastRow},
    evaluation::{self, ModelEvaluationResult},
    metrics, models, reports, split,
};
use crate::market::OhlcvLoader;
use std::{
    io::{self, Write},
    path::PathBuf,
    time::Instant,
};

pub fn run_forecast(cfg: &ForecastConfig) -> Result<(), String> {
    let run_started = Instant::now();
    print_plan(cfg);
    reports::ensure_dir(&cfg.reports_dir)?;
    let target = target_selector(cfg);
    let mut reports_written: Vec<String> = Vec::new();
    let mut eval_results: Vec<ModelEvaluationResult> = Vec::new();

    for symbol in &cfg.symbols {
        println!();
        println!("Dataset");
        println!("-------");
        println!("Symbol           : {symbol}");
        flush_stdout();

        let paths = cfg.historical_paths_for(symbol);
        println!("Historical files : {}", paths.len());
        for (i, path) in paths.iter().enumerate() {
            println!("  {}. {}", i + 1, path.display());
        }
        flush_stdout();

        let missing: Vec<&PathBuf> = paths.iter().filter(|p| !p.exists()).collect();
        if !missing.is_empty() {
            return Err(format_missing_data(symbol, &missing));
        }

        let load_started = Instant::now();
        println!("Loading candles  : started");
        flush_stdout();
        let loaded = OhlcvLoader::load_files(&paths).map_err(|e| format!("{e}"))?;
        println!(
            "Loading candles  : done | rows={} | elapsed={:.1}s",
            loaded.candles.len(),
            load_started.elapsed().as_secs_f64()
        );
        flush_stdout();

        let dataset_started = Instant::now();
        println!("Building dataset : started");
        flush_stdout();
        let ds = dataset::build_dataset(symbol, &loaded.candles, cfg);
        println!(
            "Building dataset : done | output_rows={} | skipped_missing_feature={} | skipped_label_horizon={} | elapsed={:.1}s",
            ds.rows.len(),
            ds.summary.skipped_missing_feature,
            ds.summary.skipped_label_horizon,
            dataset_started.elapsed().as_secs_f64()
        );
        flush_stdout();
        reports::write_dataset_reports(&cfg.reports_dir, &ds)?;
        reports_written.push("dataset_summary.json".to_string());
        reports_written.push("feature_summary.csv".to_string());
        reports_written.push("label_summary.json".to_string());

        let windows = split::build_windows(&ds.rows, &cfg.walk_forward);
        reports::write_windows(&cfg.reports_dir, &windows)?;
        reports_written.push("walk_forward_windows.csv".to_string());
        println!();
        println!("Walk Forward");
        println!("------------");
        println!("Windows          : {}", windows.len());
        println!("Train months     : {}", cfg.walk_forward.train_months);
        println!("Test months      : {}", cfg.walk_forward.test_months);
        println!("Step months      : {}", cfg.walk_forward.step_months);
        println!("Embargo bars     : {}", cfg.walk_forward.embargo_bars);
        flush_stdout();
        if windows.is_empty() {
            println!(
                "No walk-forward windows were produced; skipping model evaluation for {symbol}."
            );
            flush_stdout();
            continue;
        }

        for model in &cfg.enabled_models {
            let model_started = Instant::now();
            println!();
            println!("Model: {model}");
            println!("{}", "-".repeat(7 + model.len()));
            flush_stdout();

            let mut preds = Vec::new();
            let mut rf_split_counts: Vec<usize> = vec![0; cfg.enabled_features.len()];
            for (window_pos, w) in windows.iter().enumerate() {
                let window_started = Instant::now();
                let train = &ds.rows[w.train_start_idx..=w.train_end_idx];
                let test = &ds.rows[w.test_start_idx..=w.test_end_idx];
                println!(
                    "  Window {}/{} | train_rows={} | test_rows={} | train={}..{} | test={}..{}",
                    window_pos + 1,
                    windows.len(),
                    train.len(),
                    test.len(),
                    w.train_start,
                    w.train_end,
                    w.test_start,
                    w.test_end
                );
                flush_stdout();

                match model.as_str() {
                    "ridge" => {
                        let mut p = models::ridge::evaluate(
                            train,
                            test,
                            cfg.ridge.alpha,
                            cfg.ridge.standardize,
                            target,
                        );
                        println!(
                            "    ridge window {}/{} done | predictions={} | elapsed={:.1}s",
                            window_pos + 1,
                            windows.len(),
                            p.len(),
                            window_started.elapsed().as_secs_f64()
                        );
                        flush_stdout();
                        preds.append(&mut p);
                    }
                    "random_forest" => {
                        let label = format!("window {}/{}", window_pos + 1, windows.len());
                        let mut r = models::random_forest::evaluate_with_progress(
                            train,
                            test,
                            cfg.random_forest.trees,
                            cfg.random_forest.max_depth,
                            cfg.random_forest.min_samples_leaf,
                            cfg.random_forest.feature_subsample_ratio,
                            target,
                            Some(&label),
                        );
                        println!(
                            "    random_forest window {}/{} done | predictions={} | elapsed={:.1}s",
                            window_pos + 1,
                            windows.len(),
                            r.predictions.len(),
                            window_started.elapsed().as_secs_f64()
                        );
                        flush_stdout();
                        preds.append(&mut r.predictions);
                        for (i, c) in r.split_counts.iter().enumerate() {
                            if let Some(slot) = rf_split_counts.get_mut(i) {
                                *slot += c;
                            }
                        }
                    }
                    _ => {}
                }
            }
            let m = metrics::regression_metrics(&preds);
            let b = evaluation::prediction_buckets(&preds);
            reports::write_model(&cfg.reports_dir, model, &m, &b, &preds)?;
            reports_written.push(format!("{model}_summary.json"));
            reports_written.push(format!("{model}_prediction_buckets.csv"));
            reports_written.push(format!("{model}_walk_forward.csv"));
            if model == "random_forest" {
                reports::write_random_forest_importance(
                    &cfg.reports_dir,
                    &cfg.enabled_features,
                    &rf_split_counts,
                )?;
                reports_written.push("random_forest_feature_importance.csv".to_string());
            }
            println!(
                "Model {model} complete | predictions={} | rmse={:.6} | corr={:.6} | elapsed={:.1}s",
                preds.len(),
                m.rmse,
                m.correlation,
                model_started.elapsed().as_secs_f64()
            );
            flush_stdout();
            eval_results.push(ModelEvaluationResult {
                model_name: model.clone(),
                metrics: m,
                buckets: b,
                prediction_count: preds.len(),
                window_count: windows.len(),
            });
        }
    }
    println!();
    println!("Reports Written");
    println!("---------------");
    reports_written.push("model_comparison.json".to_string());
    reports_written.push("forecast_run_manifest.json".to_string());
    for report in &reports_written {
        println!("  - {report}");
    }
    reports::write_comparison_and_manifest(&cfg.reports_dir, cfg, &eval_results, &reports_written)?;
    println!(
        "\nForecast research complete: wrote reports to {} | total_elapsed={:.1}s",
        cfg.reports_dir,
        run_started.elapsed().as_secs_f64()
    );
    flush_stdout();
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
    flush_stdout();
}

fn flush_stdout() {
    let _ = io::stdout().flush();
}

/// Formats a clean, actionable missing-data message matching the research
/// logging style: a symbol header, a numbered list of expected files, and
/// how-to-fix guidance. Never panics and never surfaces a raw loader error
/// when missing files can be detected up front.
fn format_missing_data(symbol: &str, missing: &[&PathBuf]) -> String {
    let header = format!("Symbol: {symbol}");
    let mut msg = String::new();
    msg.push_str(&header);
    msg.push('\n');
    msg.push_str(&"-".repeat(header.chars().count().max(1)));
    msg.push('\n');
    msg.push_str("Missing historical data.\n");
    msg.push_str("Expected files:\n");
    for (i, p) in missing.iter().enumerate() {
        msg.push_str(&format!("  {}. {}\n", i + 1, p.display()));
    }
    msg.push_str("How to fix:\n");
    msg.push_str("  - configure [historical_files], or\n");
    msg.push_str("  - place fallback CSV at data_dir/<SYMBOL>.csv\n");
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_data_formatter_lists_numbered_files_and_is_actionable() {
        let p1 = PathBuf::from("data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv");
        let p2 = PathBuf::from("data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv");
        let missing = vec![&p1, &p2];
        let msg = format_missing_data("BTCUSDT", &missing);
        assert!(msg.contains("Symbol: BTCUSDT"));
        assert!(msg.contains("Missing historical data."));
        assert!(msg.contains("Expected files:"));
        assert!(msg.contains("  1. data/historical/BTCUSDT/1m/BTCUSDT-1m-2020.csv"));
        assert!(msg.contains("  2. data/historical/BTCUSDT/1m/BTCUSDT-1m-2021.csv"));
        assert!(msg.contains("How to fix:"));
        assert!(msg.contains("configure [historical_files]"));
        assert!(msg.contains("place fallback CSV at data_dir/<SYMBOL>.csv"));
    }

    #[test]
    fn missing_data_formatter_header_dashes_match_header_length() {
        let p1 = PathBuf::from("x.csv");
        let missing = vec![&p1];
        let msg = format_missing_data("ETHUSDT", &missing);
        let header = "Symbol: ETHUSDT";
        let dashes = "-".repeat(header.chars().count());
        assert!(msg.contains(&format!("{header}\n{dashes}\n")));
    }
}
