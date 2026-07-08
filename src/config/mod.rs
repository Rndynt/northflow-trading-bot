//! ResearchConfig — parsed from config/research.toml.
//!
//! Timeframe roles are explicit; never inferred from array order:
//!   entry_timeframe        = "1m"
//!   screening_timeframe    = "15m"
//!   confirmation_timeframe = "5m"

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use crate::core::{NorthflowError, Timeframe};
use crate::strategy::ids::{BASIC_SAMPLE_STRATEGY_ID, TREND_REGIME_STRATEGY_ID};

#[derive(Debug, Clone)]
pub struct ResearchConfig {
    pub symbols: Vec<String>,
    pub entry_timeframe: String,
    pub screening_timeframe: String,
    pub confirmation_timeframe: String,
    pub data_dir: String,
    pub source_timeframe: String,
    pub historical_files: HashMap<String, Vec<PathBuf>>,
    pub reports_dir: String,
    pub strategy_id: String,
    pub strategy_run_mode: String,
    pub strategies: Vec<String>,
    pub initial_equity: f64,
    pub risk_per_trade_pct: f64,
    pub max_open_positions: usize,
    pub max_leverage: f64,
    pub min_reward_risk: f64,
    pub max_daily_loss_pct: f64,
    pub max_drawdown_pct: f64,
    pub taker_fee_bps: f64,
    pub slippage_bps: f64,
    pub spread_bps: f64,
    pub market_impact_bps: f64,
    pub stop_slippage_bps: f64,
    pub conservative_intrabar: bool,
    pub max_bars_held: u32,
    pub min_confidence: u8,
    pub entry_geometry_mode: String,
    pub entry_lookback_bars: usize,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            symbols: vec!["BTCUSDT".to_string()],
            entry_timeframe: "1m".to_string(),
            screening_timeframe: "15m".to_string(),
            confirmation_timeframe: "5m".to_string(),
            data_dir: "data/historical".to_string(),
            source_timeframe: "1m".to_string(),
            historical_files: HashMap::new(),
            reports_dir: "reports".to_string(),
            strategy_id: BASIC_SAMPLE_STRATEGY_ID.to_string(),
            strategy_run_mode: "single".to_string(),
            strategies: vec![],
            initial_equity: 5000.0,
            risk_per_trade_pct: 0.25,
            max_open_positions: 1,
            max_leverage: 3.0,
            min_reward_risk: 1.5,
            max_daily_loss_pct: 1.5,
            max_drawdown_pct: 5.0,
            taker_fee_bps: 4.0,
            slippage_bps: 2.0,
            spread_bps: 1.0,
            market_impact_bps: 1.0,
            stop_slippage_bps: 5.0,
            conservative_intrabar: true,
            max_bars_held: 60,
            min_confidence: 65,
            entry_geometry_mode: "preserve_signal_levels".to_string(),
            entry_lookback_bars: 0,
        }
    }
}

