//! Gain previews, whole-assignment relational totals, and the capsules that apply a change only when its gain is favourable.

use super::*;

/// Which pairwise cost the gain previews scan with, wrapped into range; `3` is each match's own shape fit rather than a relation between the pair.
pub fn pairwise_gain_operator(
    ctx: &StaticContext,
    operator: usize,
    leaf_a: usize,
    stroke_a: usize,
    leaf_b: usize,
    stroke_b: usize,
) -> f64 {
    let (leaf_a, stroke_a, leaf_b, stroke_b) =
        (Some(leaf_a), Some(stroke_a), Some(leaf_b), Some(stroke_b));
    match operator % 4 {
        0 => relative_order_cost(ctx, leaf_a, stroke_a, leaf_b, stroke_b),
        1 => placement_cost(ctx, leaf_a, stroke_a, leaf_b, stroke_b),
        2 => relative_length_cost(ctx, leaf_a, stroke_a, leaf_b, stroke_b),
        _ => shape_cost(ctx, leaf_a, stroke_a) + shape_cost(ctx, leaf_b, stroke_b),
    }
}

/// Sums `pairwise_gain_operator` over every pair of currently plainly-`Matched` leaves.
pub fn total_pairwise_cost(ctx: &StaticContext, leaf_state: &[LeafState], operator: usize) -> f64 {
    let matched: Vec<(usize, usize)> = leaf_state
        .iter()
        .enumerate()
        .filter_map(|(leaf, state)| {
            if let LeafState::Matched(stroke) = state {
                Some((leaf, *stroke))
            } else {
                None
            }
        })
        .collect();
    let mut total = 0.0;
    for i in 0..matched.len() {
        for j in (i + 1)..matched.len() {
            let (leaf_a, stroke_a) = matched[i];
            let (leaf_b, stroke_b) = matched[j];
            total += pairwise_gain_operator(ctx, operator, leaf_a, stroke_a, leaf_b, stroke_b);
        }
    }
    total
}

/// `total_pairwise_cost` restricted to same-group pairs: the within-component half, directly comparable to the full total.
pub fn total_group_pairwise_cost(
    ctx: &StaticContext,
    leaf_state: &[LeafState],
    operator: usize,
) -> f64 {
    let matched: Vec<(usize, usize)> = leaf_state
        .iter()
        .enumerate()
        .filter_map(|(leaf, state)| {
            if let LeafState::Matched(stroke) = state {
                Some((leaf, *stroke))
            } else {
                None
            }
        })
        .collect();
    let mut total = 0.0;
    for i in 0..matched.len() {
        for j in (i + 1)..matched.len() {
            let (leaf_a, stroke_a) = matched[i];
            let (leaf_b, stroke_b) = matched[j];
            if ctx.parent_id.get(leaf_a) != ctx.parent_id.get(leaf_b) {
                continue;
            }
            total += pairwise_gain_operator(ctx, operator, leaf_a, stroke_a, leaf_b, stroke_b);
        }
    }
    total
}

pub fn swap_gain(
    ctx: &StaticContext,
    leaf_state: &[LeafState],
    operator: usize,
    leaf_a: Option<usize>,
    leaf_b: Option<usize>,
) -> f64 {
    let (Some(leaf_a), Some(leaf_b)) = (leaf_a, leaf_b) else {
        return INVALID_COST;
    };
    if leaf_a == leaf_b
        || !matches!(leaf_state.get(leaf_a), Some(LeafState::Matched(_)))
        || !matches!(leaf_state.get(leaf_b), Some(LeafState::Matched(_)))
    {
        return INVALID_COST;
    }
    let before = total_pairwise_cost(ctx, leaf_state, operator);
    let mut simulated = leaf_state.to_vec();
    commit_swap_matches(&mut simulated, Some(leaf_a), Some(leaf_b));
    total_pairwise_cost(ctx, &simulated, operator) - before
}

/// Swaps two leaves' strokes only if `swap_gain` reports a real improvement; one primitive rather than a gain, a branch and an action.
pub fn commit_swap_if_better(
    ctx: &StaticContext,
    leaf_state: &mut [LeafState],
    operator: usize,
    leaf_a: Option<usize>,
    leaf_b: Option<usize>,
) {
    if swap_gain(ctx, leaf_state, operator, leaf_a, leaf_b) < 0.0 {
        commit_swap_matches(leaf_state, leaf_a, leaf_b);
    }
}

/// The two leaf ranges under `leaf_a` and `leaf_b`'s lowest common ancestor that `commit_swap_groups` exchanges, or `None` if they are not separable.
pub fn common_ancestor_ranges(
    node: &AnalyzedKanjiNode,
    start: usize,
    leaf_a: usize,
    leaf_b: usize,
) -> Option<((usize, usize), (usize, usize))> {
    let AnalyzedKanjiNode::Group { children, .. } = node else {
        return None;
    };
    let mut cursor = start;
    let mut ranges: Vec<(usize, usize, &AnalyzedKanjiNode)> = Vec::with_capacity(children.len());
    for child in children {
        let end = cursor.saturating_add(child.leaf_count());
        ranges.push((cursor, end, child));
        cursor = end;
    }
    let range_of = |leaf: usize| ranges.iter().find(|&&(s, e, _)| (s..e).contains(&leaf));
    let (&(a_start, a_end, a_node), &(b_start, b_end, _)) = (range_of(leaf_a)?, range_of(leaf_b)?);
    if (a_start, a_end) == (b_start, b_end) {
        return common_ancestor_ranges(a_node, a_start, leaf_a, leaf_b);
    }
    Some(((a_start, a_end), (b_start, b_end)))
}

