//! Per-leaf assignment totals; these sum over `Matched` and `Filled`, unlike `gains`' relational totals which sum over `Matched` only.

use super::*;

/// Sums each matched or merge-filled leaf's own `ShapeCost` against the stroke it holds, with no relational term.
pub fn total_leaf_cost(ctx: &StaticContext, leaf_state: &[LeafState], operator: usize) -> f64 {
    leaf_state
        .iter()
        .enumerate()
        .filter_map(|(leaf, state)| match state {
            LeafState::Matched(stroke) | LeafState::Filled(stroke) => Some((leaf, *stroke)),
            LeafState::Open | LeafState::Missing => None,
        })
        .map(|(leaf, stroke)| match operator {
            0 => shape_cost(ctx, Some(leaf), Some(stroke)),
            1 => position_cost(ctx, Some(leaf), Some(stroke)),
            2 => order_cost(leaf_state, Some(leaf), Some(stroke)),
            3 => feature_cost(ctx, Some(leaf), Some(stroke), 0),
            _ => feature_cost(ctx, Some(leaf), Some(stroke), LEAF_FEATURE_COUNT - 1),
        })
        .sum()
}

pub fn total_match_cost(ctx: &StaticContext, leaf_state: &[LeafState]) -> f64 {
    leaf_state
        .iter()
        .enumerate()
        .filter_map(|(leaf, state)| match state {
            LeafState::Matched(stroke) | LeafState::Filled(stroke) => {
                Some(shape_cost(ctx, Some(leaf), Some(*stroke)))
            }
            LeafState::Open | LeafState::Missing => None,
        })
        .sum()
}
