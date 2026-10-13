//! The only operations that change an assignment; each is a no-op rather than an error on arguments that make no sense.

use super::*;

pub fn commit_match(leaf_state: &mut [LeafState], stroke_state: &mut [StrokeState], leaf: Option<usize>, stroke: Option<usize>) {
    let (Some(leaf), Some(stroke)) = (leaf, stroke) else { return };
    if leaf_state.get(leaf) != Some(&LeafState::Open) || stroke_state.get(stroke) != Some(&StrokeState::Open) {
        return;
    }
    if let Some(state) = leaf_state.get_mut(leaf) {
        *state = LeafState::Matched(stroke);
    }
    if let Some(state) = stroke_state.get_mut(stroke) {
        *state = StrokeState::Used;
    }
}

pub fn commit_missing(leaf_state: &mut [LeafState], leaf: Option<usize>) {
    let Some(leaf) = leaf else { return };
    if leaf_state.get(leaf) == Some(&LeafState::Open)
        && let Some(state) = leaf_state.get_mut(leaf)
    {
        *state = LeafState::Missing;
    }
}

pub fn commit_merge(leaf_state: &mut [LeafState], stroke_state: &mut [StrokeState], start: Option<usize>, length: Option<usize>, stroke: Option<usize>) {
    let (Some(start), Some(length), Some(stroke)) = (start, length, stroke) else { return };
    if !(2..=MAX_MERGE_LENGTH).contains(&length) {
        return;
    }
    let Some(end) = start.checked_add(length) else { return };
    let Some(window) = leaf_state.get(start..end) else { return };
    if !window.iter().all(|state| *state == LeafState::Open) || stroke_state.get(stroke) != Some(&StrokeState::Open) {
        return;
    }
    if let Some(state) = leaf_state.get_mut(start) {
        *state = LeafState::Matched(stroke);
    }
    for leaf in start.saturating_add(1)..end {
        if let Some(state) = leaf_state.get_mut(leaf) {
            *state = LeafState::Filled(stroke);
        }
    }
    if let Some(state) = stroke_state.get_mut(stroke) {
        *state = StrokeState::Used;
    }
}

/// Reopens `leaf` and its stroke; through a merge run's start leaf this tears down the whole run, and through its middle it does nothing.
pub fn commit_uncommit(leaf_state: &mut [LeafState], stroke_state: &mut [StrokeState], leaf: Option<usize>) {
    let Some(leaf) = leaf else { return };
    let Some(state) = leaf_state.get(leaf).copied() else { return };
    match state {
        LeafState::Matched(stroke) => {
            if let Some(slot) = leaf_state.get_mut(leaf) {
                *slot = LeafState::Open;
            }
            if let Some(slot) = stroke_state.get_mut(stroke) {
                *slot = StrokeState::Open;
            }
            let mut cursor = leaf.saturating_add(1);
            while let Some(LeafState::Filled(filled_stroke)) = leaf_state.get(cursor).copied() {
                if filled_stroke != stroke {
                    break;
                }
                if let Some(slot) = leaf_state.get_mut(cursor) {
                    *slot = LeafState::Open;
                }
                cursor = cursor.saturating_add(1);
            }
        }
        LeafState::Missing => {
            if let Some(slot) = leaf_state.get_mut(leaf) {
                *slot = LeafState::Open;
            }
        }
        LeafState::Open | LeafState::Filled(_) => {}
    }
}

/// Exchanges which stroke two plainly-`Matched` leaves point to; `stroke_state` is untouched, as both strokes stay `Used`.
pub fn commit_swap_matches(leaf_state: &mut [LeafState], leaf_a: Option<usize>, leaf_b: Option<usize>) {
    let (Some(leaf_a), Some(leaf_b)) = (leaf_a, leaf_b) else { return };
    if leaf_a == leaf_b {
        return;
    }
    let (Some(LeafState::Matched(stroke_a)), Some(LeafState::Matched(stroke_b))) = (leaf_state.get(leaf_a).copied(), leaf_state.get(leaf_b).copied())
    else {
        return;
    };
    if let Some(slot) = leaf_state.get_mut(leaf_a) {
        *slot = LeafState::Matched(stroke_b);
    }
    if let Some(slot) = leaf_state.get_mut(leaf_b) {
        *slot = LeafState::Matched(stroke_a);
    }
}

/// One stroke per member in order, or `None` unless every member is plainly `Matched` — which scopes every group-bulk operation out of a half-assigned group.
pub fn matched_strokes_of_group(leaf_state: &[LeafState], members: &[usize]) -> Option<Vec<usize>> {
    members
        .iter()
        .map(|&leaf| match leaf_state.get(leaf) {
            Some(LeafState::Matched(stroke)) => Some(*stroke),
            _ => None,
        })
        .collect()
}

