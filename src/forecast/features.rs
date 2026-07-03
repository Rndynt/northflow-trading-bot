use crate::{
    core::Candle,
    indicators::{Atr, Ema},
};

#[derive(Debug, Clone)]
pub struct FeatureRow {
    pub timestamp: i64,
    pub close: f64,
    pub values: Vec<f64>,
}
pub fn build_feature_rows(candles: &[Candle], names: &[String]) -> Vec<Option<FeatureRow>> {
    let mut out = Vec::with_capacity(candles.len());
    let mut atr = Atr::new(14).unwrap();
    let mut ema8 = Ema::new(8).unwrap();
    let mut ema21 = Ema::new(21).unwrap();
    let mut volumes: Vec<f64> = Vec::new();
    let mut cum_pv = 0.0;
    let mut cum_v = 0.0;
    for (i, c) in candles.iter().copied().enumerate() {
        let atr_v = atr.next(c).ok().flatten();
        let e8 = ema8.next(c.close).ok();
        let e21 = ema21.next(c.close).ok();
        volumes.push(c.volume);
        cum_pv += typical_price(c) * c.volume;
        cum_v += c.volume;
        let vwap = if cum_v > 0.0 {
            Some(cum_pv / cum_v)
        } else {
            None
        };
        let mut vals = Vec::new();
        let mut ok = c.is_valid() && c.close > 0.0;
        for n in names {
            let v = match n.as_str() {
                "return_1m" => ret(candles, i, 1),
                "return_5m" => ret(candles, i, 5),
                "return_15m" => ret(candles, i, 15),
                "atr_bps" => atr_v.map(|a| a / c.close * 10000.0),
                "volume_ratio" => vol_ratio(&volumes, 20),
                "vwap_distance_bps" => vwap.map(|v| (c.close - v) / v * 10000.0),
                "ema_8_21_spread_bps" => e8.zip(e21).map(|(a, b)| (a - b) / c.close * 10000.0),
                "range_position" => {
                    if c.high > c.low {
                        Some((c.close - c.low) / (c.high - c.low))
                    } else {
                        None
                    }
                }
                "hour_of_day" => Some(((c.timestamp / 3_600_000) % 24) as f64),
                "day_of_week" => Some((((c.timestamp / 86_400_000) + 4) % 7) as f64),
                _ => None,
            };
            if let Some(x) = v {
                if x.is_finite() {
                    vals.push(x)
                } else {
                    ok = false
                }
            } else {
                ok = false
            }
        }
        out.push(if ok {
            Some(FeatureRow {
                timestamp: c.timestamp,
                close: c.close,
                values: vals,
            })
        } else {
            None
        });
    }
    out
}
fn ret(c: &[Candle], i: usize, n: usize) -> Option<f64> {
    if i < n {
        None
    } else {
        Some((c[i].close / c[i - n].close - 1.0) * 10000.0)
    }
}
fn vol_ratio(v: &[f64], n: usize) -> Option<f64> {
    if v.len() < n {
        None
    } else {
        let s: f64 = v[v.len() - n..].iter().sum();
        let avg = s / n as f64;
        if avg > 0.0 {
            Some(*v.last().unwrap() / avg)
        } else {
            None
        }
    }
}
fn typical_price(c: Candle) -> f64 {
    (c.high + c.low + c.close) / 3.0
}
#[cfg(test)]
mod tests {
    use super::*;
    fn c(i: usize, close: f64) -> Candle {
        Candle {
            timestamp: i as i64 * 60000,
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 10.0,
        }
    }
    #[test]
    fn return_1m_uses_past_only() {
        let rows = build_feature_rows(
            &[c(0, 100.0), c(1, 110.0), c(2, 55.0)],
            &["return_1m".into()],
        );
        assert!((rows[1].as_ref().unwrap().values[0] - 1000.0).abs() < 1e-9);
    }
}
