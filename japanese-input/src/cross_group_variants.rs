//! Synthesizes cross-group training examples from a correctly labeled one, by concatenating
//! a group-ending leaf with the next sibling's opening leaf the same way a user who never
//! lifted their pen would have drawn them. Unlike [`crate::merge_variants`], the result is
//! never `FILLER` — `Solver::merges` only ever looks within one group's own children, so a
//! stroke spanning a group boundary can't be recognized as a merge at all. Both reference
//! leaves stay `MISSING` and the glued drawn stroke is left unassigned ("extra"), which is
//! the only outcome the matcher can actually reach for this input.
//!
//! Only meaningful on a *clean* base example, same restriction as `merge_variants`: the two
//! reference leaves either side of a boundary must already carry distinct, adjacent,
//! ascending user-stroke indices (no `MISSING`, no reordering) or the boundary is skipped
//! rather than guessed at.

use crate::analyzed_kanji_node::AnalyzedKanjiNode;
use crate::group_score::{MAX_BOUNDARY_GAP, boundary_gap};
use crate::match_strokes::{FILLER, MISSING, StrokeVec};
use crate::merge_variants::{GroupSite, join_ink, walk_groups};
use crate::stroke_point::StrokePoint;

/// One synthesized cross-group variant: a name suffix naming which boundary(ies) were
/// joined, the resulting drawn strokes, and the ground truth `match_strokes` should recover.
#[non_exhaustive]
pub struct CrossGroupVariant {
    pub suffix: String,
    pub ink: Vec<Vec<StrokePoint>>,
    pub truth: StrokeVec,
}

/// Every boundary between two of one group's own direct children, as the reference position
/// of the left child's last leaf paired with the right child's first leaf — the same
/// boundary `group_score::cross_group_bonus` scores, found once here instead of rescored
/// live for every candidate.
fn boundaries(site: &GroupSite<'_>) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut cursor = site.start;
    let mut previous: Option<(usize, &AnalyzedKanjiNode)> = None;
    for child in site.children {
        let count = child.leaf_count();
        let first = cursor;
        let last = cursor.saturating_add(count).saturating_sub(1);
        if let Some((previous_last, previous_child)) = previous {
            // A boundary between two plain single-leaf siblings is already
            // `merge_variants`' territory (a real `FILLER` merge can represent it); this
            // generator only covers ground a merge can't reach.
            if previous_child.leaf_count() != 1 || child.leaf_count() != 1 {
                out.push((previous_last, first));
            }
        }
        previous = Some((last, child));
        cursor = cursor.saturating_add(count);
    }
    out
}

/// The two, real, adjacent, ascending user-stroke indices a boundary absorbs, read off the
/// base truth. `None` if the boundary doesn't hold that shape (a `MISSING`/`FILLER` on
/// either side, or the two strokes weren't actually drawn back to back) — such a boundary
/// cannot be glued without guessing, so it is skipped rather than misrepresented.
fn absorbed_pair(truth: &StrokeVec, left_pos: usize, right_pos: usize) -> Option<(usize, usize)> {
    let left = *truth.get(left_pos)?;
    let right = *truth.get(right_pos)?;
    if left == MISSING || left == FILLER || right == MISSING || right == FILLER {
        return None;
    }
    let left = usize::from(left);
    let right = usize::from(right);
    (right == left.saturating_add(1)).then_some((left, right))
}

/// The cartesian product of one yes/no choice per boundary: every way to choose, for each
/// boundary independently, either to leave it alone or glue it.
fn cartesian(count: usize) -> Vec<Vec<bool>> {
    (0..count).fold(vec![Vec::new()], |acc, _| {
        acc.iter()
            .flat_map(|prefix| {
                [false, true].into_iter().map(move |choice| {
                    let mut next = prefix.clone();
                    next.push(choice);
                    next
                })
            })
            .collect()
    })
}

