//! All that connects the compiled program to real geometry: a leaf/stroke state array and a snapshot stack.

use super::*;
use super::runtime::Runtime;

pub struct Host<'a> {
    ctx: &'a StaticContext,
    leaf_state: Vec<LeafState>,
    stroke_state: Vec<StrokeState>,
    state_stack: Vec<Snapshot>,
}

impl<'a> Host<'a> {
    pub fn new(ctx: &'a StaticContext) -> Self {
        Self { leaf_state: vec![LeafState::Open; ctx.leaf_count], stroke_state: vec![StrokeState::Open; ctx.stroke_count], ctx, state_stack: Vec::new() }
    }
}

/// Runs the compiled program against one drawing and reads off the assignment.
pub fn run(reference: &AnalyzedKanjiNode, drawn: &[Vec<StrokePoint>]) -> StrokeVec {
    let ctx = build_static_context(reference, drawn);
    let mut host = Host::new(&ctx);
    super::program::construct(&mut host);
    to_stroke_vec(&host.leaf_state)
}

impl Runtime for Host<'_> {
    type Trial = (Vec<LeafState>, Vec<StrokeState>);

    // ---- counts -----------------------------------------------------------------
    fn leaf_count(&self) -> usize {
        self.ctx.leaf_count
    }
    fn stroke_count(&self) -> usize {
        self.ctx.stroke_count
    }
    fn open_leaf_count(&self) -> usize {
        self.leaf_state.iter().filter(|state| **state == LeafState::Open).count()
    }
    fn open_stroke_count(&self) -> usize {
        self.stroke_state.iter().filter(|state| **state == StrokeState::Open).count()
    }
    fn merged_leaf_count(&self) -> usize {
        self.leaf_state.iter().filter(|state| matches!(state, LeafState::Filled(_))).count()
    }

    // ---- identities -------------------------------------------------------------
    fn nth_open_leaf(&self, n: Option<usize>) -> Option<usize> {
        nth_open_leaf(&self.leaf_state, n)
    }
    fn nth_open_stroke(&self, n: Option<usize>) -> Option<usize> {
        nth_open_stroke(&self.stroke_state, n)
    }
    fn matched_stroke_of(&self, leaf: Option<usize>) -> Option<usize> {
        leaf.and_then(|leaf| self.leaf_state.get(leaf)).and_then(|state| match state {
            LeafState::Matched(stroke) | LeafState::Filled(stroke) => Some(*stroke),
            LeafState::Open | LeafState::Missing => None,
        })
    }

    // ---- predicates -------------------------------------------------------------
    fn is_leaf_open(&self, leaf: Option<usize>) -> bool {
        leaf.and_then(|leaf| self.leaf_state.get(leaf)) == Some(&LeafState::Open)
    }
    fn is_stroke_open(&self, stroke: Option<usize>) -> bool {
        stroke.and_then(|stroke| self.stroke_state.get(stroke)) == Some(&StrokeState::Open)
    }

    // Per-pair costs; feature order is direction, `HARMONICS` sideways, `HARMONICS` along-chord, then length.
    fn shape_cost(&self, leaf: Option<usize>, stroke: Option<usize>) -> f64 {
        shape_cost(self.ctx, leaf, stroke)
    }
    fn merge_shape_cost(&self, start: Option<usize>, length: Option<usize>, stroke: Option<usize>) -> f64 {
        merge_shape_cost(self.ctx, start, length, stroke)
    }
    fn best_match_for_leaf(&self, leaf: Option<usize>) -> f64 {
        best_match_for_leaf(self.ctx, &self.stroke_state, leaf)
    }

    // ---- whole-assignment totals ------------------------------------------------
    fn total_pairwise_cost(&self, operator: usize) -> f64 {
        total_pairwise_cost(self.ctx, &self.leaf_state, operator)
    }
    fn total_group_pairwise_cost(&self, operator: usize) -> f64 {
        total_group_pairwise_cost(self.ctx, &self.leaf_state, operator)
    }
    fn total_match_cost(&self) -> f64 {
        total_match_cost(self.ctx, &self.leaf_state)
    }
    fn total_leaf_cost(&self, operator: usize) -> f64 {
        total_leaf_cost(self.ctx, &self.leaf_state, operator)
    }

    // ---- gain previews ----------------------------------------------------------
    fn swap_gain(&self, operator: usize, leaf_a: Option<usize>, leaf_b: Option<usize>) -> f64 {
        swap_gain(self.ctx, &self.leaf_state, operator, leaf_a, leaf_b)
    }

    // ---- actions ----------------------------------------------------------------
    fn commit_match(&mut self, leaf: Option<usize>, stroke: Option<usize>) {
        commit_match(&mut self.leaf_state, &mut self.stroke_state, leaf, stroke);
    }
    fn commit_missing(&mut self, leaf: Option<usize>) {
        commit_missing(&mut self.leaf_state, leaf);
    }
    fn commit_merge(&mut self, start: Option<usize>, length: Option<usize>, stroke: Option<usize>) {
        commit_merge(&mut self.leaf_state, &mut self.stroke_state, start, length, stroke);
    }
    fn uncommit(&mut self, leaf: Option<usize>) {
        commit_uncommit(&mut self.leaf_state, &mut self.stroke_state, leaf);
    }
    fn swap_matches(&mut self, leaf_a: Option<usize>, leaf_b: Option<usize>) {
        commit_swap_matches(&mut self.leaf_state, leaf_a, leaf_b);
    }
    fn swap_if_better(&mut self, operator: usize, leaf_a: Option<usize>, leaf_b: Option<usize>) {
        commit_swap_if_better(self.ctx, &mut self.leaf_state, operator, leaf_a, leaf_b);
    }

    // ---- search -----------------------------------------------------------------
    fn push_state(&mut self, score: f64) {
        push_state(&mut self.state_stack, &self.leaf_state, &self.stroke_state, score);
    }
    fn pop_state(&mut self) {
        pop_state(&mut self.state_stack, &mut self.leaf_state, &mut self.stroke_state);
    }
    fn keep_best_state(&mut self, score: f64) {
        keep_best_state(&mut self.state_stack, &mut self.leaf_state, &mut self.stroke_state, score);
    }

    // ---- trial support ----------------------------------------------------------
    fn begin_trial(&mut self) -> Self::Trial {
        (self.leaf_state.clone(), self.stroke_state.clone())
    }
    fn rollback(&mut self, trial: Self::Trial) {
        let (leaves, strokes) = trial;
        self.leaf_state.copy_from_slice(&leaves);
        self.stroke_state.copy_from_slice(&strokes);
    }
    fn try_swap_groups_repaired(&mut self, leaf_a: Option<usize>, leaf_b: Option<usize>) -> bool {
        commit_swap_groups_repaired(self.ctx, &mut self.leaf_state, leaf_a, leaf_b)
    }
}
