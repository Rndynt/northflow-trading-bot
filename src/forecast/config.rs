use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::risk::CostModelConfig;

#[derive(Debug, Clone)]
pub struct RidgeConfig {
    pub alpha: f64,
    pub standardize: bool,
}
#[derive(Debug, Clone)]
pub struct RandomForestConfig {
    pub trees: usize,
    pub max_depth: usize,
    pub min_samples_leaf: usize,
    pub feature_subsample_ratio: f64,
}
#[derive(Debug, Clone)]
pub struct WalkForwardConfig {
    pub train_months: usize,
    pub test_months: usize,
    pub step_months: usize,
    pub embargo_bars: usize,
}
#[derive(Debug, Clone)]
pub struct ForecastConfig {
    pub symbols: Vec<String>,
    pub source_timeframe: String,
    pub entry_timeframe: String,
    pub forecast_horizon: String,
    pub data_dir: String,
    pub historical_files: HashMap<String, Vec<PathBuf>>,
    pub enabled_features: Vec<String>,
    pub label_target: String,
    pub horizon_bars: usize,
    pub cost_adjusted: bool,
    pub cost: CostModelConfig,
    pub enabled_models: Vec<String>,
    pub ridge: RidgeConfig,
    pub random_forest: RandomForestConfig,
    pub walk_forward: WalkForwardConfig,
    pub reports_dir: String,
}

const KNOWN_FEATURES: &[&str] = &[
    "return_1m",
    "return_5m",
    "return_15m",
    "atr_bps",
    "volume_ratio",
    "vwap_distance_bps",
    "ema_8_21_spread_bps",
    "range_position",
    "hour_of_day",
    "day_of_week",
];
const KNOWN_LABELS: &[&str] = &["future_return_bps", "future_return_after_cost_bps"];
const KNOWN_MODELS: &[&str] = &["ridge", "random_forest"];

