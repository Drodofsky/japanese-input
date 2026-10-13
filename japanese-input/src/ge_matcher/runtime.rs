//! What the compiled program may call: exactly the methods it does call, so one it is missing fails to compile and rustc names it.

pub trait Runtime {
    /// Opaque snapshot handle — see [`Runtime::begin_trial`].
    type Trial;

    // Counts: always present, so `usize`.
    fn leaf_count(&self) -> usize;
    fn stroke_count(&self) -> usize;
    fn open_leaf_count(&self) -> usize;
    fn open_stroke_count(&self) -> usize;
    fn merged_leaf_count(&self) -> usize;

    // Identities: may be absent, so `Option<usize>`.
    fn nth_open_leaf(&self, n: Option<usize>) -> Option<usize>;
    fn nth_open_stroke(&self, n: Option<usize>) -> Option<usize>;
    fn matched_stroke_of(&self, leaf: Option<usize>) -> Option<usize>;

    // Predicates.
    fn is_leaf_open(&self, leaf: Option<usize>) -> bool;
    fn is_stroke_open(&self, stroke: Option<usize>) -> bool;

    // Per-pair costs.
    fn shape_cost(&self, leaf: Option<usize>, stroke: Option<usize>) -> f64;
    fn merge_shape_cost(&self, start: Option<usize>, length: Option<usize>, stroke: Option<usize>) -> f64;
    fn best_match_for_leaf(&self, leaf: Option<usize>) -> f64;

    // Whole-assignment totals.
    fn total_pairwise_cost(&self, operator: usize) -> f64;
    fn total_group_pairwise_cost(&self, operator: usize) -> f64;
    fn total_match_cost(&self) -> f64;
    fn total_leaf_cost(&self, operator: usize) -> f64;

    // Gain previews; no mutation.
    fn swap_gain(&self, operator: usize, leaf_a: Option<usize>, leaf_b: Option<usize>) -> f64;

    // Actions: each validates first and is a no-op when ineligible.
    fn commit_match(&mut self, leaf: Option<usize>, stroke: Option<usize>);
    fn commit_missing(&mut self, leaf: Option<usize>);
    fn commit_merge(&mut self, start: Option<usize>, length: Option<usize>, stroke: Option<usize>);
    fn uncommit(&mut self, leaf: Option<usize>);
    fn swap_matches(&mut self, leaf_a: Option<usize>, leaf_b: Option<usize>);
    fn swap_if_better(&mut self, operator: usize, leaf_a: Option<usize>, leaf_b: Option<usize>);

    // Search over complete assignments.
    fn push_state(&mut self, score: f64);
    fn pop_state(&mut self);
    fn keep_best_state(&mut self, score: f64);

    // Trial support: a capsule whose acceptance test is the program's own score needs save/try/revert here.
    fn begin_trial(&mut self) -> Self::Trial;
    fn rollback(&mut self, trial: Self::Trial);
    fn try_swap_groups_repaired(&mut self, leaf_a: Option<usize>, leaf_b: Option<usize>) -> bool;

}

/// Rounds to the nearest integer, rejecting anything that is not a plausible index.
#[must_use]
#[inline]
pub fn to_index(value: f64) -> Option<usize> {
    if !value.is_finite() || value < -0.5 {
        return None;
    }
    Some(value.round() as usize)
}