/// Applies one combination of chosen boundaries to the base ink and truth, producing the
/// synthesized example, or `None` if any chosen boundary can't be safely glued (see
/// [`absorbed_pair`]) or two chosen boundaries would need to glue the same drawn stroke
/// twice (only possible when three or more singleton children sit in a row).
fn apply(
    ink: &[Vec<StrokePoint>],
    truth: &StrokeVec,
    all_boundaries: &[(usize, usize)],
    choices: &[bool],
) -> Option<CrossGroupVariant> {
    let chosen: Vec<(usize, usize)> = all_boundaries
        .iter()
        .zip(choices.iter())
        .filter_map(|(&boundary, &pick)| pick.then_some(boundary))
        .collect();
    if chosen.is_empty() {
        return None;
    }

    let mut pairs: Vec<(usize, usize)> = Vec::with_capacity(chosen.len());
    for &(left_pos, right_pos) in &chosen {
        pairs.push(absorbed_pair(truth, left_pos, right_pos)?);
    }
    let mut consumed: Vec<usize> = pairs.iter().flat_map(|&(l, r)| [l, r]).collect();
    consumed.sort_unstable();
    if consumed.windows(2).any(|pair| pair[0] == pair[1]) {
        return None;
    }

    let mut member_of: Vec<Option<usize>> = vec![None; ink.len()];
    for (pair_id, &(left, right)) in pairs.iter().enumerate() {
        *member_of.get_mut(left)? = Some(pair_id);
        *member_of.get_mut(right)? = Some(pair_id);
    }

    let mut new_ink: Vec<Vec<StrokePoint>> = Vec::new();
    let mut old_index = 0_usize;
    while old_index < ink.len() {
        if let Some(pair_id) = member_of.get(old_index).copied().flatten() {
            let &(left, right) = pairs.get(pair_id)?;
            if old_index != left {
                // The right half of a pair is always its left half's very next index, so
                // the left half is always reached first in this left-to-right walk.
                return None;
            }
            let strokes = [ink.get(left)?, ink.get(right)?];
            new_ink.push(join_ink(&strokes));
            old_index = old_index.saturating_add(2);
        } else {
            new_ink.push(ink.get(old_index)?.clone());
            old_index = old_index.saturating_add(1);
        }
    }

    let mut old_to_new: Vec<Option<usize>> = vec![None; ink.len()];
    let mut new_index = 0_usize;
    let mut old_index = 0_usize;
    while old_index < ink.len() {
        if let Some(pair_id) = member_of.get(old_index).copied().flatten() {
            let &(left, right) = pairs.get(pair_id)?;
            *old_to_new.get_mut(left)? = Some(new_index);
            *old_to_new.get_mut(right)? = Some(new_index);
            old_index = old_index.saturating_add(2);
        } else {
            *old_to_new.get_mut(old_index)? = Some(new_index);
            old_index = old_index.saturating_add(1);
        }
        new_index = new_index.saturating_add(1);
    }

    let mut new_truth = StrokeVec::new();
    for (position, &value) in truth.iter().enumerate() {
        let at_a_boundary = chosen.iter().any(|&(l, r)| l == position || r == position);
        if at_a_boundary || value == MISSING {
            new_truth.push(MISSING);
            continue;
        }
        let mapped = (*old_to_new.get(usize::from(value))?)?;
        new_truth.push(u8::try_from(mapped).ok()?);
    }

    let suffix = chosen
        .iter()
        .map(|&(left, right)| format!("{left}-{right}"))
        .collect::<Vec<_>>()
        .join("+");
    Some(CrossGroupVariant {
        suffix: format!("_wc{suffix}"),
        ink: new_ink,
        truth: new_truth,
    })
}

