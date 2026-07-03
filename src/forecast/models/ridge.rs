use crate::forecast::{dataset::ForecastRow, metrics::Prediction};
pub fn evaluate(
    train: &[ForecastRow],
    test: &[ForecastRow],
    alpha: f64,
    standardize: bool,
) -> Vec<Prediction> {
    if train.is_empty() {
        return vec![];
    }
    let nfeat = train[0].features.len();
    let mut mean = vec![0.0; nfeat];
    for r in train {
        for (j, v) in r.features.iter().enumerate() {
            mean[j] += v
        }
    }
    for m in &mut mean {
        *m /= train.len() as f64
    }
    let mut std = vec![1.0; nfeat];
    if standardize {
        for r in train {
            for (j, v) in r.features.iter().enumerate() {
                std[j] += (v - mean[j]).powi(2)
            }
        }
        for s in &mut std {
            *s = (*s / train.len() as f64).sqrt();
            if *s == 0.0 {
                *s = 1.0
            }
        }
    }
    let ymean = train
        .iter()
        .map(|r| r.future_return_after_cost_bps)
        .sum::<f64>()
        / train.len() as f64;
    let mut w = vec![0.0; nfeat];
    for j in 0..nfeat {
        let mut num = 0.0;
        let mut den = alpha.max(0.0);
        for r in train {
            let x = if standardize {
                (r.features[j] - mean[j]) / std[j]
            } else {
                r.features[j]
            };
            num += x * (r.future_return_after_cost_bps - ymean);
            den += x * x;
        }
        w[j] = if den > 0.0 { num / den } else { 0.0 };
    }
    test.iter()
        .map(|r| {
            let pred = ymean
                + w.iter()
                    .enumerate()
                    .map(|(j, wj)| {
                        let x = if standardize {
                            (r.features[j] - mean[j]) / std[j]
                        } else {
                            r.features[j]
                        };
                        wj * x
                    })
                    .sum::<f64>();
            Prediction {
                timestamp: r.timestamp,
                actual_bps: r.future_return_bps,
                actual_after_cost_bps: r.future_return_after_cost_bps,
                predicted_bps: pred,
            }
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn r(x: f64, y: f64) -> ForecastRow {
        ForecastRow {
            timestamp: 0,
            close: 1.0,
            features: vec![x],
            future_return_bps: y,
            future_return_after_cost_bps: y,
        }
    }
    #[test]
    fn predicts_test_only() {
        let p = evaluate(&[r(1.0, 1.0), r(2.0, 2.0)], &[r(3.0, 3.0)], 1.0, true);
        assert_eq!(p.len(), 1);
    }
}
