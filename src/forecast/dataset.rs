use super::{config::ForecastConfig, features, labels};
use crate::core::Candle;
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub struct ForecastDataset {
    pub symbol: String,
    pub feature_names: Vec<String>,
    pub rows: Vec<ForecastRow>,
    pub summary: DatasetSummary,
}
#[derive(Debug, Clone)]
pub struct ForecastRow {
    pub timestamp: i64,
    pub close: f64,
    pub features: Vec<f64>,
    pub future_return_bps: f64,
    pub future_return_after_cost_bps: f64,
}
#[derive(Debug, Clone, Default)]
pub struct DatasetSummary {
    pub input_rows: usize,
    pub output_rows: usize,
    pub skipped_missing_feature: usize,
    pub skipped_invalid_feature: usize,
    pub skipped_label_horizon: usize,
    pub skipped_invalid_close: usize,
    pub skipped_invalid_label: usize,
}
pub fn build_dataset(symbol: &str, candles: &[Candle], cfg: &ForecastConfig) -> ForecastDataset {
    let feats = features::build_feature_rows(candles, &cfg.enabled_features);
    let mut s = DatasetSummary {
        input_rows: candles.len(),
        ..Default::default()
    };
    let mut rows = Vec::new();
    for i in 0..candles.len() {
        let c = candles[i];
        if !c.close.is_finite() || c.close <= 0.0 {
            s.skipped_invalid_close += 1;
            continue;
        }
        if i + cfg.horizon_bars >= candles.len() {
            s.skipped_label_horizon += 1;
            continue;
        }
        let Some(fr) = &feats[i] else {
            s.skipped_missing_feature += 1;
            continue;
        };
        if fr.values.iter().any(|v| !v.is_finite()) {
            s.skipped_invalid_feature += 1;
            continue;
        }
        let Some(ret) = labels::future_return_bps(candles, i, cfg.horizon_bars) else {
            s.skipped_invalid_label += 1;
            continue;
        };
        let adj = ret - cfg.round_trip_cost_bps();
        if !ret.is_finite() || !adj.is_finite() {
            s.skipped_invalid_label += 1;
            continue;
        }
        rows.push(ForecastRow {
            timestamp: fr.timestamp,
            close: fr.close,
            features: fr.values.clone(),
            future_return_bps: ret,
            future_return_after_cost_bps: adj,
        });
    }
    s.output_rows = rows.len();
    ForecastDataset {
        symbol: symbol.into(),
        feature_names: cfg.enabled_features.clone(),
        rows,
        summary: s,
    }
}
pub fn skip_reasons(s: &DatasetSummary) -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("missing_feature", s.skipped_missing_feature),
        ("invalid_feature", s.skipped_invalid_feature),
        ("label_horizon", s.skipped_label_horizon),
        ("invalid_close", s.skipped_invalid_close),
        ("invalid_label", s.skipped_invalid_label),
    ])
}