impl Default for ForecastConfig {
    fn default() -> Self {
        Self {
            symbols: vec!["BTCUSDT".into()],
            source_timeframe: "1m".into(),
            entry_timeframe: "1m".into(),
            forecast_horizon: "15m".into(),
            data_dir: "data/historical".into(),
            historical_files: HashMap::new(),
            enabled_features: KNOWN_FEATURES.iter().map(|s| s.to_string()).collect(),
            label_target: "future_return_bps".into(),
            horizon_bars: 15,
            cost_adjusted: true,
            cost: CostModelConfig {
                taker_fee_bps: 4.0,
                slippage_bps: 2.0,
                spread_bps: 1.0,
                market_impact_bps: 1.0,
                stop_slippage_bps: 5.0,
            },
            enabled_models: vec!["ridge".into(), "random_forest".into()],
            ridge: RidgeConfig {
                alpha: 1.0,
                standardize: true,
            },
            random_forest: RandomForestConfig {
                trees: 100,
                max_depth: 8,
                min_samples_leaf: 50,
                feature_subsample_ratio: 0.7,
            },
            walk_forward: WalkForwardConfig {
                train_months: 12,
                test_months: 3,
                step_months: 3,
                embargo_bars: 15,
            },
            reports_dir: "reports/forecast/btcusdt_1m_h15".into(),
        }
    }
}
impl ForecastConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let raw = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("failed to read config {}: {e}", path.as_ref().display()))?;
        let cfg = Self::try_parse(&raw)?;
        cfg.validate()?;
        Ok(cfg)
    }
    pub fn try_parse(raw: &str) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(raw).map_err(|e| format!("malformed TOML: {e}"))?;
        let mut c = Self::default();
        if let Some(mode) = value.get("mode") {
            if let Some(r) = s(mode, "run_mode") {
                if r != "forecast" {
                    return Err("run_mode must be \"forecast\"".into());
                }
            }
        }
        if let Some(p) = value.get("pairs") {
            if let Some(v) = arr_s(p, "symbols") {
                c.symbols = v
            }
            if let Some(v) = s(p, "source_timeframe") {
                c.source_timeframe = v
            }
            if let Some(v) = s(p, "entry_timeframe") {
                c.entry_timeframe = v
            }
            if let Some(v) = s(p, "forecast_horizon") {
                c.forecast_horizon = v
            }
        }
        if let Some(d) = value.get("data") {
            if let Some(v) = s(d, "data_dir") {
                c.data_dir = v
            }
        }
        if let Some(h) = value.get("historical_files").and_then(|v| v.as_table()) {
            c.historical_files = h
                .iter()
                .filter_map(|(k, v)| {
                    v.as_array().map(|a| {
                        (
                            k.clone(),
                            a.iter()
                                .filter_map(|x| x.as_str().map(PathBuf::from))
                                .collect(),
                        )
                    })
                })
                .collect();
        }
        if let Some(f) = value.get("features") {
            if let Some(v) = arr_s(f, "enabled") {
                c.enabled_features = v
            }
        }
        if let Some(l) = value.get("label") {
            if let Some(v) = s(l, "target") {
                c.label_target = v
            }
            if let Some(v) = u(l, "horizon_bars") {
                c.horizon_bars = v
            }
            if let Some(v) = l.get("cost_adjusted").and_then(|x| x.as_bool()) {
                c.cost_adjusted = v
            }
        }
        if let Some(co) = value.get("cost") {
            for (name, slot) in [
                ("taker_fee_bps", &mut c.cost.taker_fee_bps),
                ("slippage_bps", &mut c.cost.slippage_bps),
                ("spread_bps", &mut c.cost.spread_bps),
                ("market_impact_bps", &mut c.cost.market_impact_bps),
                ("stop_slippage_bps", &mut c.cost.stop_slippage_bps),
            ] {
                if let Some(v) = f64v(co, name) {
                    *slot = v
                }
            }
        }
        if let Some(m) = value.get("models") {
            if let Some(v) = arr_s(m, "enabled") {
                c.enabled_models = v
            }
            if let Some(r) = m.get("ridge") {
                if let Some(v) = f64v(r, "alpha") {
                    c.ridge.alpha = v
                }
                if let Some(v) = r.get("standardize").and_then(|x| x.as_bool()) {
                    c.ridge.standardize = v
                }
            }
            if let Some(rf) = m.get("random_forest") {
                if let Some(v) = u(rf, "trees") {
                    c.random_forest.trees = v
                }
                if let Some(v) = u(rf, "max_depth") {
                    c.random_forest.max_depth = v
                }
                if let Some(v) = u(rf, "min_samples_leaf") {
                    c.random_forest.min_samples_leaf = v
                }
                if let Some(v) = f64v(rf, "feature_subsample_ratio") {
                    c.random_forest.feature_subsample_ratio = v
                }
            }
        }
        if let Some(w) = value.get("walk_forward") {
            if let Some(v) = u(w, "train_months") {
                c.walk_forward.train_months = v
            }
            if let Some(v) = u(w, "test_months") {
                c.walk_forward.test_months = v
            }
            if let Some(v) = u(w, "step_months") {
                c.walk_forward.step_months = v
            }
            if let Some(v) = u(w, "embargo_bars") {
                c.walk_forward.embargo_bars = v
            }
        }
        if let Some(r) = value.get("reports") {
            if let Some(v) = s(r, "reports_dir") {
                c.reports_dir = v
            }
        }
        Ok(c)
    }
    pub fn historical_paths_for(&self, symbol: &str) -> Vec<PathBuf> {
        self.historical_files
            .get(symbol)
            .filter(|v| !v.is_empty())
            .cloned()
            .unwrap_or_else(|| vec![Path::new(&self.data_dir).join(format!("{symbol}.csv"))])
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.symbols.is_empty() {
            return Err("symbols must be non-empty".into());
        }
        if self.source_timeframe != "1m" {
            return Err("source_timeframe must be \"1m\"".into());
        }
        if self.horizon_bars == 0 {
            return Err("horizon_bars must be > 0".into());
        }
        for f in &self.enabled_features {
            if !KNOWN_FEATURES.contains(&f.as_str()) {
                return Err(format!("unknown forecast feature: {f}"));
            }
        }
        if self.label_target == "future_direction_after_cost" {
            return Err("future_direction_after_cost requires a classification evaluator and is not supported yet".into());
        }
        if !KNOWN_LABELS.contains(&self.label_target.as_str()) {
            return Err(format!("unknown label target: {}", self.label_target));
        }
        if !(self.random_forest.feature_subsample_ratio > 0.0
            && self.random_forest.feature_subsample_ratio <= 1.0)
        {
            return Err(
                "models.random_forest.feature_subsample_ratio must be > 0.0 and <= 1.0".into(),
            );
        }
        for m in &self.enabled_models {
            if !KNOWN_MODELS.contains(&m.as_str()) {
                return Err(format!("unknown forecast model: {m}"));
            }
        }
        for (n, v) in [
            ("taker_fee_bps", self.cost.taker_fee_bps),
            ("slippage_bps", self.cost.slippage_bps),
            ("spread_bps", self.cost.spread_bps),
            ("market_impact_bps", self.cost.market_impact_bps),
            ("stop_slippage_bps", self.cost.stop_slippage_bps),
        ] {
            if !v.is_finite() || v < 0.0 {
                return Err(format!("{n} must be finite and >= 0"));
            }
        }
        if self.walk_forward.train_months == 0
            || self.walk_forward.test_months == 0
            || self.walk_forward.step_months == 0
        {
            return Err("walk-forward months must be positive".into());
        }
        if self.reports_dir.trim().is_empty() {
            return Err("reports_dir must be non-empty".into());
        }
        Ok(())
    }
    pub fn round_trip_cost_bps(&self) -> f64 {
        self.cost.taker_fee_bps * 2.0
            + self.cost.slippage_bps * 2.0
            + self.cost.spread_bps
            + self.cost.market_impact_bps
    }
}
fn s(v: &toml::Value, k: &str) -> Option<String> {
    v.get(k)?.as_str().map(str::to_string)
}
fn arr_s(v: &toml::Value, k: &str) -> Option<Vec<String>> {
    Some(
        v.get(k)?
            .as_array()?
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
    )
}
fn u(v: &toml::Value, k: &str) -> Option<usize> {
    v.get(k)?.as_integer().and_then(|x| usize::try_from(x).ok())
}
fn f64v(v: &toml::Value, k: &str) -> Option<f64> {
    v.get(k)?
        .as_float()
        .or_else(|| v.get(k)?.as_integer().map(|x| x as f64))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_config_parses() {
        let c = ForecastConfig::try_parse(
            "[mode]\nrun_mode='forecast'\n[pairs]\nsymbols=['ETHUSDT']\nsource_timeframe='1m'\n",
        )
        .unwrap();
        assert_eq!(c.symbols, vec!["ETHUSDT"]);
        assert!(c.validate().is_ok());
    }
    #[test]
    fn invalid_rf_ratio_rejected() {
        let mut c = ForecastConfig::default();
        c.random_forest.feature_subsample_ratio = 0.0;
        assert!(c
            .validate()
            .unwrap_err()
            .contains("feature_subsample_ratio"));
    }
    #[test]
    fn classification_target_rejected() {
        let mut c = ForecastConfig::default();
        c.label_target = "future_direction_after_cost".into();
        assert!(c
            .validate()
            .unwrap_err()
            .contains("classification evaluator"));
    }
    #[test]
    fn effective_target_uses_cost_flag() {
        let mut c = ForecastConfig::default();
        c.label_target = "future_return_bps".into();
        c.cost_adjusted = true;
        assert_eq!(c.effective_target_name(), "future_return_after_cost_bps");
        c.cost_adjusted = false;
        assert_eq!(c.effective_target_name(), "future_return_bps");
    }
    #[test]
    fn invalid_feature_rejected() {
        let mut c = ForecastConfig::default();
        c.enabled_features = vec!["future_leak".into()];
        assert!(c
            .validate()
            .unwrap_err()
            .contains("unknown forecast feature"));
    }
}

impl ForecastConfig {
    pub fn effective_target_name(&self) -> &'static str {
        if self.cost_adjusted || self.label_target == "future_return_after_cost_bps" {
            "future_return_after_cost_bps"
        } else {
            "future_return_bps"
        }
    }
}
