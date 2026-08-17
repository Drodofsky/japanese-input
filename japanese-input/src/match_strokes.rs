use core::f64;
use core::mem::swap;
use smallvec::{SmallVec, smallvec};
use std::collections::HashMap;
use std::rc::Rc;

use kurbo::Vec2;

use crate::{
    analyzed_kanji_node::AnalyzedKanjiNode,
    group_score::GroupScore as _,
    leaf_score::LeafScore as _,
    shape::{Shape, ToShape as _, ToShapes as _},
    stroke_geometry::StrokeGeometry,
    stroke_point::StrokePoint,
    weights::Weights,
};

/// Marks a reference stroke joined into the drawn stroke that opened the run.
pub const FILLER: u8 = 254;

/// Marks a reference stroke the user never drew.
pub const MISSING: u8 = u8::MAX;

/// Child counts small enough to try every ordering, which is what a block swap needs.
const PERMUTE_UPTO: usize = 3;

/// Longest run of consecutive children a single drawn stroke may stand for.
pub(crate) const MERGE_UPTO: usize = 3;

/// How many drawn strokes beyond its own count a child may absorb.
///
/// A part of the kanji with three strokes will not have been drawn with ten, so letting a
/// child's slice grow without limit only costs time and offers readings nobody would make.
const SLACK: usize = 2;

pub type StrokeVec = SmallVec<[u8; 32]>;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub user_stroke_order: StrokeVec,
    pub score: f64,
    pub used_mask: u32,
    pub beam_width: usize,
}

/// Matches drawn strokes against a kanji tree by cutting the drawing into one consecutive
/// slice per group.
///
/// Because a group's slice is cut into one piece per child, two children can never claim
/// the same drawn stroke, so no candidate is ever thrown away for colliding with another.
/// A forgotten stroke is an empty slice, a spare stroke is a slice with one too many, and
/// a joined stroke is one slice standing for a run of children.
#[inline]
#[must_use]
pub fn match_strokes(
    kanji_tree: AnalyzedKanjiNode,
    user_strokes: Vec<Vec<StrokePoint>>,
    weights: Weights,
    beam_with: usize,
) -> Vec<MatchInfo> {
    let span = user_strokes.len();
    let mut solver = Solver::new(&kanji_tree, &user_strokes, weights, beam_with.max(1));
    let mut results = solver.solve(&kanji_tree, 0, 0, span, true);
    for result in &mut results {
        result.beam_width = beam_with;
    }
    results.sort_by(|a, b| a.score.total_cmp(&b.score));
    results
}

/// One assignment under construction.
#[derive(Clone)]
struct Candidate {
    score: f64,
    order: StrokeVec,
}

/// The children of one group, with where each child's strokes begin.
struct Layout<'array> {
    children: &'array [AnalyzedKanjiNode],
    counts: &'array [usize],
    offsets: &'array [usize],
}

struct Solver {
    reference: Vec<Shape>,
    user: Vec<Shape>,
    geometry: Vec<StrokeGeometry>,
    weights: Weights,
    beam: usize,
    memo: HashMap<(usize, usize, usize, usize), Rc<[Candidate]>>,
    joined: HashMap<(usize, usize), Option<Shape>>,
    points: Vec<Vec<StrokePoint>>,
}

impl Solver {
    fn new(
        tree: &AnalyzedKanjiNode,
        user_strokes: &[Vec<StrokePoint>],
        weights: Weights,
        beam: usize,
    ) -> Self {
        let points = tree.collect_strokes();
        Self {
            reference: points.to_shapes(),
            user: crate::length_scale::user_shapes(&points, user_strokes),
            geometry: user_strokes
                .iter()
                .map(|stroke| StrokeGeometry::from_stroke(stroke))
                .collect(),
            weights,
            beam,
            memo: HashMap::new(),
            joined: HashMap::new(),
            points,
        }
    }

    fn solve(
        &mut self,
        node: &AnalyzedKanjiNode,
        reference_offset: usize,
        start: usize,
        end: usize,
        root: bool,
    ) -> Vec<MatchInfo> {
        self.candidates(node, reference_offset, start, end, root)
            .iter()
            .map(|candidate| MatchInfo {
                used_mask: mask_of(&candidate.order),
                user_stroke_order: candidate.order.clone(),
                score: candidate.score,
                beam_width: self.beam,
            })
            .collect()
    }

