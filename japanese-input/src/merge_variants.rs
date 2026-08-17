//! Synthesizes merge training examples from a correctly labeled one, by concatenating
//! consecutive sibling strokes within a group the same way a user who never lifted their
//! pen would have drawn them. The ground truth for the result is derived, not guessed,
//! since we know exactly which real strokes were glued together.
//!
//! Only meaningful on a *clean* base example: the truth must already assign one real,
//! distinct, ascending user-stroke index to every reference leaf in each merged run (no
//! `MISSING` inside a run, no reordering). [`generate`] returns nothing for a run that
//! doesn't hold that shape rather than guess; callers should restrict this to the plain
//! correctly-drawn base cases, not already-perturbed error variants.

use kurbo::Vec2;

use crate::analyzed_kanji_node::AnalyzedKanjiNode;
use crate::match_strokes::{FILLER, MERGE_UPTO, MISSING, StrokeVec};
use crate::stroke_point::StrokePoint;

/// One synthesized merge variant: a name suffix naming which run(s) were joined, the
/// resulting drawn strokes, and the ground truth `match_strokes` should recover.
#[non_exhaustive]
pub struct MergeVariant {
    pub suffix: String,
    pub ink: Vec<Vec<StrokePoint>>,
    pub truth: StrokeVec,
}

/// Every group in the tree, paired with the reference position its first child's first
/// leaf occupies — the same cursor walk `accumulate_groups` uses to find a group's slice
/// of a `user_stroke_order`, generalized to yield the group itself rather than accumulate
/// a feature.
pub(crate) struct GroupSite<'tree> {
    pub(crate) start: usize,
    pub(crate) children: &'tree [AnalyzedKanjiNode],
}

pub(crate) fn walk_groups<'tree>(
    node: &'tree AnalyzedKanjiNode,
    start: usize,
    out: &mut Vec<GroupSite<'tree>>,
) {
    let AnalyzedKanjiNode::Group { children, .. } = node else {
        return;
    };
    out.push(GroupSite { start, children });
    let mut cursor = start;
    for child in children {
        walk_groups(child, cursor, out);
        cursor = cursor.saturating_add(child.leaf_count());
    }
}

/// Every `(reference_start, length)` run of consecutive plain-leaf children within one
/// group eligible to merge — mirrors the `strokes != length` guard in
/// `match_strokes::Solver::merges`, which refuses a run touching a sub-group.
fn eligible_runs(site: &GroupSite<'_>) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut cursor = site.start;
    let mut run: Vec<usize> = Vec::new();
    for child in site.children {
        if matches!(child, AnalyzedKanjiNode::Stroke { .. }) {
            run.push(cursor);
        } else {
            emit_runs(&run, &mut out);
            run.clear();
        }
        cursor = cursor.saturating_add(child.leaf_count());
    }
    emit_runs(&run, &mut out);
    out
}

fn emit_runs(positions: &[usize], out: &mut Vec<(usize, usize)>) {
    for length in 2..=MERGE_UPTO {
        if positions.len() < length {
            continue;
        }
        for window in positions.windows(length) {
            if let Some(&start) = window.first() {
                out.push((start, length));
            }
        }
    }
}

/// The cartesian product of one option list per group: every way to choose, for each
/// group independently, either to leave it alone (`None`) or merge one specific run.
fn cartesian(per_group: &[Vec<Option<(usize, usize)>>]) -> Vec<Vec<Option<(usize, usize)>>> {
    per_group.iter().fold(vec![Vec::new()], |acc, options| {
        acc.iter()
            .flat_map(|prefix| {
                options.iter().map(move |option| {
                    let mut next = prefix.clone();
                    next.push(*option);
                    next
                })
            })
            .collect()
    })
}

