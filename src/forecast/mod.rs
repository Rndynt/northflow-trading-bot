//! Independent forecast research module.

pub mod config;
pub mod dataset;
pub mod evaluation;
pub mod features;
pub mod labels;
pub mod metrics;
pub mod models;
pub mod reports;
pub mod runner;
pub mod split;

pub use config::ForecastConfig;
pub use runner::run_forecast;