    /// Best few assignments for one node over one slice, computed once and remembered.
    ///
    /// A node is identified by where its strokes begin together with how many it has,
    /// because a group and its first child share a starting offset.
    fn candidates(
        &mut self,
        node: &AnalyzedKanjiNode,
        reference_offset: usize,
        start: usize,
        end: usize,
        root: bool,
    ) -> Rc<[Candidate]> {
        let key = (reference_offset, node.leaf_count(), start, end);
        let cached = if root { None } else { self.memo.get(&key) };
        if let Some(hit) = cached {
            return Rc::clone(hit);
        }
        let out: Rc<[Candidate]> = Rc::from(match node {
            AnalyzedKanjiNode::Stroke { .. } => self.leaf(reference_offset, start, end),
            AnalyzedKanjiNode::Group { children, .. } => {
                self.group(node, children, reference_offset, start, end, root)
            }
        });
        if !root {
            self.memo.insert(key, Rc::clone(&out));
        }
        out
    }

    /// A stroke takes one drawn stroke from its slice; any others in it are spare.
    fn leaf(&self, reference_offset: usize, start: usize, end: usize) -> Vec<Candidate> {
        let span = end.saturating_sub(start);
        if span == 0 {
            return vec![Candidate {
                score: self.weights.missing_penalty,
                order: smallvec![MISSING],
            }];
        }
        let Some(reference) = self.reference.get(reference_offset) else {
            return Vec::new();
        };
        let spare = self.weights.extra_penalty * convert(span.saturating_sub(1));
        let mut out: Vec<Candidate> = (start..end)
            .filter_map(|chosen| {
                let drawn = self.user.get(chosen)?;
                let cost = reference.leaf_cost(drawn, &self.weights)?;
                Some(Candidate {
                    score: cost + spare,
                    order: smallvec![index_of(chosen)],
                })
            })
            .collect();
        out.sort_by(|a, b| a.score.total_cmp(&b.score));
        out.truncate(self.beam);
        out
    }

    /// Cuts the slice into one consecutive piece per child, in some order.
    fn group(
        &mut self,
        node: &AnalyzedKanjiNode,
        children: &[AnalyzedKanjiNode],
        reference_offset: usize,
        start: usize,
        end: usize,
        root: bool,
    ) -> Vec<Candidate> {
        let counts: Vec<usize> = children.iter().map(AnalyzedKanjiNode::leaf_count).collect();
        let mut offsets = Vec::with_capacity(counts.len());
        let mut run = reference_offset;
        for count in &counts {
            offsets.push(run);
            run = run.saturating_add(*count);
        }
        let layout = Layout {
            children,
            counts: &counts,
            offsets: &offsets,
        };
        let mut out: Vec<Candidate> = Vec::new();
        for order in orderings(children.len()) {
            for (score, taken) in self.splits(&layout, &order, start, end) {
                let flat = reorder(&counts, &order, &taken);
                let group = node.group_score(
                    &scoring_order(&flat),
                    &self.geometry,
                    &self.user,
                    &self.weights,
                    root,
                );
                out.push(Candidate {
                    score: score + group,
                    order: flat,
                });
            }
        }
        out.sort_by(|a, b| a.score.total_cmp(&b.score));
        out.truncate(self.beam);
        out
    }

