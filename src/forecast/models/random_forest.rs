use crate::forecast::{dataset::ForecastRow, metrics::Prediction};

#[derive(Debug, Clone)]
enum Node {
    Leaf(f64),
    Split {
        feature: usize,
        threshold: f64,
        left: Box<Node>,
        right: Box<Node>,
    },
}

pub fn evaluate(
    train: &[ForecastRow],
    test: &[ForecastRow],
    trees: usize,
    max_depth: usize,
    min_leaf: usize,
    feature_subsample_ratio: f64,
    target: fn(&ForecastRow) -> f64,
) -> Vec<Prediction> {
    if train.is_empty() || test.is_empty() || trees == 0 {
        return vec![];
    }
    let n_features = train[0].features.len();
    if n_features == 0 {
        return vec![];
    }
    let forest = (0..trees)
        .map(|tree_id| {
            let sample = deterministic_bootstrap(train.len(), tree_id);
            build_tree(
                train,
                &sample,
                max_depth.max(1),
                min_leaf.max(1),
                tree_id,
                n_features,
                feature_subsample_ratio,
                target,
            )
        })
        .collect::<Vec<_>>();

    test.iter()
        .map(|row| {
            let predicted_bps = forest
                .iter()
                .map(|tree| predict(tree, &row.features))
                .sum::<f64>()
                / forest.len() as f64;
            Prediction {
                timestamp: row.timestamp,
                actual_bps: row.future_return_bps,
                actual_after_cost_bps: row.future_return_after_cost_bps,
                effective_actual_bps: target(row),
                predicted_bps,
            }
        })
        .collect()
}

fn build_tree(
    rows: &[ForecastRow],
    idxs: &[usize],
    depth_left: usize,
    min_leaf: usize,
    tree_id: usize,
    n_features: usize,
    feature_subsample_ratio: f64,
    target: fn(&ForecastRow) -> f64,
) -> Node {
    let mean = mean_y(rows, idxs, target);
    if depth_left == 0
        || idxs.len() <= min_leaf * 2
        || variance_y(rows, idxs, mean, target) <= 1e-12
    {
        return Node::Leaf(mean);
    }

    let feature_count = ((n_features as f64 * feature_subsample_ratio).ceil() as usize)
        .max(1)
        .min(n_features);
    let mut best: Option<(usize, f64, f64)> = None;
    for offset in 0..feature_count {
        let feature = (tree_id + depth_left + offset * 7) % n_features;
        let threshold =
            idxs.iter().map(|&i| rows[i].features[feature]).sum::<f64>() / idxs.len() as f64;
        let (left, right): (Vec<_>, Vec<_>) = idxs
            .iter()
            .copied()
            .partition(|&i| rows[i].features[feature] <= threshold);
        if left.len() < min_leaf || right.len() < min_leaf {
            continue;
        }
        let lm = mean_y(rows, &left, target);
        let rm = mean_y(rows, &right, target);
        let loss = squared_error(rows, &left, lm, target) + squared_error(rows, &right, rm, target);
        if best.map(|(_, _, b_loss)| loss < b_loss).unwrap_or(true) {
            best = Some((feature, threshold, loss));
        }
    }

    let Some((feature, threshold, _)) = best else {
        return Node::Leaf(mean);
    };
    let (left, right): (Vec<_>, Vec<_>) = idxs
        .iter()
        .copied()
        .partition(|&i| rows[i].features[feature] <= threshold);
    Node::Split {
        feature,
        threshold,
        left: Box::new(build_tree(
            rows,
            &left,
            depth_left - 1,
            min_leaf,
            tree_id + 11,
            n_features,
            feature_subsample_ratio,
            target,
        )),
        right: Box::new(build_tree(
            rows,
            &right,
            depth_left - 1,
            min_leaf,
            tree_id + 17,
            n_features,
            feature_subsample_ratio,
            target,
        )),
    }
}

fn deterministic_bootstrap(len: usize, tree_id: usize) -> Vec<usize> {
    let mut state = 0x9E37_79B9_7F4A_7C15u64 ^ tree_id as u64;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state as usize) % len
        })
        .collect()
}
fn predict(node: &Node, features: &[f64]) -> f64 {
    match node {
        Node::Leaf(v) => *v,
        Node::Split {
            feature,
            threshold,
            left,
            right,
        } => {
            if features[*feature] <= *threshold {
                predict(left, features)
            } else {
                predict(right, features)
            }
        }
    }
}
fn mean_y(rows: &[ForecastRow], idxs: &[usize], target: fn(&ForecastRow) -> f64) -> f64 {
    idxs.iter().map(|&i| target(&rows[i])).sum::<f64>() / idxs.len() as f64
}
fn variance_y(
    rows: &[ForecastRow],
    idxs: &[usize],
    mean: f64,
    target: fn(&ForecastRow) -> f64,
) -> f64 {
    squared_error(rows, idxs, mean, target) / idxs.len() as f64
}
fn squared_error(
    rows: &[ForecastRow],
    idxs: &[usize],
    mean: f64,
    target: fn(&ForecastRow) -> f64,
) -> f64 {
    idxs.iter()
        .map(|&i| (target(&rows[i]) - mean).powi(2))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(x: f64, y: f64) -> ForecastRow {
        ForecastRow {
            timestamp: 0,
            close: 1.0,
            features: vec![x],
            future_return_bps: y,
            future_return_after_cost_bps: y,
        }
    }
    #[test]
    fn random_forest_predictions_are_deterministic() {
        let train = [
            row(1.0, 1.0),
            row(2.0, 2.0),
            row(10.0, 10.0),
            row(11.0, 11.0),
        ];
        let test = [row(9.0, 9.0)];
        let a = evaluate(&train, &test, 5, 3, 1, 0.5, |r| {
            r.future_return_after_cost_bps
        });
        let b = evaluate(&train, &test, 5, 3, 1, 0.5, |r| {
            r.future_return_after_cost_bps
        });
        assert_eq!(a[0].predicted_bps, b[0].predicted_bps);
    }
}
