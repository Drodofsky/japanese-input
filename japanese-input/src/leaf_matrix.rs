use ordered_float::OrderedFloat;
use pathfinding::matrix::MatrixFormatError;

use crate::{
    convert_lossy::ConvertLossy as _, leaf_score::LeafScore as _, shape::Shape, weights::Weights,
};

type Cost = OrderedFloat<f64>;

#[derive(Debug)]
#[non_exhaustive]
pub enum LeafMatrixError {
    Format(MatrixFormatError),
}

impl From<MatrixFormatError> for LeafMatrixError {
    #[inline]
    fn from(value: MatrixFormatError) -> Self {
        LeafMatrixError::Format(value)
    }
}

/// Every user stroke scored against every reference stroke, padded for unmatched strokes.
#[non_exhaustive]
pub struct LeafMatrix {
    n_user: usize,
    n_ref: usize,
    missing_penalty: f64,
    blocked: f64,
    costs: Vec<Cost>,
}

/// The cheapest one-to-one matching, indexed by reference stroke.
#[non_exhaustive]
pub struct MatchResult {
    pub score: f64,
    pub assignment: Vec<Option<usize>>,
}

impl LeafMatrix {
    /// Scores every pair, then pads to a square matrix so unmatched strokes have a home.
    #[must_use]
    #[inline]
    pub fn build(user: &[Shape], reference: &[Shape], weights: &Weights) -> Self {
        let n_user = user.len();
        let n_ref = reference.len();
        let pairs = pair_costs(user, reference, weights);
        let blocked = blocked_cost(&pairs, n_user, n_ref, weights);
        let size = n_user.saturating_add(n_ref);
        let mut costs = Vec::with_capacity(size.saturating_mul(size));
        for row in 0..size {
            for col in 0..size {
                costs.push(OrderedFloat(cell(
                    &pairs, n_user, n_ref, weights, blocked, row, col,
                )));
            }
        }
        Self {
            n_user,
            n_ref,
            missing_penalty: weights.missing_penalty,
            blocked,
            costs,
        }
    }
    #[must_use]
    #[inline]
    pub fn cost(&self, reference: usize, user: usize) -> f64 {
        if user >= self.n_user {
            return self.missing_penalty;
        }
        if reference >= self.n_ref {
            return self.blocked;
        }
        let size = self.n_user.saturating_add(self.n_ref);
        let index = user.saturating_mul(size).saturating_add(reference);
        self.costs
            .get(index)
            .map_or(self.blocked, |cost| cost.into_inner())
    }

    #[must_use]
    #[inline]
    pub fn is_blocked(&self, reference: usize, user: usize) -> bool {
        user < self.n_user && self.cost(reference, user) >= self.blocked
    }

    #[must_use]
    #[inline]
    pub fn blocked_cost(&self) -> f64 {
        self.blocked
    }

    #[must_use]
    #[inline]
    pub fn user_stroke_count(&self) -> usize {
        self.n_user
    }

    #[must_use]
    #[inline]
    pub fn reference_stroke_count(&self) -> usize {
        self.n_ref
    }
}

fn pair_costs(user: &[Shape], reference: &[Shape], weights: &Weights) -> Vec<Option<f64>> {
    let mut pairs = Vec::with_capacity(user.len().saturating_mul(reference.len()));
    for drawn in user {
        for expected in reference {
            pairs.push(expected.leaf_cost(drawn, weights));
        }
    }
    pairs
}

fn blocked_cost(pairs: &[Option<f64>], n_user: usize, n_ref: usize, weights: &Weights) -> f64 {
    let real: f64 = pairs.iter().filter_map(|cost| *cost).sum();
    let missing = weights.missing_penalty.max(0.0) * n_ref.convert_lossy();
    let extra = weights.extra_penalty.max(0.0) * n_user.convert_lossy();
    let total = 1.0_f64 + real + missing + extra;
    if total.is_finite() {
        total
    } else {
        f64::MAX / 8.0
    }
}

fn cell(
    pairs: &[Option<f64>],
    n_user: usize,
    n_ref: usize,
    weights: &Weights,
    blocked: f64,
    row: usize,
    col: usize,
) -> f64 {
    match (row < n_user, col < n_ref) {
        (true, true) => row
            .checked_mul(n_ref)
            .and_then(|offset| offset.checked_add(col))
            .and_then(|index| pairs.get(index).copied().flatten())
            .unwrap_or(blocked),
        (true, false) => {
            if col.checked_sub(n_ref) == Some(row) {
                weights.extra_penalty
            } else {
                blocked
            }
        }
        (false, true) => {
            if row.checked_sub(n_user) == Some(col) {
                weights.missing_penalty
            } else {
                blocked
            }
        }
        (false, false) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        shape::ToShape as _,
        stroke_point::{StrokePoint, to_stroke_points},
    };
    use kurbo::Point;

    fn shape(points: &[(f64, f64)]) -> Shape {
        let stroke: Vec<StrokePoint> =
            to_stroke_points(points.iter().map(|&(x, y)| Point::new(x, y)));
        stroke.to_shape()
    }

    fn horizontal(y: f64) -> Shape {
        shape(&[(0.0, y), (1.0, y)])
    }

    fn vertical(x: f64) -> Shape {
        shape(&[(x, 0.0), (x, 1.0)])
    }

    #[test]
    fn the_blocked_cost_beats_every_real_alternative_and_stays_finite() {
        let weights = Weights::v1();
        let strokes = vec![horizontal(0.0), vertical(0.0)];
        let matrix = LeafMatrix::build(&strokes, &strokes, &weights);
        let worst = weights.missing_penalty * 2.0 + weights.extra_penalty * 2.0;
        assert!(matrix.blocked_cost() > worst, "{}", matrix.blocked_cost());
        assert!(matrix.blocked_cost().is_finite());
    }

    #[test]
    fn a_ghost_user_index_reports_the_missing_penalty() {
        let weights = Weights::v1();
        let matrix = LeafMatrix::build(&[horizontal(0.0)], &[horizontal(0.0)], &weights);
        assert!((matrix.cost(0, 1) - weights.missing_penalty).abs() < 1e-12);
        assert!((matrix.cost(0, 99) - weights.missing_penalty).abs() < 1e-12);
        assert!(!matrix.is_blocked(0, 1));
    }

    #[test]
    fn an_out_of_range_reference_reports_blocked() {
        let matrix = LeafMatrix::build(&[horizontal(0.0)], &[horizontal(0.0)], &Weights::v1());
        assert!((matrix.cost(9, 0) - matrix.blocked_cost()).abs() < 1e-12);
    }

    #[test]
    fn counts_are_reported_back() {
        let matrix = LeafMatrix::build(
            &[horizontal(0.0)],
            &[horizontal(0.0), vertical(0.0)],
            &Weights::v1(),
        );
        assert_eq!(matrix.user_stroke_count(), 1);
        assert_eq!(matrix.reference_stroke_count(), 2);
    }

    #[test]
    fn strokes_of_equal_shape_tie_regardless_of_where_they_sit() {
        let matrix = LeafMatrix::build(
            &[horizontal(0.0), horizontal(0.5)],
            &[horizontal(0.0)],
            &Weights::v1(),
        );
        let first = matrix.cost(0, 0);
        let second = matrix.cost(0, 1);
        assert!((first - second).abs() < 1e-12, "{first} vs {second}");
    }
}
