//! The stroke matcher: a program that builds one assignment of drawn strokes to reference strokes, then revises it until its own objective stops improving.

use crate::analyzed_kanji_node::AnalyzedKanjiNode;
use crate::leaf_score::{LEAF_FEATURE_COUNT, LeafScore as _};
use crate::match_strokes::{MISSING, StrokeVec, joined_reference_shape};
use crate::shape::{HARMONICS, Shape, ToShape};
use crate::stroke_geometry::StrokeGeometry;
use crate::stroke_point::StrokePoint;

pub mod actions;
pub mod costs;
pub mod gains;
pub mod host;
pub mod program;
pub mod runtime;
pub mod snapshot;
pub mod totals;

// Re-exported for the submodules, which all `use super::*`; several are unused here and used heavily by siblings.
#[allow(unused_imports)]
use actions::{commit_match, commit_merge, commit_missing, commit_swap_matches, commit_uncommit, matched_strokes_of_group};
#[allow(unused_imports)]
use costs::{LOG_RATIO_EPS, best_match_for_leaf, drawn_centroid_gap, feature_cost, log_ratio, merge_shape_cost, order_cost, placement_cost, position_cost, reference_centroid_gap, relative_length_cost, shape_cost, shape_features};
#[allow(unused_imports)]
use gains::{commit_swap_groups, commit_swap_groups_repaired, commit_swap_if_better, common_ancestor_ranges, pairwise_gain_operator, range_members, relative_order_cost, repair_criterion, swap_gain, swap_groups_span, total_group_pairwise_cost, total_pairwise_cost};
#[allow(unused_imports)]
use snapshot::{Snapshot, keep_best_state, pop_state, push_state, restore_snapshot};
#[allow(unused_imports)]
use totals::{total_leaf_cost, total_match_cost};

/// Longest run of consecutive reference leaves one drawn stroke may stand for; eligibility never checks which group they belong to.
pub const MAX_MERGE_LENGTH: usize = 3;

/// How many assignment snapshots one run may hold, so pushing inside a loop cannot grow memory without limit.
pub const MAX_STATE_STACK_DEPTH: usize = 8;