    /// Sweeps left to right, keeping the best few partial cuts at each stopping point.
    ///
    /// The partial assignment is carried as the flat list it will become, so growing it is
    /// a copy of a small inline buffer rather than of a list of per child records.
    fn splits(
        &mut self,
        layout: &Layout<'_>,
        order: &[usize],
        start: usize,
        end: usize,
    ) -> Vec<(f64, StrokeVec)> {
        let width = end.saturating_sub(start).saturating_add(1);
        let depth = order.len().saturating_add(1);
        let cells = width.saturating_mul(depth);
        let mut frontier: Vec<Vec<(f64, StrokeVec)>> = vec![Vec::new(); cells];
        let mut next: Vec<Vec<(f64, StrokeVec)>> = vec![Vec::new(); cells];
        let last = width
            .saturating_sub(1)
            .saturating_mul(depth)
            .saturating_add(order.len());
        if let Some(slot) = frontier.first_mut() {
            slot.push((0.0_f64, StrokeVec::new()));
        }
        let mut done: Vec<(f64, StrokeVec)> = Vec::new();
        for taken in 0..order.len() {
            // A merge can jump several slots of `taken` ahead in one step (it lands at
            // depth `taken + length`, not `taken + 1`), so a hypothesis it produces has to
            // survive iterations that read some other depth before its own depth comes up.
            // Carrying `frontier` forward keeps it alive; only the depths this round
            // actually writes to change.
            next.clone_from(&frontier);
            let Some(index) = order.get(taken).copied() else {
                break;
            };
            let Some(child) = layout.children.get(index) else {
                break;
            };
            let child_offset = layout.offsets.get(index).copied().unwrap_or(0);
            let widest = layout
                .counts
                .get(index)
                .copied()
                .unwrap_or(1)
                .saturating_add(SLACK);
            for step in 0..width {
                let Some(partials) = frontier.get(step.saturating_mul(depth).saturating_add(taken))
                else {
                    continue;
                };
                if partials.is_empty() {
                    continue;
                }
                let partials = partials.clone();
                let position = start.saturating_add(step);
                let ceiling = end.min(position.saturating_add(widest));
                for stop in position..=ceiling {
                    let pieces = self.candidates(child, child_offset, position, stop, false);
                    if pieces.is_empty() {
                        continue;
                    }
                    let key = stop
                        .saturating_sub(start)
                        .saturating_mul(depth)
                        .saturating_add(taken.saturating_add(1));
                    let Some(slot) = next.get_mut(key) else {
                        continue;
                    };
                    for (base, history) in &partials {
                        for piece in pieces.iter() {
                            let mut grown = history.clone();
                            grown.extend(piece.order.iter().copied());
                            slot.push((base + piece.score, grown));
                        }
                    }
                }
                for (length, cost, joined) in self.merges(layout, order, taken, position, end) {
                    let key = position
                        .saturating_add(1)
                        .saturating_sub(start)
                        .saturating_mul(depth)
                        .saturating_add(taken.saturating_add(length));
                    let Some(slot) = next.get_mut(key) else {
                        continue;
                    };
                    for (base, history) in &partials {
                        let mut grown = history.clone();
                        grown.extend(joined.iter().copied());
                        slot.push((base + cost, grown));
                    }
                }
            }
            for bucket in &mut next {
                bucket.sort_by(|a, b| a.0.total_cmp(&b.0));
                bucket.truncate(self.beam);
            }
            swap(&mut frontier, &mut next);
            if let Some(bucket) = frontier.get_mut(last) {
                done.extend(bucket.iter().cloned());
                // Harvested once; left alone it would keep being carried forward and
                // re-harvested by every remaining iteration's check below.
                bucket.clear();
            }
        }
        if order.is_empty() && start == end {
            done.push((0.0_f64, StrokeVec::new()));
        }
        done.sort_by(|a, b| a.0.total_cmp(&b.0));
        done.truncate(self.beam);
        done
    }

    /// One drawn stroke standing for a run of consecutive children.
    fn merges(
        &mut self,
        layout: &Layout<'_>,
        order: &[usize],
        taken: usize,
        position: usize,
        end: usize,
    ) -> Vec<(usize, f64, StrokeVec)> {
        if position >= end || self.user.get(position).is_none() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for length in 2..=MERGE_UPTO {
            let Some(run) = order.get(taken..taken.saturating_add(length)) else {
                break;
            };
            let Some(first) = run.first().copied() else {
                break;
            };
            let consecutive = run
                .iter()
                .enumerate()
                .all(|(step, value)| *value == first.saturating_add(step));
            if !consecutive || layout.children.len() < first.saturating_add(length) {
                continue;
            }
            let strokes: usize = run
                .iter()
                .filter_map(|value| layout.counts.get(*value).copied())
                .sum();
            if strokes != length {
                continue;
            }
            let Some(offset) = layout.offsets.get(first).copied() else {
                continue;
            };
            let Some(joined) = self.joined_reference(offset, length) else {
                continue;
            };
            let Some(drawn) = self.user.get(position) else {
                continue;
            };
            let Some(cost) = joined.leaf_cost(drawn, &self.weights) else {
                continue;
            };
            let mut assign = StrokeVec::new();
            assign.push(index_of(position));
            for _ in 1..length {
                assign.push(FILLER);
            }
            out.push((length, cost + self.weights.merge_penalty, assign));
        }
        out
    }

    /// Shape of consecutive reference strokes drawn as one motion, cached per run.
    fn joined_reference(&mut self, offset: usize, length: usize) -> Option<Shape> {
        if let Some(cached) = self.joined.get(&(offset, length)) {
            return *cached;
        }
        let out = joined_reference_shape(&self.points, offset, length);
        self.joined.insert((offset, length), out);
        out
    }
}

