//! Northflow CLI entry point.
//!
//! research  — deterministic backtest and strategy research mode.
//! paper     — DISABLED until research engine validated for paper.
//! live      — DISABLED until paper/live parity proven.

use northflow_trading_bot::{config::ResearchConfig, research::run_research};
use std::{env, process};

fn main() {
    if let Err(err) = real_main() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("help");
    match command {
        "research" => {
            let config_path =
                read_config_arg(&args).unwrap_or_else(|| "config/research.toml".to_string());
            let cfg = ResearchConfig::load(&config_path)?;
            run_research(&cfg)
        }
        "paper" => {
            Err("paper mode is disabled — research engine not yet validated for paper".to_string())
        }
        "live" => Err("live mode is disabled — paper/live parity not yet proven".to_string()),
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn read_config_arg(args: &[String]) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == "--config" || pair[0] == "-c")
        .map(|pair| pair[1].clone())
}

fn print_help() {
    println!("Northflow Crypto Trading Bot");
    println!();
    println!("Usage:");
    println!("  northflow research [--config config/research.toml]");
    println!("  northflow paper   # disabled — research engine not yet validated for paper");
    println!("  northflow live    # disabled — paper/live parity not yet proven");
    println!();
    println!("Research mode:");
    println!("  Runs deterministic historical backtests only.");
    println!("  Outputs simulated Trade records, reports, diagnostics, and attribution files.");
    println!("  No live orders, no paper trading, no exchange calls.");
    println!();
    println!("Historical data:");
    println!("  Configure [historical_files] in the preset, or place fallback CSV at data_dir/<SYMBOL>.csv.");
    println!("  Source data currently must be 1m OHLCV.");
    println!("  Columns: timestamp,open,high,low,close,volume");
    println!("  Alternative timestamp column: open_time");
}
