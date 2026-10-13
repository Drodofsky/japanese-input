//! How well one reference stroke matches one drawn stroke; pure functions of the static context, and the raw ingredients every objective is built from.

use super::*;

pub fn shape_features(
    ctx: &StaticContext,
    leaf: usize,
    stroke: usize,
) -> Option<[f64; LEAF_FEATURE_COUNT]> {
    let reference = ctx.reference_shapes.get(leaf)?;
    let drawn = ctx.drawn_shapes.get(stroke)?;
    Some(reference.leaf_features(drawn))
}

pub fn feature_cost(
    ctx: &StaticContext,
    leaf: Option<usize>,
    stroke: Option<usize>,
    index: usize,
) -> f64 {
    let (Some(leaf), Some(stroke)) = (leaf, stroke) else {
        return INVALID_COST;
    };
    shape_features(ctx, leaf, stroke)
        .and_then(|features| features.get(index).copied())
        .unwrap_or(INVALID_COST)
}

pub fn shape_cost(ctx: &StaticContext, leaf: Option<usize>, stroke: Option<usize>) -> f64 {
    let (Some(leaf), Some(stroke)) = (leaf, stroke) else {
        return INVALID_COST;
    };
    shape_features(ctx, leaf, stroke).map_or(INVALID_COST, |features| features.iter().sum())
}

/// Lowest `ShapeCost` over every currently-open stroke without committing to any — how good this leaf's best remaining option is, or `INVALID_COST` if there is none.
pub fn best_match_for_leaf(
    ctx: &StaticContext,
    stroke_state: &[StrokeState],
    leaf: Option<usize>,
) -> f64 {
    let Some(leaf) = leaf else {
        return INVALID_COST;
    };
    stroke_state
        .iter()
        .enumerate()
        .filter(|(_, state)| **state == StrokeState::Open)
        .map(|(stroke, _)| shape_cost(ctx, Some(leaf), Some(stroke)))
        .fold(INVALID_COST, f64::min)
}

pub fn position_cost(ctx: &StaticContext, leaf: Option<usize>, stroke: Option<usize>) -> f64 {
    let (Some(leaf), Some(stroke)) = (leaf, stroke) else {
        return INVALID_COST;
    };
    let Some(reference) = ctx
        .reference_geometry
        .get(leaf)
        .and_then(|geometry| geometry.centroid)
    else {
        return INVALID_COST;
    };
    let Some(drawn) = ctx
        .drawn_geometry
        .get(stroke)
        .and_then(|geometry| geometry.centroid)
    else {
        return INVALID_COST;
    };
    let dx = reference.x - drawn.x;
    let dy = reference.y - drawn.y;
    dx.mul_add(dx, dy * dy)
}

pub fn reference_centroid_gap(
    ctx: &StaticContext,
    a: Option<usize>,
    b: Option<usize>,
) -> Option<(f64, f64)> {
    let left = ctx.reference_geometry.get(a?)?.centroid?;
    let right = ctx.reference_geometry.get(b?)?.centroid?;
    Some((left.x - right.x, left.y - right.y))
}

pub fn drawn_centroid_gap(
    ctx: &StaticContext,
    a: Option<usize>,
    b: Option<usize>,
) -> Option<(f64, f64)> {
    let left = ctx.drawn_geometry.get(a?)?.centroid?;
    let right = ctx.drawn_geometry.get(b?)?.centroid?;
    Some((left.x - right.x, left.y - right.y))
}

pub fn placement_cost(
    ctx: &StaticContext,
    leaf_a: Option<usize>,
    stroke_a: Option<usize>,
    leaf_b: Option<usize>,
    stroke_b: Option<usize>,
) -> f64 {
    let Some((expected_x, expected_y)) = reference_centroid_gap(ctx, leaf_a, leaf_b) else {
        return INVALID_COST;
    };
    let Some((drawn_x, drawn_y)) = drawn_centroid_gap(ctx, stroke_a, stroke_b) else {
        return INVALID_COST;
    };
    let gap_x = expected_x - drawn_x;
    let gap_y = expected_y - drawn_y;
    gap_x.mul_add(gap_x, gap_y * gap_y)
}

/// Log ratio of two lengths, guarding a near-zero length or divisor.
pub const LOG_RATIO_EPS: f64 = 1e-12;

pub fn log_ratio(left: f64, right: f64) -> Option<f64> {
    if left <= LOG_RATIO_EPS || right <= LOG_RATIO_EPS || !left.is_finite() || !right.is_finite() {
        return None;
    }
    Some((left / right).ln())
}

pub fn relative_length_cost(
    ctx: &StaticContext,
    leaf_a: Option<usize>,
    stroke_a: Option<usize>,
    leaf_b: Option<usize>,
    stroke_b: Option<usize>,
) -> f64 {
    let (Some(leaf_a), Some(stroke_a), Some(leaf_b), Some(stroke_b)) =
        (leaf_a, stroke_a, leaf_b, stroke_b)
    else {
        return INVALID_COST;
    };
    let (Some(reference_a), Some(reference_b), Some(drawn_a), Some(drawn_b)) = (
        ctx.reference_geometry.get(leaf_a),
        ctx.reference_geometry.get(leaf_b),
        ctx.drawn_geometry.get(stroke_a),
        ctx.drawn_geometry.get(stroke_b),
    ) else {
        return INVALID_COST;
    };
    let (Some(expected), Some(drawn)) = (
        log_ratio(reference_a.arc_len, reference_b.arc_len),
        log_ratio(drawn_a.arc_len, drawn_b.arc_len),
    ) else {
        return INVALID_COST;
    };
    (expected - drawn).abs()
}

pub fn merge_shape_cost(
    ctx: &StaticContext,
    start: Option<usize>,
    length: Option<usize>,
    stroke: Option<usize>,
) -> f64 {
    let (Some(start), Some(length), Some(stroke)) = (start, length, stroke) else {
        return INVALID_COST;
    };
    if !(2..=MAX_MERGE_LENGTH).contains(&length) {
        return INVALID_COST;
    }
    let Some(joined) = joined_reference_shape(&ctx.reference_leaves, start, length) else {
        return INVALID_COST;
    };
    let Some(drawn) = ctx.drawn_shapes.get(stroke) else {
        return INVALID_COST;
    };
    joined.leaf_features(drawn).iter().sum()
}

/// How many already-`Matched` leaves this pair would invert against, normalized by how many are committed; merge continuations are not counted.
pub fn order_cost(leaf_state: &[LeafState], leaf: Option<usize>, stroke: Option<usize>) -> f64 {
    let (Some(leaf), Some(stroke)) = (leaf, stroke) else {
        return INVALID_COST;
    };
    let mut inversions = 0_usize;
    let mut committed = 0_usize;
    for (other_leaf, state) in leaf_state.iter().enumerate() {
        if other_leaf == leaf {
            continue;
        }
        if let LeafState::Matched(other_stroke) = *state {
            committed = committed.saturating_add(1);
            let leaf_sign = (leaf as i64 - other_leaf as i64).signum();
            let stroke_sign = (stroke as i64 - other_stroke as i64).signum();
            if leaf_sign != 0 && stroke_sign != 0 && leaf_sign != stroke_sign {
                inversions = inversions.saturating_add(1);
            }
        }
    }
    if committed == 0 {
        0.0
    } else {
        inversions as f64 / committed as f64
    }
}