/// Shape of `length` consecutive reference strokes starting at `offset`, drawn as one
/// motion.
#[must_use]
#[inline]
pub fn joined_reference_shape(
    reference_strokes: &[Vec<StrokePoint>],
    offset: usize,
    length: usize,
) -> Option<Shape> {
    let mut points: Vec<StrokePoint> = Vec::new();
    for step in 0..length {
        let stroke = reference_strokes.get(offset.saturating_add(step))?;
        for (index, point) in stroke.iter().enumerate() {
            let mut copy = *point;
            if index == 0 {
                copy.displacement = match points.last() {
                    Some(previous) => Vec2::new(
                        point.position.x - previous.position.x,
                        point.position.y - previous.position.y,
                    ),
                    None => Vec2::ZERO,
                };
            }
            points.push(copy);
        }
    }
    let shape = points.to_shape();
    shape.is_usable().then_some(shape)
}

/// Puts the flat list back in reference order, since children were taken in drawing order.
fn reorder(counts: &[usize], order: &[usize], taken: &[u8]) -> StrokeVec {
    let mut slices: Vec<Option<&[u8]>> = vec![None; counts.len()];
    let mut cursor = 0_usize;
    for index in order {
        let width = counts.get(*index).copied().unwrap_or(1);
        let piece = taken.get(cursor..cursor.saturating_add(width));
        if let (Some(slot), Some(piece)) = (slices.get_mut(*index), piece) {
            *slot = Some(piece);
        }
        cursor = cursor.saturating_add(width);
    }
    let mut flat = StrokeVec::new();
    for (position, slice) in slices.iter().enumerate() {
        let width = counts.get(position).copied().unwrap_or(1);
        match slice {
            Some(values) if values.len() == width => flat.extend(values.iter().copied()),
            _ => flat.resize(flat.len().saturating_add(width), MISSING),
        }
    }
    flat
}

/// Every ordering worth trying for a group, least disorder first.
fn orderings(count: usize) -> Vec<Vec<usize>> {
    let identity: Vec<usize> = (0..count).collect();
    if !(2..=PERMUTE_UPTO).contains(&count) {
        return vec![identity];
    }
    let mut out = Vec::new();
    permute(&identity, &mut Vec::new(), &mut out);
    out.sort_by_key(|order| inversions(order));
    out
}

fn permute(rest: &[usize], acc: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if rest.is_empty() {
        out.push(acc.clone());
        return;
    }
    for (index, value) in rest.iter().enumerate() {
        let mut remaining = rest.to_vec();
        remaining.remove(index);
        acc.push(*value);
        permute(&remaining, acc, out);
        acc.pop();
    }
}

fn inversions(order: &[usize]) -> usize {
    order
        .iter()
        .enumerate()
        .flat_map(|(index, left)| {
            order
                .iter()
                .skip(index.saturating_add(1))
                .filter(move |right| left > right)
        })
        .count()
}

/// Filler entries are not drawn strokes, so scoring must see them as undrawn.
pub(crate) fn scoring_order(order: &[u8]) -> StrokeVec {
    order
        .iter()
        .map(|value| if *value == FILLER { MISSING } else { *value })
        .collect()
}

fn mask_of(order: &[u8]) -> u32 {
    let mut mask = 0_u32;
    for value in order {
        if *value != MISSING && *value != FILLER {
            mask |= 1_u32.checked_shl(u32::from(*value)).unwrap_or(0);
        }
    }
    mask
}

fn index_of(value: usize) -> u8 {
    u8::try_from(value).unwrap_or(MISSING)
}