/// Concatenates consecutive drawn strokes into one, as if the pen never lifted between
/// them. Mirrors `match_strokes::Solver::joined_reference`'s seam handling: only the
/// first point of each absorbed stroke needs its displacement corrected to reflect the
/// jump from the previous stroke's last point, matching how a joined shape is already
/// built on the reference side.
pub(crate) fn join_ink(strokes: &[&Vec<StrokePoint>]) -> Vec<StrokePoint> {
    let mut out: Vec<StrokePoint> = Vec::new();
    for stroke in strokes {
        for (index, point) in stroke.iter().enumerate() {
            let mut copy = *point;
            if index == 0 {
                copy.displacement = match out.last() {
                    Some(previous) => Vec2::new(
                        point.position.x - previous.position.x,
                        point.position.y - previous.position.y,
                    ),
                    None => Vec2::ZERO,
                };
            }
            out.push(copy);
        }
    }
    out
}

/// The real, distinct, ascending user-stroke indices one run absorbs, read off the base
/// truth in reference order. `None` if the run doesn't hold that shape (a `MISSING`
/// inside it, or the indices aren't actually consecutive) — such a run cannot be merged
/// without guessing, so it is skipped rather than misrepresented.
fn absorbed_indices(truth: &StrokeVec, start: usize, length: usize) -> Option<Vec<usize>> {
    let mut members = Vec::with_capacity(length);
    for offset in 0..length {
        let value = *truth.get(start.saturating_add(offset))?;
        if value == MISSING || value == FILLER {
            return None;
        }
        members.push(usize::from(value));
    }
    let first = *members.first()?;
    members
        .iter()
        .enumerate()
        .all(|(offset, &value)| value == first.saturating_add(offset))
        .then_some(members)
}

/// Applies one combination of per-group choices to the base ink and truth, producing the
/// synthesized example, or `None` if any chosen run can't be safely merged (see
/// [`absorbed_indices`]).
fn apply(
    ink: &[Vec<StrokePoint>],
    truth: &StrokeVec,
    combo: &[Option<(usize, usize)>],
) -> Option<MergeVariant> {
    let runs: Vec<(usize, usize)> = combo.iter().filter_map(|choice| *choice).collect();
    if runs.is_empty() {
        return None;
    }

    let mut absorbed: Vec<Vec<usize>> = Vec::with_capacity(runs.len());
    for &(start, length) in &runs {
        absorbed.push(absorbed_indices(truth, start, length)?);
    }

    // Which run (if any) each original stroke belongs to, so a left-to-right pass over
    // the old ink knows when to emit one joined stroke instead of copying it through.
    let mut member_of: Vec<Option<usize>> = vec![None; ink.len()];
    for (run_id, members) in absorbed.iter().enumerate() {
        for &old_index in members {
            *member_of.get_mut(old_index)? = Some(run_id);
        }
    }

    let mut new_ink: Vec<Vec<StrokePoint>> = Vec::new();
    let mut old_to_new: Vec<Option<usize>> = vec![None; ink.len()];
    let mut old_index = 0_usize;
    while old_index < ink.len() {
        if let Some(run_id) = member_of.get(old_index).copied().flatten() {
            let members = absorbed.get(run_id)?;
            let strokes: Vec<&Vec<StrokePoint>> =
                members.iter().filter_map(|&m| ink.get(m)).collect();
            if strokes.len() != members.len() {
                return None;
            }
            new_ink.push(join_ink(&strokes));
            let new_index = new_ink.len().checked_sub(1)?;
            for &m in members {
                *old_to_new.get_mut(m)? = Some(new_index);
            }
            old_index = old_index.saturating_add(members.len());
        } else {
            new_ink.push(ink.get(old_index)?.clone());
            *old_to_new.get_mut(old_index)? = Some(new_ink.len().checked_sub(1)?);
            old_index = old_index.saturating_add(1);
        }
    }

    let mut new_truth = StrokeVec::new();
    for (position, &value) in truth.iter().enumerate() {
        let starts_a_run = runs.iter().any(|&(start, _)| start == position);
        let inside_a_run = runs
            .iter()
            .any(|&(start, length)| (start..start.saturating_add(length)).contains(&position));
        if inside_a_run && !starts_a_run {
            new_truth.push(FILLER);
            continue;
        }
        if value == MISSING {
            new_truth.push(MISSING);
            continue;
        }
        let mapped = (*old_to_new.get(usize::from(value))?)?;
        new_truth.push(u8::try_from(mapped).ok()?);
    }

    let suffix = runs
        .iter()
        .map(|&(start, length)| {
            format!("{start}-{}", start.saturating_add(length).saturating_sub(1))
        })
        .collect::<Vec<_>>()
        .join("+");
    Some(MergeVariant {
        suffix: format!("_m{suffix}"),
        ink: new_ink,
        truth: new_truth,
    })
}

