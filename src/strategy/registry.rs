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
                "unknown strategy_id: '{other}'. Available strategy: '{BASIC_SAMPLE_STRATEGY_ID}'"
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
        let runtime = build_strategy_runtime(BASIC_SAMPLE_STRATEGY_ID).unwrap();
        assert_eq!(runtime.strategy_id, BASIC_SAMPLE_STRATEGY_ID);
        assert_eq!(runtime.strategy.strategy_id(), BASIC_SAMPLE_STRATEGY_ID);
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
