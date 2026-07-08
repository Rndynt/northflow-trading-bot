//! Strategy registry for research/backtest runs.

use crate::core::NorthflowError;
use crate::strategy::{ids::BASIC_SAMPLE_STRATEGY_ID, BasicSampleStrategy, Strategy};

pub struct StrategyRuntime {
    pub strategy_id: String,
    pub strategy: Box<dyn Strategy>,
}

pub fn build_strategy_runtime(strategy_id: &str) -> Result<StrategyRuntime, NorthflowError> {
    let strategy: Box<dyn Strategy> = match strategy_id {
        BASIC_SAMPLE_STRATEGY_ID => Box::new(BasicSampleStrategy),
        other => {
            return Err(NorthflowError::ConfigError(format!(
                "unknown strategy_id: '{other}'. Available strategies: '{BASIC_SAMPLE_STRATEGY_ID}'"
            )));
        }
    };

    Ok(StrategyRuntime {
        strategy_id: strategy_id.to_string(),
        strategy,
    })
}