/// Every combination of consecutive-sibling-stroke merges — within a group, up to length
/// 3, across every eligible group in the tree at once — synthesizable from one correctly
/// labeled base example.
#[must_use]
#[inline]
pub fn generate(
    tree: &AnalyzedKanjiNode,
    ink: &[Vec<StrokePoint>],
    truth: &StrokeVec,
) -> Vec<MergeVariant> {
    let mut sites = Vec::new();
    walk_groups(tree, 0, &mut sites);
    if sites.is_empty() {
        return Vec::new();
    }
    let per_group: Vec<Vec<Option<(usize, usize)>>> = sites
        .iter()
        .map(|site| {
            let mut options: Vec<Option<(usize, usize)>> = vec![None];
            options.extend(eligible_runs(site).into_iter().map(Some));
            options
        })
        .collect();
    cartesian(&per_group)
        .into_iter()
        .filter(|combo| combo.iter().any(Option::is_some))
        .filter_map(|combo| apply(ink, truth, &combo))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignment::AssignmentFeatures as _;
    use crate::match_strokes::match_strokes;
    use crate::shape::ToShapes as _;
    use crate::stroke_geometry::StrokeGeometry;
    use crate::stroke_point::to_stroke_points;
    use crate::weights::Weights;
    use kurbo::Point;
    use smallvec::smallvec;

    fn path(points: &[(f64, f64)]) -> Vec<StrokePoint> {
        to_stroke_points(points.iter().map(|&(x, y)| Point::new(x, y)))
    }

    fn stroke(index: u8, points: &[(f64, f64)]) -> AnalyzedKanjiNode {
        let path = path(points);
        let geometry = StrokeGeometry::from_stroke(&path);
        AnalyzedKanjiNode::Stroke {
            index,
            path,
            geometry,
        }
    }

    fn horizontal(y: f64) -> Vec<(f64, f64)> {
        vec![(0.2, y), (0.8, y)]
    }

    fn three() -> AnalyzedKanjiNode {
        AnalyzedKanjiNode::Group {
            element: '三',
            children: vec![
                stroke(0, &horizontal(0.2)),
                stroke(1, &horizontal(0.5)),
                stroke(2, &horizontal(0.8)),
            ],
        }
    }

    fn nested() -> AnalyzedKanjiNode {
        AnalyzedKanjiNode::Group {
            element: '語',
            children: vec![
                AnalyzedKanjiNode::Group {
                    element: '言',
                    children: vec![
                        stroke(0, &[(0.1, 0.1), (0.3, 0.1)]),
                        stroke(1, &[(0.1, 0.3), (0.3, 0.3)]),
                    ],
                },
                AnalyzedKanjiNode::Group {
                    element: '吾',
                    children: vec![
                        stroke(2, &[(0.7, 0.1), (0.9, 0.1)]),
                        stroke(3, &[(0.7, 0.3), (0.9, 0.3)]),
                    ],
                },
            ],
        }
    }

    /// A run of `count` sibling strokes plus one unrelated sibling, so a merge run never
    /// spans the whole group — exercises a run starting away from index 0.
    fn five() -> AnalyzedKanjiNode {
        AnalyzedKanjiNode::Group {
            element: '生',
            children: (0..5_u8)
                .map(|i| stroke(i, &horizontal(0.1 + f64::from(i) * 0.15)))
                .collect(),
        }
    }

    #[test]
    fn a_flat_three_stroke_group_offers_every_run_up_to_the_whole_group() {
        let tree = three();
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0, 1, 2];
        let variants = generate(&tree, &ink, &truth);
        let suffixes: Vec<&str> = variants.iter().map(|v| v.suffix.as_str()).collect();
        assert_eq!(variants.len(), 3, "{suffixes:?}");
        assert!(suffixes.contains(&"_m0-1"));
        assert!(suffixes.contains(&"_m1-2"));
        assert!(suffixes.contains(&"_m0-2"));
    }

    #[test]
    fn a_merged_variant_shrinks_the_stroke_count_by_what_it_absorbed() {
        let tree = three();
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0, 1, 2];
        let variant = generate(&tree, &ink, &truth)
            .into_iter()
            .find(|v| v.suffix == "_m0-1")
            .expect("a 2-run merge");
        assert_eq!(variant.ink.len(), 2, "three strokes minus one absorbed");
        assert_eq!(variant.truth.as_slice(), &[0, FILLER, 1]);
    }

    #[test]
    fn a_five_stroke_group_offers_runs_not_starting_at_zero() {
        let tree = five();
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0, 1, 2, 3, 4];
        let variants = generate(&tree, &ink, &truth);
        assert!(
            variants.iter().any(|v| v.suffix == "_m2-3"),
            "{:?}",
            variants.iter().map(|v| &v.suffix).collect::<Vec<_>>()
        );
        // length-2 runs: (0,1)(1,2)(2,3)(3,4) = 4; length-3 runs: (0,1,2)(1,2,3)(2,3,4) = 3.
        assert_eq!(variants.len(), 7);
    }

    #[test]
    fn a_nested_tree_also_generates_multi_group_combinations() {
        let tree = nested();
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0, 1, 2, 3];
        let variants = generate(&tree, &ink, &truth);
        // Each 2-stroke group offers exactly one run (its whole self); both groups
        // merged at once is the third, distinct combination.
        assert_eq!(
            variants.len(),
            3,
            "{:?}",
            variants.iter().map(|v| &v.suffix).collect::<Vec<_>>()
        );
        let both = variants
            .iter()
            .find(|v| v.ink.len() == 2)
            .expect("a combination merging both groups");
        assert_eq!(both.truth.as_slice(), &[0, FILLER, 1, FILLER]);
    }

    #[test]
    fn every_generated_variant_is_accepted_by_the_matcher() {
        for tree in [three(), five(), nested()] {
            let ink = tree.collect_strokes();
            let leaves = u8::try_from(tree.leaf_count()).expect("test trees stay under 256 leaves");
            let truth: StrokeVec = (0..leaves).collect();
            for variant in generate(&tree, &ink, &truth) {
                let strokes = tree.collect_strokes();
                let reference = strokes.to_shapes();
                let user = crate::length_scale::user_shapes(&strokes, &variant.ink);
                let geometry: Vec<StrokeGeometry> = variant
                    .ink
                    .iter()
                    .map(|s| StrokeGeometry::from_stroke(s))
                    .collect();
                assert!(
                    tree.assignment_features(
                        &variant.truth,
                        &reference,
                        &strokes,
                        &user,
                        &geometry,
                        &Weights::v1()
                    )
                    .is_some(),
                    "{}: {:?} rejected by a gate",
                    variant.suffix,
                    variant.truth
                );
                let results = match_strokes(tree.clone(), variant.ink.clone(), Weights::v1(), 64);
                assert!(
                    results
                        .iter()
                        .any(|result| result.user_stroke_order == variant.truth),
                    "{}: the matcher never offered {:?}",
                    variant.suffix,
                    variant.truth
                );
            }
        }
    }

    #[test]
    fn a_group_with_only_one_stroke_offers_nothing() {
        let tree = AnalyzedKanjiNode::Group {
            element: '一',
            children: vec![stroke(0, &horizontal(0.5))],
        };
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0];
        assert!(generate(&tree, &ink, &truth).is_empty());
    }
}