/// Every leaf in `range` (a half-open `[start, end)` leaf-index span), in order.
pub fn range_members(range: (usize, usize)) -> Vec<usize> {
    (range.0..range.1).collect()
}

/// Exchanges two sibling groups' stroke assignments with array-splice semantics, so slots of different leaf counts are the ordinary case rather than an error.
pub fn commit_swap_groups(
    ctx: &StaticContext,
    leaf_state: &mut [LeafState],
    leaf_a: Option<usize>,
    leaf_b: Option<usize>,
) {
    let (Some(leaf_a), Some(leaf_b)) = (leaf_a, leaf_b) else {
        return;
    };
    if leaf_a == leaf_b {
        return;
    }
    let Some((range_x, range_y)) = common_ancestor_ranges(&ctx.tree, 0, leaf_a, leaf_b) else {
        return;
    };
    let (first, second) = if range_x.0 <= range_y.0 {
        (range_x, range_y)
    } else {
        (range_y, range_x)
    };
    let combined = (first.0, second.1);
    let members = range_members(combined);
    let Some(strokes) = matched_strokes_of_group(leaf_state, &members) else {
        return;
    };

    let first_len = first.1.saturating_sub(first.0);
    let second_start_in_combined = second.0.saturating_sub(combined.0);
    let second_len = second.1.saturating_sub(second.0);
    let (first_strokes, rest) = strokes.split_at(first_len);
    let (middle_strokes, second_strokes) =
        rest.split_at(second_start_in_combined.saturating_sub(first_len));
    debug_assert_eq!(second_strokes.len(), second_len);

    let reordered: Vec<usize> = second_strokes
        .iter()
        .chain(middle_strokes)
        .chain(first_strokes)
        .copied()
        .collect();
    for (&member, &stroke) in members.iter().zip(reordered.iter()) {
        if let Some(slot) = leaf_state.get_mut(member) {
            *slot = LeafState::Matched(stroke);
        }
    }
}

/// The span `commit_swap_groups` would act on, so the repaired capsule repairs exactly the leaves the splice touched.
pub fn swap_groups_span(
    ctx: &StaticContext,
    leaf_state: &[LeafState],
    leaf_a: usize,
    leaf_b: usize,
) -> Option<Vec<usize>> {
    if leaf_a == leaf_b {
        return None;
    }
    let (range_x, range_y) = common_ancestor_ranges(&ctx.tree, 0, leaf_a, leaf_b)?;
    let (first, second) = if range_x.0 <= range_y.0 {
        (range_x, range_y)
    } else {
        (range_y, range_x)
    };
    let members = range_members((first.0, second.1));
    matched_strokes_of_group(leaf_state, &members)?;
    Some(members)
}

/// What the repair hill-climbs on: summed per-leaf shape plus the within-group relational total; fixed here so the repair costs the caller nothing per pair.
pub fn repair_criterion(ctx: &StaticContext, leaf_state: &[LeafState]) -> f64 {
    total_match_cost(ctx, leaf_state) + total_group_pairwise_cost(ctx, leaf_state, 1)
}

/// A group exchange plus a hill-climb inside it; the two must happen together, since neither lowers any objective on its own.
pub fn commit_swap_groups_repaired(
    ctx: &StaticContext,
    leaf_state: &mut [LeafState],
    leaf_a: Option<usize>,
    leaf_b: Option<usize>,
) -> bool {
    let (Some(leaf_a), Some(leaf_b)) = (leaf_a, leaf_b) else {
        return false;
    };
    if swap_groups_span(ctx, leaf_state, leaf_a, leaf_b).is_none() {
        return false;
    }
    commit_swap_groups(ctx, leaf_state, Some(leaf_a), Some(leaf_b));

    let leaves = leaf_state.len();
    // Two passes, so a swap that only becomes worthwhile after an earlier one is still found.
    for _ in 0..2 {
        let mut improved = false;
        for first in 0..leaves {
            for second in (first + 1)..leaves {
                let before = repair_criterion(ctx, leaf_state);
                let snapshot = leaf_state.to_vec();
                commit_swap_matches(leaf_state, Some(first), Some(second));
                if repair_criterion(ctx, leaf_state) < before {
                    improved = true;
                } else {
                    leaf_state.copy_from_slice(&snapshot);
                }
            }
        }
        if !improved {
            break;
        }
    }
    true
}

/// Whether two pairs' relative order agrees between reference and drawn space, scaled by how discriminating the chosen axis is — scale-invariant, so it needs no outside weight.
pub fn relative_order_cost(
    ctx: &StaticContext,
    leaf_a: Option<usize>,
    stroke_a: Option<usize>,
    leaf_b: Option<usize>,
    stroke_b: Option<usize>,
) -> f64 {
    let Some((ref_dx, ref_dy)) = reference_centroid_gap(ctx, leaf_a, leaf_b) else {
        return INVALID_COST;
    };
    let Some((drawn_dx, drawn_dy)) = drawn_centroid_gap(ctx, stroke_a, stroke_b) else {
        return INVALID_COST;
    };
    let x_is_major = ref_dx.abs() >= ref_dy.abs();
    let (ref_major, ref_minor) = if x_is_major {
        (ref_dx, ref_dy)
    } else {
        (ref_dy, ref_dx)
    };
    let drawn_major = if x_is_major { drawn_dx } else { drawn_dy };
    if ref_major.abs() < 1e-9 || ref_major.signum() == drawn_major.signum() {
        return 0.0;
    }
    ((ref_major.abs() - ref_minor.abs()) / (ref_major.abs() + ref_minor.abs())).max(0.0)
}
