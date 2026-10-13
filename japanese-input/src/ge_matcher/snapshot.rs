//! The state stack: snapshot an assignment, restore it, or keep whichever scored better — what makes search possible rather than only greedy commit-and-repair.

use super::*;

#[derive(Clone)]
/// One complete assignment plus the score it was saved with; full copies, not diffs, since any number of changes may happen before the matching pop.
pub struct Snapshot {
    leaf_state: Vec<LeafState>,
    stroke_state: Vec<StrokeState>,
    score: f64,
}

/// Saves the current assignment and `score`; a no-op once the stack is full, so pushing too deep degrades rather than fails.
pub fn push_state(
    stack: &mut Vec<Snapshot>,
    leaf_state: &[LeafState],
    stroke_state: &[StrokeState],
    score: f64,
) {
    if stack.len() >= MAX_STATE_STACK_DEPTH {
        return;
    }
    stack.push(Snapshot {
        leaf_state: leaf_state.to_vec(),
        stroke_state: stroke_state.to_vec(),
        score,
    });
}

/// Writes a snapshot back over the live state; the length guard keeps a mismatched snapshot a no-op rather than a panic.
pub fn restore_snapshot(
    snapshot: &Snapshot,
    leaf_state: &mut [LeafState],
    stroke_state: &mut [StrokeState],
) {
    if snapshot.leaf_state.len() != leaf_state.len()
        || snapshot.stroke_state.len() != stroke_state.len()
    {
        return;
    }
    leaf_state.copy_from_slice(&snapshot.leaf_state);
    stroke_state.copy_from_slice(&snapshot.stroke_state);
}

/// `PopState`: restore the most recent snapshot, abandoning whatever happened since.
pub fn pop_state(
    stack: &mut Vec<Snapshot>,
    leaf_state: &mut [LeafState],
    stroke_state: &mut [StrokeState],
) {
    if let Some(snapshot) = stack.pop() {
        restore_snapshot(&snapshot, leaf_state, stroke_state);
    }
}

/// Consumes the most recent snapshot, keeping whichever assignment scored lower; a tie keeps the current one, so equal retries do not thrash.
pub fn keep_best_state(
    stack: &mut Vec<Snapshot>,
    leaf_state: &mut [LeafState],
    stroke_state: &mut [StrokeState],
    score: f64,
) {
    let Some(snapshot) = stack.pop() else { return };
    if snapshot.score < score {
        restore_snapshot(&snapshot, leaf_state, stroke_state);
    }
}