/// Returned for an out-of-range index: large enough to lose every `<` comparison, without producing `NaN` or infinity.
pub const INVALID_COST: f64 = 1.0e6;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LeafState {
    Open,
    Matched(usize),
    /// Continues a merge run, holding the same stroke index as its start — already the public repeat notation, so nothing is converted on the way out.
    Filled(usize),
    Missing,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StrokeState {
    Open,
    Used,
    Extra,
}

/// Everything about one drawing that does not change while the program runs.
pub struct StaticContext {
    pub reference_leaves: Vec<Vec<StrokePoint>>,
    pub reference_shapes: Vec<Shape>,
    pub reference_geometry: Vec<StrokeGeometry>,
    pub drawn_shapes: Vec<Shape>,
    pub drawn_geometry: Vec<StrokeGeometry>,
    /// Per leaf index: an id shared by every leaf with the same direct parent group node.
    pub parent_id: Vec<usize>,
    /// Per leaf index: how many leaves share that same direct parent (including itself).
    pub group_size: Vec<usize>,
    pub leaf_count: usize,
    pub stroke_count: usize,
    /// Kept so a group swap can find two leaves' lowest common ancestor, which the flat one-level topology cannot answer.
    pub tree: AnalyzedKanjiNode,
}

#[must_use]
pub fn build_static_context(reference: &AnalyzedKanjiNode, drawn: &[Vec<StrokePoint>]) -> StaticContext {
    let reference_leaves = reference.collect_strokes();
    let reference_shapes: Vec<Shape> = reference_leaves.iter().map(ToShape::to_shape).collect();
    let reference_geometry: Vec<StrokeGeometry> = reference_leaves.iter().map(|leaf| StrokeGeometry::from_stroke(leaf)).collect();
    let drawn_shapes: Vec<Shape> = drawn.iter().map(ToShape::to_shape).collect();
    let drawn_geometry: Vec<StrokeGeometry> = drawn.iter().map(|stroke| StrokeGeometry::from_stroke(stroke)).collect();

    let leaf_count = reference_leaves.len();
    let stroke_count = drawn.len();
    let topology = group_topology(reference, leaf_count);

    StaticContext {
        reference_leaves,
        reference_shapes,
        reference_geometry,
        drawn_shapes,
        drawn_geometry,
        parent_id: topology.parent_id,
        group_size: topology.group_size,
        leaf_count,
        stroke_count,
        tree: reference.clone(),
    }
}

/// Reports any leaf the program never decided as `MISSING`, so the output is always a valid [`StrokeVec`].
#[must_use]
pub fn to_stroke_vec(leaf_state: &[LeafState]) -> StrokeVec {
    leaf_state
        .iter()
        .map(|state| match state {
            LeafState::Matched(stroke) | LeafState::Filled(stroke) => u8::try_from(*stroke).unwrap_or(MISSING),
            LeafState::Open | LeafState::Missing => MISSING,
        })
        .collect()
}

struct GroupTopology {
    parent_id: Vec<usize>,
    group_size: Vec<usize>,
}

/// Records each leaf's *direct* parent group id in `collect_strokes` order; an unwrapped root stroke gets a singleton group.
fn group_topology(root: &AnalyzedKanjiNode, leaf_count: usize) -> GroupTopology {
    let mut parent_id = vec![0_usize; leaf_count];
    let mut group_counts: Vec<usize> = Vec::new();
    let mut cursor = 0_usize;
    let mut next_id = 0_usize;
    walk_topology(root, None, &mut cursor, &mut next_id, &mut parent_id, &mut group_counts);
    let group_size = parent_id.iter().map(|&id| group_counts.get(id).copied().unwrap_or(1)).collect();
    GroupTopology { parent_id, group_size }
}

fn walk_topology(node: &AnalyzedKanjiNode, current_group: Option<usize>, cursor: &mut usize, next_id: &mut usize, parent_id: &mut [usize], group_counts: &mut Vec<usize>) {
    match node {
        AnalyzedKanjiNode::Stroke { .. } => {
            let group = current_group.unwrap_or_else(|| {
                let id = *next_id;
                *next_id = next_id.saturating_add(1);
                group_counts.push(0);
                id
            });
            if let Some(slot) = parent_id.get_mut(*cursor) {
                *slot = group;
            }
            if let Some(count) = group_counts.get_mut(group) {
                *count = count.saturating_add(1);
            }
            *cursor = cursor.saturating_add(1);
        }
        AnalyzedKanjiNode::Group { children, .. } => {
            let id = *next_id;
            *next_id = next_id.saturating_add(1);
            group_counts.push(0);
            for child in children {
                walk_topology(child, Some(id), cursor, next_id, parent_id, group_counts);
            }
        }
        // `AnalyzedKanjiNode` is `#[non_exhaustive]`; only the two variants above exist today.
        _ => {}
    }
}

#[must_use]
pub fn ratio(part: usize, whole: usize) -> f64 {
    if whole == 0 { 0.0 } else { part.min(whole) as f64 / whole as f64 }
}

/// Rounds to the nearest integer, rejecting negatives, `NaN` and infinities rather than wrapping them into a huge `usize`.
#[must_use]
pub fn to_index(value: f64) -> Option<usize> {
    if !value.is_finite() || value < -0.5 {
        return None;
    }
    Some(value.round() as usize)
}

#[must_use]
pub fn nth_open_leaf(leaf_state: &[LeafState], n: Option<usize>) -> Option<usize> {
    let n = n?;
    leaf_state.iter().enumerate().filter(|(_, state)| **state == LeafState::Open).nth(n).map(|(index, _)| index)
}

#[must_use]
pub fn nth_open_stroke(stroke_state: &[StrokeState], n: Option<usize>) -> Option<usize> {
    let n = n?;
    stroke_state.iter().enumerate().filter(|(_, state)| **state == StrokeState::Open).nth(n).map(|(index, _)| index)
}

/// Matches drawn strokes against a kanji tree, in the public repeat notation: a join repeats its stroke's index, an unexplained leaf is [`MISSING`].
#[must_use]
pub fn match_strokes_evolved(kanji_tree: &AnalyzedKanjiNode, user_strokes: &[Vec<StrokePoint>]) -> StrokeVec {
    host::run(kanji_tree, user_strokes)
}
