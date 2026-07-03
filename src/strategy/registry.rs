//! Strategy registry for research/backtest runs.

use crate::core::NorthflowError;
use crate::strategy::{BasicSampleStrategy, Strategy};

pub struct StrategyRuntime {
    pub strategy_id: String,
    pub strategy: Box<dyn Strategy>,
}

pub fn build_strategy_runtime(strategy_id: &str) -> Result<StrategyRuntime, NorthflowError> {
    let strategy: Box<dyn Strategy> = match strategy_id {
        "basic_sample_strategy" => Box::new(BasicSampleStrategy),
        other => {
            return Err(NorthflowError::ConfigError(format!(
                "unknown strategy_id: '{other}'. Available strategy: 'basic_sample_strategy'"
            )));
        }
    };

    Ok(StrategyRuntime {
        strategy_id: strategy_id.to_string(),
        strategy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_sample_strategy_resolves() {
        let runtime = build_strategy_runtime("basic_sample_strategy").unwrap();
        assert_eq!(runtime.strategy_id, "basic_sample_strategy");
        assert_eq!(runtime.strategy.strategy_id(), "basic_sample_strategy");
    }

    #[test]
    fn old_strategy_ids_are_rejected() {
        for old in [
            concat!("screened_", "vwap_", "scalp"),
            concat!("screened_", "vwap_", "scalp_", "v2"),
            concat!("ema_", "trend_", "pullback_", "v1"),
            concat!("vwap_", "reclaim_", "short_", "v1"),
            concat!("vwap_", "reclaim_", "short_", "v2"),
            concat!("mean_", "revert_", "v1"),
            concat!("liquidity_", "sweep_", "reclaim_", "v1"),
        ] {
            assert!(
                build_strategy_runtime(old).is_err(),
                "{old} should be rejected"
            );
        }
    }
}