/// Every combination of cross-group-boundary glues — across every eligible boundary in the
/// tree at once — synthesizable from one correctly labeled base example.
#[must_use]
#[inline]
pub fn generate(
    tree: &AnalyzedKanjiNode,
    ink: &[Vec<StrokePoint>],
    truth: &StrokeVec,
) -> Vec<CrossGroupVariant> {
    let mut sites = Vec::new();
    walk_groups(tree, 0, &mut sites);
    let reference_points = tree.collect_strokes();
    let all_boundaries: Vec<(usize, usize)> = sites
        .iter()
        .flat_map(boundaries)
        .filter(|&(left, right)| {
            boundary_gap(&reference_points, left, right).is_some_and(|gap| gap <= MAX_BOUNDARY_GAP)
        })
        .collect();
    if all_boundaries.is_empty() {
        return Vec::new();
    }
    cartesian(all_boundaries.len())
        .into_iter()
        .filter(|choices| choices.iter().any(|choice| *choice))
        .filter_map(|choices| apply(ink, truth, &all_boundaries, &choices))
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

    /// Loosely 百: a leaf directly beside a nested group, so the one boundary sits between a
    /// plain sibling and a sub-group's opening leaf rather than between two plain siblings.
    fn leaf_beside_nested_group() -> AnalyzedKanjiNode {
        AnalyzedKanjiNode::Group {
            element: '白',
            children: vec![
                stroke(0, &[(0.5, 0.2), (0.4, 0.4)]),
                AnalyzedKanjiNode::Group {
                    element: '日',
                    children: vec![
                        stroke(1, &[(0.3, 0.45), (0.3, 0.9)]),
                        stroke(2, &[(0.3, 0.45), (0.7, 0.45)]),
                    ],
                },
            ],
        }
    }

    /// Loosely 語: two sibling groups, each with their own internal structure irrelevant to
    /// the one boundary between them.
    fn two_sibling_groups() -> AnalyzedKanjiNode {
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
                        // Starts near where 言's last stroke ends, so the boundary is within
                        // `MAX_BOUNDARY_GAP` — a real writer plausibly connecting them.
                        stroke(2, &[(0.35, 0.32), (0.9, 0.1)]),
                        stroke(3, &[(0.7, 0.3), (0.9, 0.3)]),
                    ],
                },
            ],
        }
    }

    #[test]
    fn a_leaf_beside_a_nested_group_offers_one_boundary() {
        let tree = leaf_beside_nested_group();
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0, 1, 2];
        let variants = generate(&tree, &ink, &truth);
        assert_eq!(
            variants.len(),
            1,
            "{:?}",
            variants.iter().map(|v| &v.suffix).collect::<Vec<_>>()
        );
        let only = &variants[0];
        assert_eq!(only.suffix, "_wc0-1");
        assert_eq!(only.ink.len(), 2, "three strokes minus one absorbed");
        assert_eq!(only.truth.as_slice(), &[MISSING, MISSING, 1]);
    }

    /// Every boundary in a flat, single-group kanji sits between two plain leaf siblings, so
    /// it's already `merge_variants`' territory — this generator has nothing to add there.
    #[test]
    fn a_flat_group_offers_nothing_since_every_boundary_is_already_mergeable() {
        let tree = three();
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0, 1, 2];
        assert!(generate(&tree, &ink, &truth).is_empty());
    }

    #[test]
    fn two_sibling_groups_offer_exactly_the_boundary_between_them() {
        let tree = two_sibling_groups();
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0, 1, 2, 3];
        let variants = generate(&tree, &ink, &truth);
        assert_eq!(
            variants.len(),
            1,
            "{:?}",
            variants.iter().map(|v| &v.suffix).collect::<Vec<_>>()
        );
        assert_eq!(variants[0].suffix, "_wc1-2");
        assert_eq!(variants[0].truth.as_slice(), &[0, MISSING, MISSING, 2]);
    }

    #[test]
    fn a_flat_group_with_no_boundary_offers_nothing() {
        let tree = AnalyzedKanjiNode::Group {
            element: '一',
            children: vec![stroke(0, &horizontal(0.5))],
        };
        let ink = tree.collect_strokes();
        let truth: StrokeVec = smallvec![0];
        assert!(generate(&tree, &ink, &truth).is_empty());
    }

    #[test]
    fn every_generated_variant_is_accepted_by_the_matcher() {
        for tree in [three(), leaf_beside_nested_group(), two_sibling_groups()] {
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
                let results = match_strokes(tree.clone(), variant.ink.clone(), Weights::v1(), 400);
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
}