impl ResearchConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let raw = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("failed to read config {}: {e}", path.as_ref().display()))?;
        let cfg = Self::try_parse(&raw)?;
        cfg.validate_runtime_config().map_err(|e| format!("{e}"))?;
        cfg.validate_strategy_config().map_err(|e| format!("{e}"))?;
        cfg.validate_strategy_runner_config()
            .map_err(|e| format!("{e}"))?;
        Ok(cfg)
    }

    pub fn try_parse(raw: &str) -> Result<Self, String> {
        let mut cfg = Self::default();
        let value: toml::Value = toml::from_str(raw).map_err(|e| format!("malformed TOML: {e}"))?;
        apply_toml_value(&mut cfg, &value);
        cfg.historical_files = parse_historical_files(&value);
        Ok(cfg)
    }

    pub fn parse(raw: &str) -> Self {
        Self::try_parse(raw).unwrap_or_default()
    }

    pub fn historical_paths_for(&self, symbol: &str) -> Vec<PathBuf> {
        match self.historical_files.get(symbol) {
            Some(paths) if !paths.is_empty() => paths.clone(),
            _ => vec![Path::new(&self.data_dir).join(format!("{symbol}.csv"))],
        }
    }

    pub fn cooldown_bars_for_strategy(&self, _strategy_id: &str) -> u64 {
        0
    }

    pub fn validate_strategy_config(&self) -> Result<(), NorthflowError> {
        if crate::strategy::registry::build_strategy_runtime(&self.strategy_id).is_err() {
            return Err(NorthflowError::ConfigError(format!(
                "unknown strategy_id: '{}'. Available strategies: '{BASIC_SAMPLE_STRATEGY_ID}', '{TREND_REGIME_STRATEGY_ID}'",
                self.strategy_id
            )));
        }
        Ok(())
    }

    pub fn validate_strategy_runner_config(&self) -> Result<(), NorthflowError> {
        match self.strategy_run_mode.as_str() {
            "single" | "comparison" => {}
            "multi" => return Err(NorthflowError::ConfigError("multi-strategy portfolio backtest is not implemented yet; use strategy_run_mode = \"comparison\"".to_string())),
            other => return Err(NorthflowError::ConfigError(format!("unknown strategy_run_mode: '{other}'. Valid values: 'single', 'comparison', 'multi'"))),
        }
        let mut seen = HashSet::new();
        for strategy in &self.strategies {
            if crate::strategy::registry::build_strategy_runtime(strategy).is_err() {
                return Err(NorthflowError::ConfigError(format!(
                    "unknown strategy in strategies list: '{strategy}'. Available strategies: '{BASIC_SAMPLE_STRATEGY_ID}', '{TREND_REGIME_STRATEGY_ID}'"
                )));
            }
            if !seen.insert(strategy.as_str()) {
                return Err(NorthflowError::ConfigError(format!(
                    "duplicate strategy in strategies list: '{strategy}'"
                )));
            }
        }
        if self.strategy_run_mode == "single" && self.strategies.len() > 1 {
            return Err(NorthflowError::ConfigError(
                "strategy_run_mode = 'single' requires at most one strategy".to_string(),
            ));
        }
        if self.strategy_run_mode == "comparison" && self.strategies.is_empty() {
            return Err(NorthflowError::ConfigError("strategy_run_mode = 'comparison' requires at least one strategy in strategies list".to_string()));
        }
        Ok(())
    }

    pub fn selected_strategies(&self) -> Result<Vec<String>, NorthflowError> {
        match self.strategy_run_mode.as_str() {
            "single" => Ok(vec![self.strategies.first().cloned().unwrap_or_else(|| self.strategy_id.clone())]),
            "comparison" => Ok(self.strategies.clone()),
            "multi" => Err(NorthflowError::ConfigError("multi-strategy portfolio backtest is not implemented yet; use strategy_run_mode = \"comparison\"".to_string())),
            other => Err(NorthflowError::ConfigError(format!("unknown strategy_run_mode: '{other}'. Valid values: 'single', 'comparison', 'multi'"))),
        }
    }

    pub fn with_strategy_for_run(&self, strategy_id: &str, reports_dir: String) -> Self {
        let mut cfg = self.clone();
        cfg.strategy_id = strategy_id.to_string();
        cfg.reports_dir = reports_dir;
        cfg
    }

    pub fn risk_config(&self) -> crate::risk::RiskConfig {
        crate::risk::RiskConfig {
            risk_per_trade_pct: self.risk_per_trade_pct,
            max_open_positions: self.max_open_positions,
            max_leverage: self.max_leverage,
            min_reward_risk: self.min_reward_risk,
            max_daily_loss_pct: self.max_daily_loss_pct,
            max_drawdown_pct: self.max_drawdown_pct,
        }
    }

    pub fn cost_model_config(&self) -> crate::risk::CostModelConfig {
        crate::risk::CostModelConfig {
            taker_fee_bps: self.taker_fee_bps,
            slippage_bps: self.slippage_bps,
            spread_bps: self.spread_bps,
            market_impact_bps: self.market_impact_bps,
            stop_slippage_bps: self.stop_slippage_bps,
        }
    }

    pub fn validate_runtime_config(&self) -> Result<(), NorthflowError> {
        if self.max_open_positions != 1 {
            return Err(NorthflowError::ConfigError(
                "multi-position backtest is not implemented; set max_open_positions = 1"
                    .to_string(),
            ));
        }
        self.validate_timeframes()?;
        for (name, value) in [
            ("initial_equity_usd", self.initial_equity),
            ("risk_per_trade_pct", self.risk_per_trade_pct),
            ("max_leverage", self.max_leverage),
            ("min_reward_risk", self.min_reward_risk),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(NorthflowError::ConfigError(format!(
                    "{name} must be finite and positive"
                )));
            }
        }
        for (name, value) in [
            ("taker_fee_bps", self.taker_fee_bps),
            ("slippage_bps", self.slippage_bps),
            ("spread_bps", self.spread_bps),
            ("market_impact_bps", self.market_impact_bps),
            ("stop_slippage_bps", self.stop_slippage_bps),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(NorthflowError::ConfigError(format!(
                    "{name} must be finite and non-negative"
                )));
            }
        }
        if self.symbols.is_empty() || self.symbols.iter().any(|s| s.trim().is_empty()) {
            return Err(NorthflowError::ConfigError(
                "symbols must contain at least one non-empty symbol".to_string(),
            ));
        }
        if self.source_timeframe != "1m" {
            return Err(NorthflowError::ConfigError(
                "source_timeframe currently supports only '1m' because higher timeframes are built from 1m candles".to_string(),
            ));
        }
        if self.min_confidence > 100 {
            return Err(NorthflowError::ConfigError(
                "min_confidence must be in 0..=100".to_string(),
            ));
        }
        if self.max_bars_held == 0 {
            return Err(NorthflowError::ConfigError(
                "max_bars_held must be > 0".to_string(),
            ));
        }
        crate::backtest::EntryGeometryMode::parse(&self.entry_geometry_mode)?;
        if self.reports_dir.trim().is_empty() {
            return Err(NorthflowError::ConfigError(
                "reports_dir must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_timeframes(&self) -> Result<(), NorthflowError> {
        let entry = Timeframe::from_str(&self.entry_timeframe)
            .map_err(|e| NorthflowError::ConfigError(format!("entry_timeframe invalid: {e}")))?;
        let confirmation = Timeframe::from_str(&self.confirmation_timeframe).map_err(|e| {
            NorthflowError::ConfigError(format!("confirmation_timeframe invalid: {e}"))
        })?;
        let screening = Timeframe::from_str(&self.screening_timeframe).map_err(|e| {
            NorthflowError::ConfigError(format!("screening_timeframe invalid: {e}"))
        })?;
        if entry == confirmation || entry == screening || confirmation == screening {
            return Err(NorthflowError::ConfigError(format!("all three timeframe roles must be distinct: entry={entry}, confirmation={confirmation}, screening={screening}")));
        }
        if entry >= confirmation {
            return Err(NorthflowError::ConfigError(format!("entry_timeframe ({entry}) must be shorter than confirmation_timeframe ({confirmation})")));
        }
        if confirmation >= screening {
            return Err(NorthflowError::ConfigError(format!("confirmation_timeframe ({confirmation}) must be shorter than screening_timeframe ({screening})")));
        }
        Ok(())
    }
}

fn apply_toml_value(cfg: &mut ResearchConfig, value: &toml::Value) {
    let get = |section: &str, key: &str| value.get(section).and_then(|s| s.get(key));
    if let Some(v) = get("pairs", "symbols").and_then(|v| v.as_array()) {
        cfg.symbols = v
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
    }
    if let Some(v) = get("data", "data_dir")
        .or_else(|| get("backtest", "data_dir"))
        .or_else(|| value.get("data_dir"))
        .and_then(|v| v.as_str())
    {
        cfg.data_dir = v.to_string();
    }
    if let Some(v) = get("backtest", "reports_dir")
        .or_else(|| get("reports", "reports_dir"))
        .or_else(|| value.get("reports_dir"))
        .and_then(|v| v.as_str())
    {
        cfg.reports_dir = v.to_string();
    }
    if let Some(v) = get("pairs", "entry_timeframe")
        .or_else(|| get("timeframes", "entry_timeframe"))
        .or_else(|| value.get("entry_timeframe"))
        .and_then(|v| v.as_str())
    {
        cfg.entry_timeframe = v.to_string();
    }
    if let Some(v) = get("pairs", "screening_timeframe")
        .or_else(|| get("timeframes", "screening_timeframe"))
        .or_else(|| value.get("screening_timeframe"))
        .and_then(|v| v.as_str())
    {
        cfg.screening_timeframe = v.to_string();
    }
    if let Some(v) = get("pairs", "confirmation_timeframe")
        .or_else(|| get("timeframes", "confirmation_timeframe"))
        .or_else(|| value.get("confirmation_timeframe"))
        .and_then(|v| v.as_str())
    {
        cfg.confirmation_timeframe = v.to_string();
    }
    if let Some(v) = get("data", "source_timeframe").and_then(|v| v.as_str()) {
        cfg.source_timeframe = v.to_string();
    }
    if let Some(v) = get("strategy", "strategy_id")
        .or_else(|| get("strategy", "active"))
        .and_then(|v| v.as_str())
    {
        cfg.strategy_id = v.to_string();
    }
    if let Some(v) = get("strategy", "strategy_run_mode")
        .or_else(|| get("backtest", "strategy_run_mode"))
        .and_then(|v| v.as_str())
    {
        cfg.strategy_run_mode = v.to_string();
    }
    if let Some(v) = get("strategy", "strategies")
        .or_else(|| get("backtest", "strategies"))
        .and_then(|v| v.as_array())
    {
        cfg.strategies = v
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
    }
    if let Some(v) = get("risk", "initial_equity_usd")
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
    {
        cfg.initial_equity = v;
    }
    if let Some(v) = get("risk", "risk_per_trade_pct").and_then(num) {
        cfg.risk_per_trade_pct = v;
    }
    if let Some(v) = get("risk", "max_open_positions").and_then(|v| v.as_integer()) {
        cfg.max_open_positions = v as usize;
    }
    if let Some(v) = get("risk", "max_leverage").and_then(num) {
        cfg.max_leverage = v;
    }
    if let Some(v) = get("risk", "min_reward_risk").and_then(num) {
        cfg.min_reward_risk = v;
    }
    if let Some(v) = get("risk", "max_daily_loss_pct").and_then(num) {
        cfg.max_daily_loss_pct = v;
    }
    if let Some(v) = get("risk", "max_drawdown_pct").and_then(num) {
        cfg.max_drawdown_pct = v;
    }
    if let Some(v) = get("cost", "taker_fee_bps").and_then(num) {
        cfg.taker_fee_bps = v;
    }
    if let Some(v) = get("cost", "slippage_bps").and_then(num) {
        cfg.slippage_bps = v;
    }
    if let Some(v) = get("cost", "spread_bps").and_then(num) {
        cfg.spread_bps = v;
    }
    if let Some(v) = get("cost", "market_impact_bps").and_then(num) {
        cfg.market_impact_bps = v;
    }
    if let Some(v) = get("cost", "stop_slippage_bps").and_then(num) {
        cfg.stop_slippage_bps = v;
    }
    if let Some(v) = get("backtest", "conservative_intrabar").and_then(|v| v.as_bool()) {
        cfg.conservative_intrabar = v;
    }
    if let Some(v) = get("backtest", "max_bars_held").and_then(|v| v.as_integer()) {
        cfg.max_bars_held = v as u32;
    }
    if let Some(v) = get("strategy", "min_confidence")
        .or_else(|| get("backtest", "min_confidence"))
        .and_then(|v| v.as_integer())
    {
        cfg.min_confidence = v as u8;
    }
    if let Some(v) = get("backtest", "entry_geometry_mode").and_then(|v| v.as_str()) {
        cfg.entry_geometry_mode = v.to_string();
    }
    if let Some(v) = get("backtest", "entry_lookback_bars").and_then(|v| v.as_integer()) {
        cfg.entry_lookback_bars = v as usize;
    }
}

fn num(value: &toml::Value) -> Option<f64> {
    value
        .as_float()
        .or_else(|| value.as_integer().map(|i| i as f64))
}

fn parse_historical_files(value: &toml::Value) -> HashMap<String, Vec<PathBuf>> {
    let mut files = HashMap::new();
    if let Some(table) = value.get("historical_files").and_then(|v| v.as_table()) {
        for (symbol, paths) in table {
            if let Some(arr) = paths.as_array() {
                files.insert(
                    symbol.clone(),
                    arr.iter()
                        .filter_map(|p| p.as_str().map(PathBuf::from))
                        .collect(),
                );
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_strategy_is_basic_sample() {
        assert_eq!(
            ResearchConfig::default().strategy_id,
            BASIC_SAMPLE_STRATEGY_ID
        );
    }

    #[test]
    fn parses_basic_sample_strategy_id() {
        let cfg =
            ResearchConfig::try_parse("[strategy]\nstrategy_id = \"basic_sample_strategy\"\n")
                .unwrap();
        assert_eq!(cfg.strategy_id, BASIC_SAMPLE_STRATEGY_ID);
        assert!(cfg.validate_strategy_config().is_ok());
    }

    #[test]
    fn rejects_old_strategy_ids() {
        for old in [
            concat!("screened_", "vwap_", "scalp"),
            concat!("screened_", "vwap_", "scalp_", "v2"),
            concat!("ema_", "trend_", "pullback_", "v1"),
            concat!("vwap_", "reclaim_", "short_", "v1"),
            concat!("vwap_", "reclaim_", "short_", "v2"),
            concat!("mean_", "revert_", "v1"),
            concat!("liquidity_", "sweep_", "reclaim_", "v1"),
        ] {
            let mut cfg = ResearchConfig::default();
            cfg.strategy_id = old.to_string();
            assert!(
                cfg.validate_strategy_config().is_err(),
                "{old} should be rejected"
            );
        }
    }

    #[test]
    fn selected_strategies_falls_back_to_strategy_id() {
        let cfg = ResearchConfig::default();
        assert_eq!(
            cfg.selected_strategies().unwrap(),
            vec![BASIC_SAMPLE_STRATEGY_ID]
        );
    }

    #[test]
    fn strategy_runner_rejects_old_ids_in_list() {
        let mut cfg = ResearchConfig::default();
        cfg.strategies = vec![concat!("screened_", "vwap_", "scalp").to_string()];
        assert!(cfg.validate_strategy_runner_config().is_err());
    }

    #[test]
    fn parses_current_preset_sections() {
        let raw = r#"
[pairs]
symbols = ["ETHUSDT"]
entry_timeframe = "5m"
confirmation_timeframe = "15m"
screening_timeframe = "1h"
[data]
source_timeframe = "1m"
data_dir = "custom/data"
[strategy]
strategy_id = "basic_sample_strategy"
min_confidence = 91
[backtest]
reports_dir = "custom/reports"
strategy_run_mode = "single"
strategies = ["basic_sample_strategy"]
"#;
        let cfg = ResearchConfig::try_parse(raw).unwrap();
        cfg.validate_runtime_config().unwrap();
        cfg.validate_strategy_config().unwrap();
        cfg.validate_strategy_runner_config().unwrap();
        assert_eq!(cfg.symbols, vec!["ETHUSDT"]);
        assert_eq!(cfg.entry_timeframe, "5m");
        assert_eq!(cfg.confirmation_timeframe, "15m");
        assert_eq!(cfg.screening_timeframe, "1h");
        assert_eq!(cfg.source_timeframe, "1m");
        assert_eq!(cfg.data_dir, "custom/data");
        assert_eq!(cfg.reports_dir, "custom/reports");
        assert_eq!(cfg.min_confidence, 91);
    }

    #[test]
    fn rejects_non_1m_source_timeframe() {
        let mut cfg = ResearchConfig::default();
        cfg.source_timeframe = "5m".to_string();
        let err = cfg.validate_runtime_config().unwrap_err().to_string();
        assert!(err.contains("source_timeframe currently supports only '1m'"));
    }

    #[test]
    fn rejects_invalid_min_confidence_and_symbols() {
        let mut cfg = ResearchConfig::default();
        cfg.min_confidence = 101;
        assert!(cfg.validate_runtime_config().is_err());
        let mut cfg = ResearchConfig::default();
        cfg.symbols.clear();
        assert!(cfg.validate_runtime_config().is_err());
    }

    #[test]
    fn rejects_duplicate_strategy_ids_and_multi_mode() {
        let mut cfg = ResearchConfig::default();
        cfg.strategies = vec![
            BASIC_SAMPLE_STRATEGY_ID.to_string(),
            BASIC_SAMPLE_STRATEGY_ID.to_string(),
        ];
        assert!(cfg.validate_strategy_runner_config().is_err());
        let mut cfg = ResearchConfig::default();
        cfg.strategy_run_mode = "multi".to_string();
        assert!(cfg
            .validate_strategy_runner_config()
            .unwrap_err()
            .to_string()
            .contains("not implemented"));
    }

    #[test]
    fn rejects_invalid_timeframe_roles() {
        let mut cfg = ResearchConfig::default();
        cfg.entry_timeframe = "5m".to_string();
        cfg.confirmation_timeframe = "5m".to_string();
        cfg.screening_timeframe = "1h".to_string();
        assert!(cfg.validate_timeframes().is_err());
        cfg.entry_timeframe = "15m".to_string();
        cfg.confirmation_timeframe = "5m".to_string();
        assert!(cfg.validate_timeframes().is_err());
    }

    #[test]
    fn malformed_toml_returns_error() {
        assert!(ResearchConfig::try_parse("[pairs\nsymbols = [").is_err());
    }
}