fn convert(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke_point::to_stroke_points;
    use kurbo::Point;

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
        let mut left = Vec::new();
        let mut right = Vec::new();
        for step in 0..4_u8 {
            let y = 0.1 + f64::from(step) * 0.2;
            left.push(stroke(step, &[(0.08, y), (0.38, y)]));
            right.push(stroke(step.saturating_add(4), &[(0.60, y), (0.92, y)]));
        }
        AnalyzedKanjiNode::Group {
            element: '語',
            children: vec![
                AnalyzedKanjiNode::Group {
                    element: '言',
                    children: left,
                },
                AnalyzedKanjiNode::Group {
                    element: '吾',
                    children: right,
                },
            ],
        }
    }

    fn best(tree: AnalyzedKanjiNode, user: Vec<Vec<StrokePoint>>, width: usize) -> StrokeVec {
        match_strokes(tree, user, Weights::v1(), width)
            .first()
            .map(|result| result.user_stroke_order.clone())
            .unwrap_or_default()
    }

    #[test]
    fn a_clean_drawing_matches_in_order() {
        let tree = three();
        let ink = tree.collect_strokes();
        assert_eq!(best(tree, ink, 3).as_slice(), &[0, 1, 2]);
    }

    #[test]
    fn a_nested_tree_matches_in_order() {
        let tree = nested();
        let ink = tree.collect_strokes();
        assert_eq!(best(tree, ink, 3).as_slice(), &[0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn a_forgotten_stroke_may_be_absorbed_by_a_merge_instead_of_left_missing() {
        let user = vec![path(&horizontal(0.2)), path(&horizontal(0.8))];
        let order = best(three(), user, 3);
        assert_eq!(order.len(), 3);
        let missing = order.iter().filter(|v| **v == MISSING).count();
        let merged = order.iter().filter(|v| **v == FILLER).count();
        assert_eq!(missing + merged, 1, "{order:?}");
    }

    #[test]
    fn a_spare_stroke_is_absorbed_by_a_slice() {
        let tree = three();
        let mut user = tree.collect_strokes();
        user.push(path(&[(0.5, 0.1), (0.5, 0.9)]));
        let order = best(tree, user, 3);
        assert_eq!(order.len(), 3);
        assert!(order.iter().all(|value| *value != MISSING));
    }

    #[test]
    fn a_reordered_drawing_is_recovered() {
        let user = vec![
            path(&horizontal(0.8)),
            path(&horizontal(0.5)),
            path(&horizontal(0.2)),
        ];
        assert_eq!(best(three(), user, 3).as_slice(), &[2, 1, 0]);
    }

    #[test]
    fn every_result_covers_every_reference_stroke() {
        let tree = nested();
        let ink = tree.collect_strokes();
        let leaves = tree.leaf_count();
        for result in match_strokes(tree, ink, Weights::v1(), 3) {
            assert_eq!(result.user_stroke_order.len(), leaves);
        }
    }

    #[test]
    fn no_drawn_stroke_is_claimed_twice() {
        let tree = nested();
        let ink = tree.collect_strokes();
        for result in match_strokes(tree, ink, Weights::v1(), 3) {
            let mut used: Vec<u8> = result
                .user_stroke_order
                .iter()
                .copied()
                .filter(|value| *value != MISSING && *value != FILLER)
                .collect();
            used.sort_unstable();
            let before = used.len();
            used.dedup();
            assert_eq!(before, used.len(), "{:?}", result.user_stroke_order);
        }
    }

    #[test]
    fn a_joined_stroke_is_told_apart_from_a_forgotten_one() {
        let user = vec![path(&[(0.2, 0.2), (0.8, 0.2), (0.2, 0.5), (0.8, 0.5)])];
        let results = match_strokes(three(), user, Weights::v1(), 16);
        assert!(
            results
                .iter()
                .any(|result| result.user_stroke_order.contains(&FILLER)),
            "no joined reading was offered"
        );
    }

    #[test]
    fn a_joined_reading_never_reuses_the_drawn_stroke() {
        let user = vec![path(&[(0.2, 0.2), (0.8, 0.2), (0.2, 0.5), (0.8, 0.5)])];
        for result in match_strokes(three(), user, Weights::v1(), 16) {
            let drawn = result
                .user_stroke_order
                .iter()
                .filter(|value| **value != MISSING && **value != FILLER)
                .count();
            assert!(drawn <= 1, "{:?}", result.user_stroke_order);
        }
    }

    #[test]
    fn a_drawing_with_nothing_in_it_still_answers() {
        let order = best(three(), Vec::new(), 3);
        assert_eq!(order.as_slice(), &[MISSING, MISSING, MISSING]);
    }

    #[test]
    fn a_wider_beam_never_makes_the_best_score_worse() {
        let tree = nested();
        let ink = tree.collect_strokes();
        let narrow = match_strokes(tree.clone(), ink.clone(), Weights::v1(), 2)
            .first()
            .map_or(f64::INFINITY, |result| result.score);
        let wide = match_strokes(tree, ink, Weights::v1(), 12)
            .first()
            .map_or(f64::INFINITY, |result| result.score);
        assert!(wide <= narrow + 1e-9, "{wide} vs {narrow}");
    }

    #[test]
    fn the_reported_width_is_the_width_asked_for() {
        let tree = nested();
        let ink = tree.collect_strokes();
        for result in match_strokes(tree, ink, Weights::v1(), 7) {
            assert_eq!(result.beam_width, 7);
        }
    }

    #[test]
    fn every_order_is_as_long_as_the_tree_has_strokes() {
        for tree in [three(), nested()] {
            let leaves = tree.leaf_count();
            let mut ink = tree.collect_strokes();
            ink.truncate(leaves.saturating_sub(1));
            for result in match_strokes(tree.clone(), ink, Weights::v1(), 4) {
                assert_eq!(result.user_stroke_order.len(), leaves);
            }
        }
    }
}
