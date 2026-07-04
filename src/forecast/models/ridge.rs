use crate::forecast::{dataset::ForecastRow, metrics::Prediction};

pub fn evaluate(
    train: &[ForecastRow],
    test: &[ForecastRow],
    alpha: f64,
    standardize: bool,
    target: fn(&ForecastRow) -> f64,
) -> Vec<Prediction> {
    if train.is_empty() || test.is_empty() {
        return vec![];
    }
    let nfeat = train[0].features.len();
    let (mean, std) = standardization(train, nfeat, standardize);
    let cols = nfeat + 1;
    let mut a = vec![vec![0.0; cols]; cols];
    let mut b = vec![0.0; cols];
    for r in train {
        let x = design_row(r, &mean, &std, standardize);
        let y = target(r);
        for i in 0..cols {
            b[i] += x[i] * y;
            for j in 0..cols {
                a[i][j] += x[i] * x[j];
            }
        }
    }
    for (i, row) in a.iter_mut().enumerate().skip(1) {
        row[i] += alpha.max(0.0);
    }
    let w = solve(a, b).unwrap_or_else(|| vec![0.0; cols]);
    test.iter()
        .map(|r| {
            let x = design_row(r, &mean, &std, standardize);
            Prediction {
                timestamp: r.timestamp,
                actual_bps: r.future_return_bps,
                actual_after_cost_bps: r.future_return_after_cost_bps,
                effective_actual_bps: target(r),
                predicted_bps: dot(&w, &x),
            }
        })
        .collect()
}
fn standardization(train: &[ForecastRow], nfeat: usize, standardize: bool) -> (Vec<f64>, Vec<f64>) {
    let mut mean = vec![0.0; nfeat];
    for r in train {
        for (j, v) in r.features.iter().enumerate() {
            mean[j] += v;
        }
    }
    for m in &mut mean {
        *m /= train.len() as f64;
    }
    let mut std = vec![1.0; nfeat];
    if standardize {
        for r in train {
            for (j, v) in r.features.iter().enumerate() {
                std[j] += (v - mean[j]).powi(2);
            }
        }
        for s in &mut std {
            *s = (*s / train.len() as f64).sqrt();
            if *s <= 1e-12 {
                *s = 1.0;
            }
        }
    }
    (mean, std)
}
fn design_row(r: &ForecastRow, mean: &[f64], std: &[f64], standardize: bool) -> Vec<f64> {
    let mut x = Vec::with_capacity(r.features.len() + 1);
    x.push(1.0);
    for (j, v) in r.features.iter().enumerate() {
        x.push(if standardize {
            (v - mean[j]) / std[j]
        } else {
            *v
        });
    }
    x
}
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let div = a[col][col];
        for j in col..n {
            a[col][j] /= div;
        }
        b[col] /= div;
        for i in 0..n {
            if i != col {
                let f = a[i][col];
                for j in col..n {
                    a[i][j] -= f * a[col][j];
                }
                b[i] -= f * b[col];
            }
        }
    }
    Some(b)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn r(x: f64, z: f64, y: f64) -> ForecastRow {
        ForecastRow {
            timestamp: 0,
            close: 1.0,
            features: vec![x, z],
            future_return_bps: y,
            future_return_after_cost_bps: y,
        }
    }
    #[test]
    fn true_multivariate_ridge_learns_two_features() {
        let tr = [r(0., 0., 1.), r(1., 0., 3.), r(0., 1., 4.), r(1., 1., 6.)];
        let te = [r(2., 3., 14.)];
        let p = evaluate(&tr, &te, 0.0, false, |r| r.future_return_after_cost_bps);
        assert!((p[0].predicted_bps - 14.0).abs() < 1e-8);
    }
}
