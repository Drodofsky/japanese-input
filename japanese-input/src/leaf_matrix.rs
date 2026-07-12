use ordered_float::OrderedFloat;
use pathfinding::{
    kuhn_munkres::kuhn_munkres_min,
    matrix::{Matrix, MatrixFormatError},
};

use crate::stroke_point::StrokePoint;

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
#[non_exhaustive]
pub struct LeafMatrix {
    n_user: usize,
    n_ref: usize,
    missing_penalty: f64,
    costs: Vec<Cost>,
}
#[non_exhaustive]
pub struct MatchResult {
    pub score: f64,
    pub assignment: Vec<Option<usize>>,
}

impl LeafMatrix {
    #[must_use]
    #[inline]
    pub fn build<F>(
        user: &[Vec<StrokePoint>],
        reference: &[Vec<StrokePoint>],
        missing_penalty: f64,
        score_fn: F,
    ) -> Self
    where
        F: Fn(&[StrokePoint], &[StrokePoint]) -> f64,
    {
        let n_user = user.len();
        let n_ref = reference.len();
        let size = n_user.saturating_add(n_ref);
        let blocked = OrderedFloat(f64::INFINITY);
        let missing = OrderedFloat(missing_penalty);

        let cell = |row: usize, col: usize| -> Cost {
            match (user.get(row), reference.get(col)) {
                (Some(u), Some(r)) => OrderedFloat(score_fn(u, r)),
                (Some(_), None) => {
                    if col.checked_sub(n_ref) == Some(row) {
                        missing
                    } else {
                        blocked
                    }
                }
                (None, Some(_)) => {
                    if row.checked_sub(n_user) == Some(col) {
                        missing
                    } else {
                        blocked
                    }
                }
                (None, None) => OrderedFloat(0.0_f64),
            }
        };

        let costs = (0..size)
            .flat_map(|row| (0..size).map(move |col| cell(row, col)))
            .collect();

        Self {
            n_user,
            n_ref,
            missing_penalty,
            costs,
        }
    }

    /// # Errors
    /// Gibt [`LeafMatrixError`] zurück, falls die Kostenmatrix nicht
    /// konstruiert werden kann.
    #[inline]
    pub fn solve(&self) -> Result<MatchResult, LeafMatrixError> {
        let size = self.n_user.saturating_add(self.n_ref);
        if size == 0 {
            return Ok(MatchResult {
                score: 0.0,
                assignment: Vec::new(),
            });
        }

        let matrix = Matrix::from_vec(size, size, self.costs.clone())?;
        let (total, columns) = kuhn_munkres_min(&matrix);

        let mut assignment = vec![None; self.n_ref];
        for (user_idx, &col) in columns.iter().take(self.n_user).enumerate() {
            if let Some(slot) = assignment.get_mut(col) {
                *slot = Some(user_idx);
            }
        }

        Ok(MatchResult {
            score: total.into_inner(),
            assignment,
        })
    }
    /// # Errors
    /// Gibt [`LeafMatrixError`] zurück, falls die Kostenmatrix nicht
    /// konstruiert werden kann.
    #[inline]
    pub fn score(&self) -> Result<f64, LeafMatrixError> {
        let size = self.n_user.saturating_add(self.n_ref);
        if size == 0 {
            return Ok(0.0);
        }

        let matrix = Matrix::from_vec(size, size, self.costs.clone())?;
        let (total, _) = kuhn_munkres_min(&matrix);
        Ok(total.into_inner())
    }
    #[must_use]
    #[inline]
    pub fn cost(&self, reference: usize, user: usize) -> f64 {
        if user >= self.user_stroke_count() {
            return self.missing_penalty;
        }
        let size = self.user_stroke_count().saturating_add(self.n_ref);
        let idx = user.saturating_mul(size).saturating_add(reference);
        self.costs
            .get(idx)
            .map_or(self.missing_penalty, |c| c.into_inner())
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
