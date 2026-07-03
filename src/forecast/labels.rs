use crate::core::Candle;
pub fn future_return_bps(candles: &[Candle], index: usize, horizon: usize) -> Option<f64> {
    let fut = candles.get(index + horizon)?;
    let cur = candles.get(index)?;
    if cur.close > 0.0 && fut.close.is_finite() {
        Some((fut.close / cur.close - 1.0) * 10000.0)
    } else {
        None
    }
}
pub fn future_return_after_cost_bps(
    candles: &[Candle],
    index: usize,
    horizon: usize,
    cost_bps: f64,
) -> Option<f64> {
    future_return_bps(candles, index, horizon).map(|r| r - cost_bps)
}
pub fn future_direction_after_cost(
    candles: &[Candle],
    index: usize,
    horizon: usize,
    cost_bps: f64,
) -> Option<i8> {
    future_return_after_cost_bps(candles, index, horizon, cost_bps).map(|r| {
        if r > 0.0 {
            1
        } else if r < 0.0 {
            -1
        } else {
            0
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn c(close: f64) -> Candle {
        Candle {
            timestamp: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        }
    }
    #[test]
    fn exact_horizon() {
        let cs = [c(100.0), c(120.0), c(150.0)];
        assert_eq!(future_return_bps(&cs, 0, 2).unwrap(), 5000.0);
    }
}
