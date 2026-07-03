//! Strategy registry for research/backtest runs.
//!
//! This is the only active module that maps configured strategy identifiers to
//! concrete strategy implementations. The backtest engine receives a trait
//! object and stays focused on deterministic replay.

use crate::core::NorthflowError;
use crate::strategy::{LiquiditySweepReclaimV1, ScreenedVwapScalp, Strategy};

pub struct StrategyRuntime {
    pub strategy_id: String,
    pub strategy: Box<dyn Strategy>,
}

pub fn build_strategy_runtime(strategy_id: &str) -> Result<StrategyRuntime, NorthflowError> {
    let strategy: Box<dyn Strategy> = match strategy_id {
        "screened_vwap_scalp" | "screened_vwap_scalp_v2" => Box::new(ScreenedVwapScalp::default()),
        "liquidity_sweep_reclaim_v1" => Box::new(LiquiditySweepReclaimV1::default()),
        // Historical config/tests still reference these ids, but their source
        // files are not present in this checkout. Keep them mapped to the
        // baseline strategy until they are restored or removed in a dedicated
        // strategy cleanup, rather than making engine compilation depend on
        // absent modules.
        "ema_trend_pullback_v1"
        | "vwap_reclaim_short_v1"
        | "vwap_reclaim_short_v2"
        | "mean_revert_v1" => Box::new(ScreenedVwapScalp::default()),
        other => {
            return Err(NorthflowError::ConfigError(format!(
                "unknown strategy_id: '{other}'"
            )));
        }
    };

    Ok(StrategyRuntime {
        strategy_id: strategy_id.to_string(),
        strategy,
    })
}
