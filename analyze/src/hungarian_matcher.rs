use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;
use smallvec::SmallVec;

use crate::leaf_matrix::LeafMatrix;
use crate::match_node::{MatchInfo, StrokeVec};
const SCORE_SCALE: f32 = 1_000_000.0;

const EXTRA_PENALTY: f32 = 1.0;

#[must_use]
pub fn match_hungarian(leaf_matrix: &LeafMatrix) -> Vec<MatchInfo> {
    let n_user = leaf_matrix.n_user();
    let n_leaves = leaf_matrix.n_leaves();

    let n_cols = n_user + n_leaves;
    // i64::MAX/2 avoids overflow when Hungarian sums and subtracts.
    let blocked: i64 = i64::MAX / 4;

    let mut cost: Vec<i64> = Vec::with_capacity(usize::from(n_leaves) * usize::from(n_cols));
    for leaf in 0..n_leaves {
        // Real user stroke costs.
        for u in 0..n_user {
            let s = leaf_matrix.look_up(leaf, u);
            cost.push((s * SCORE_SCALE) as i64);
        }
        // MISSING columns: this leaf's own slot has the missing cost; others are blocked.
        for k in 0..n_leaves {
            if k == leaf {
                let s = leaf_matrix.look_up(leaf, n_user); // MISSING entry
                cost.push((s * SCORE_SCALE) as i64);
            } else {
                cost.push(blocked);
            }
        }
    }

    let matrix = Matrix::from_vec(n_leaves.into(), n_cols.into(), cost)
        .expect("cost matrix dimensions mismatch");

    let (total_cost_scaled, assignment) = kuhn_munkres_min(&matrix);

    // Decode assignment[leaf] = column.
    // - If column < n_user: user stroke `column` was picked.
    // - If column >= n_user: MISSING.
    let mut user_strokes: StrokeVec = SmallVec::with_capacity(n_leaves.into());
    let mut used_mask: u32 = 0;
    let mut assigned_real_count = 0;

    for &col in &assignment {
        if col < n_user.into() {
            user_strokes.push(col.try_into().unwrap_or(u8::MAX));
            used_mask |= 1u32 << (col as u32);
            assigned_real_count += 1;
        } else {
            user_strokes.push(u8::MAX);
        }
    }

    // Hungarian's score (raw assignment sum), unscaled.
    let mut score = (total_cost_scaled as f32) / SCORE_SCALE;

    // Apply EXTRA_PENALTY for user strokes the kanji didn't claim.
    let unused = n_user.saturating_sub(assigned_real_count);
    score += EXTRA_PENALTY * f32::from(unused);

    vec![MatchInfo {
        user_strokes,
        score,
        used_mask,
    }]
}
