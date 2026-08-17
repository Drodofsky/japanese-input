use kurbo::{Point, Vec2};
use smallvec::SmallVec;

use crate::{
    analyzed_kanji_node::AnalyzedKanjiNode, convert_lossy::ConvertLossy as _,
    leaf_score::LeafScore as _, match_strokes::joined_reference_shape, shape::Shape,
    stroke_geometry::StrokeGeometry, stroke_point::StrokePoint, weights::Weights,
};

/// One centroid pair per child.
type Centroids = SmallVec<[(Point, Point); 8]>;

/// One matched leaf, holding the reference and user geometry that were paired.
type Matches = SmallVec<[(StrokeGeometry, StrokeGeometry); 32]>;

/// One entry per matched leaf, holding which user stroke it took and which child owns it.
type Ownership = SmallVec<[(u8, usize); 32]>;

pub const GROUP_FEATURE_COUNT: usize = 6;

/// Guards a division by an arc length.
const EPS: f64 = 1e-12;

pub trait GroupScore {
    fn group_features(
        &self,
        user_stroke_order: &[u8],
        user_stroke_geometries: &[StrokeGeometry],
        user_shapes: &[Shape],
        weights: &Weights,
    ) -> [f64; GROUP_FEATURE_COUNT];

    fn group_score(
        &self,
        user_stroke_order: &[u8],
        user_stroke_geometries: &[StrokeGeometry],
        user_shapes: &[Shape],
        weights: &Weights,
        root: bool,
    ) -> f64;
}

impl GroupScore for AnalyzedKanjiNode {
    #[inline]
    fn group_features(
        &self,
        user_stroke_order: &[u8],
        user_stroke_geometries: &[StrokeGeometry],
        user_shapes: &[Shape],
        weights: &Weights,
    ) -> [f64; GROUP_FEATURE_COUNT] {
        let children = match self {
            AnalyzedKanjiNode::Group { children, .. } => children,
            AnalyzedKanjiNode::Stroke { .. } => return [0.0; GROUP_FEATURE_COUNT],
        };
        if user_stroke_order.iter().all(|index| *index == u8::MAX) {
            return [0.0; GROUP_FEATURE_COUNT];
        }
        let reference = self.collect_geometry();
        let (centroids, ownership) = gather(
            children,
            &reference,
            user_stroke_order,
            user_stroke_geometries,
        );
        let matches = matched_pairs(&reference, user_stroke_order, user_stroke_geometries);
        [
            disorder(user_stroke_order),
            placement(&centroids),
            contiguity(&ownership),
            relative_length(&matches),
            absolute_position(&matches),
            cross_group_bonus(
                children,
                &self.collect_strokes(),
                user_stroke_order,
                user_shapes,
                weights,
            ),
        ]
    }

    #[inline]
    fn group_score(
        &self,
        user_stroke_order: &[u8],
        user_stroke_geometries: &[StrokeGeometry],
        user_shapes: &[Shape],
        weights: &Weights,
        root: bool,
    ) -> f64 {
        let features = self.group_features(
            user_stroke_order,
            user_stroke_geometries,
            user_shapes,
            weights,
        );
        let scales = [
            weights.order_weight,
            weights.group_weight,
            weights.contiguity_weight,
            weights.rel_length_weight,
            weights.abs_position_weight,
            weights.cross_group_weight,
        ];
        // `absolute_position` (slot 4) is recomputed identically at every nesting level (each
        // ancestor's own local order still covers the same descendant positions), so it's only
        // charged once, at the root. `cross_group_bonus` (slot 5) is the opposite: it only ever
        // looks at a boundary between two of *this* node's own direct children, a boundary no
        // ancestor or descendant call can see, so it can never be double-counted and is charged
        // at every level.
        features
            .iter()
            .zip(scales.iter())
            .enumerate()
            .filter(|(index, _)| root || *index != 4)
            .map(|(_, (feature, scale))| feature * scale)
            .sum()
    }
}

fn gather(
    children: &[AnalyzedKanjiNode],
    reference: &[StrokeGeometry],
    user_stroke_order: &[u8],
    user_stroke_geometries: &[StrokeGeometry],
) -> (Centroids, Ownership) {
    let mut centroids = Centroids::new();
    let mut ownership = Ownership::new();
    let mut start = 0_usize;
    for (child_index, child) in children.iter().enumerate() {
        let end = start.saturating_add(child.leaf_count());
        let mut reference_sum = Vec2::ZERO;
        let mut user_sum = Vec2::ZERO;
        let mut matched = 0_usize;
        for local in start..end {
            let pair = matched_centroids(
                reference.get(local),
                user_stroke_order.get(local),
                user_stroke_geometries,
            );
            if let Some((user_index, expected, drawn)) = pair {
                reference_sum = add(reference_sum, expected);
                user_sum = add(user_sum, drawn);
                matched = matched.saturating_add(1);
                ownership.push((user_index, child_index));
            }
        }
        if matched > 0 {
            let scale = 1.0_f64 / matched.convert_lossy();
            centroids.push((scaled(reference_sum, scale), scaled(user_sum, scale)));
        }
        start = end;
    }
    (centroids, ownership)
}

fn matched_pairs(
    reference: &[StrokeGeometry],
    user_stroke_order: &[u8],
    user_stroke_geometries: &[StrokeGeometry],
) -> Matches {
    let mut matches = Matches::new();
    for (local, user_index) in user_stroke_order.iter().enumerate() {
        if *user_index == u8::MAX {
            continue;
        }
        let expected = reference.get(local);
        let drawn = user_stroke_geometries.get(usize::from(*user_index));
        if let (Some(expected), Some(drawn)) = (expected, drawn) {
            matches.push((*expected, *drawn));
        }
    }
    matches
}

fn relative_length(matches: &Matches) -> f64 {
    let mut total = 0.0_f64;
    let mut pairs = 0_usize;
    for (index, (left_reference, left_user)) in matches.iter().enumerate() {
        for (right_reference, right_user) in matches.iter().skip(index.saturating_add(1)) {
            let expected = log_ratio(left_reference.arc_len, right_reference.arc_len);
            let drawn = log_ratio(left_user.arc_len, right_user.arc_len);
            if let (Some(expected), Some(drawn)) = (expected, drawn) {
                // Not squared: a log-ratio mismatch under 1.0 shrinks a lot faster squared
                // than it does in absolute value, so a real but modest disagreement (the
                // usual case) barely registered even at a generously large weight. `.abs()`
                // keeps the same shape — zero for a perfect match, growing with the
                // disagreement — without crushing everything short of an extreme mismatch
                // down near zero.
                total += (expected - drawn).abs();
                pairs = pairs.saturating_add(1);
            }
        }
    }
    if pairs == 0 {
        return 0.0;
    }
    total / pairs.convert_lossy()
}

fn absolute_position(matches: &Matches) -> f64 {
    let mut total = 0.0_f64;
    let mut counted = 0_usize;
    for (expected, drawn) in matches {
        if let (Some(expected), Some(drawn)) = (expected.centroid, drawn.centroid) {
            let gap = offset(expected, drawn);
            total += gap.dot(gap);
            counted = counted.saturating_add(1);
        }
    }
    if counted == 0 {
        return 0.0;
    }
    total / counted.convert_lossy()
}

/// Log of one arc length over another, absent when either stroke has no extent.
#[inline]
fn log_ratio(left: f64, right: f64) -> Option<f64> {
    if left <= EPS || right <= EPS || !left.is_finite() || !right.is_finite() {
        return None;
    }
    Some((left / right).ln())
}

#[inline]
fn matched_centroids(
    reference: Option<&StrokeGeometry>,
    user_index: Option<&u8>,
    user_stroke_geometries: &[StrokeGeometry],
) -> Option<(u8, Point, Point)> {
    let user_index = *user_index?;
    if user_index == u8::MAX {
        return None;
    }
    let expected = reference?.centroid?;
    let drawn = user_stroke_geometries
        .get(usize::from(user_index))?
        .centroid?;
    Some((user_index, expected, drawn))
}

fn placement(centroids: &Centroids) -> f64 {
    let mut total = 0.0_f64;
    let mut pairs = 0_usize;
    for (index, &(left_reference, left_user)) in centroids.iter().enumerate() {
        for &(right_reference, right_user) in centroids.iter().skip(index.saturating_add(1)) {
            let expected = offset(left_reference, right_reference);
            let drawn = offset(left_user, right_user);
            let gap = Vec2::new(expected.x - drawn.x, expected.y - drawn.y);
            total += gap.dot(gap);
            pairs = pairs.saturating_add(1);
        }
    }
    if pairs == 0 {
        return 0.0;
    }
    total / pairs.convert_lossy()
}

fn contiguity(ownership: &Ownership) -> f64 {
    let mut sorted = ownership.clone();
    sorted.sort_unstable_by_key(|entry| entry.0);
    let mut distinct = 0_usize;
    let mut transitions = 0_usize;
    let mut previous: Option<usize> = None;
    let mut seen = 0_u32;
    for &(_, child_index) in &sorted {
        if previous != Some(child_index) {
            transitions = transitions.saturating_add(1);
            previous = Some(child_index);
        }
        let bit = 1_u32.checked_shl(u32::try_from(child_index).unwrap_or(u32::MAX));
        if let Some(bit) = bit
            && seen & bit == 0
        {
            seen |= bit;
            distinct = distinct.saturating_add(1);
        }
    }
    let span = sorted.len().saturating_sub(distinct);
    if span == 0 {
        return 0.0;
    }
    let excess = transitions.saturating_sub(distinct);
    excess.convert_lossy() / span.convert_lossy()
}

/// A negative count, for every boundary between two of this node's own direct children where
/// missing reference leaves touch the boundary from *both* sides *and* some drawn stroke's
/// shape actually matches what those missing leaves would look like glued into one motion:
/// how many missing leaves are in that connected run. Zero whenever nothing is missing right
/// at a boundary, and zero even when it is, unless a real stroke backs up the story.
///
/// One drawn stroke that glues together a group-ending leaf and the next group's opening leaf
/// can't be recognized as a merge (`Solver::merges` only ever looks within one group's own
/// children), so the matcher's only way to accept that input is to leave both reference leaves
/// missing and the drawn stroke unassigned ("extra"). `missing_penalty`/`extra_penalty` charge
/// that outcome the same as any ordinary missing or stray stroke anywhere else in the kanji,
/// which is too blunt an instrument to fix without also making those two penalties too weak
/// everywhere else. This feature gives the optimizer a narrow, separate knob for exactly this
/// shape instead — but only as much as the geometry actually backs it up.
///
/// `leaf_cost` (the same one `Solver::merges` uses for an ordinary same-group merge) returns
/// `Some` for almost any pair of usable strokes, cheap or not — accepting is a low bar, not a
/// good-match signal. So the discount isn't a flat award for finding *any* usable stroke; it's
/// the run length minus the best matching cost found anywhere in the drawing, floored at zero.
/// A perfect match keeps the full discount; a merely-plausible but poor one earns little or
/// none; nothing ever turns this into a penalty.
/// How far apart (in the kanji's own roughly-unit-square reference space) a group-ending
/// leaf's last point and the next group's opening leaf's first point may sit and still
/// plausibly be one continuous, pen-never-lifted motion.
///
/// Real hand-drawn cross-group connections (`見`'s 目/legs boundary, `百`'s 白/日 boundary)
/// sit at 0.12-0.36. `leaf_cost`'s harmonic comparison judges the *aggregate* joined shape,
/// not continuity at the seam, so on its own it let a 0.67-gap connection in `円` — the
/// bottom-right of its box radical jumping to a stroke starting near top-middle, a jump no
/// real writer makes without lifting the pen — score as a plausible match. This catches
/// what that comparison misses.
pub(crate) const MAX_BOUNDARY_GAP: f64 = 0.4;

/// The endpoint-to-endpoint distance across one candidate boundary: the last point of
/// `reference_points[left]` to the first point of `reference_points[right]`.
#[must_use]
pub(crate) fn boundary_gap(
    reference_points: &[Vec<StrokePoint>],
    left: usize,
    right: usize,
) -> Option<f64> {
    let left_last = reference_points.get(left)?.last()?.position;
    let right_first = reference_points.get(right)?.first()?.position;
    let dx = right_first.x - left_last.x;
    let dy = right_first.y - left_last.y;
    Some(dx.mul_add(dx, dy * dy).sqrt())
}

fn cross_group_bonus(
    children: &[AnalyzedKanjiNode],
    reference_points: &[Vec<StrokePoint>],
    user_stroke_order: &[u8],
    user_shapes: &[Shape],
    weights: &Weights,
) -> f64 {
    let mut ranges = SmallVec::<[(usize, usize); 8]>::new();
    let mut cursor = 0_usize;
    for child in children {
        let end = cursor.saturating_add(child.leaf_count());
        ranges.push((cursor, end));
        cursor = end;
    }
    let mut discount = 0.0_f64;
    for (siblings, pair) in children.windows(2).zip(ranges.windows(2)) {
        // A boundary between two plain single-leaf siblings is already `Solver::merges`'
        // territory (a real `FILLER` merge can represent it); scoring it here too would
        // just compete with that correct mechanism instead of covering ground it can't
        // reach.
        if siblings.iter().all(|child| child.leaf_count() == 1) {
            continue;
        }
        let (left_start, left_end) = pair[0];
        let (right_start, right_end) = pair[1];
        let too_far = boundary_gap(reference_points, left_end.saturating_sub(1), right_start)
            .is_none_or(|gap| gap > MAX_BOUNDARY_GAP);
        if too_far {
            continue;
        }
        let left_run = user_stroke_order
            .get(left_start..left_end)
            .unwrap_or(&[])
            .iter()
            .rev()
            .take_while(|value| **value == u8::MAX)
            .count();
        let right_run = user_stroke_order
            .get(right_start..right_end)
            .unwrap_or(&[])
            .iter()
            .take_while(|value| **value == u8::MAX)
            .count();
        if left_run == 0 || right_run == 0 {
            continue;
        }
        let run_start = left_end.saturating_sub(left_run);
        let run_length = left_run.saturating_add(right_run);
        let Some(joined) = joined_reference_shape(reference_points, run_start, run_length) else {
            continue;
        };
        let best_cost = user_shapes
            .iter()
            .filter_map(|drawn| joined.leaf_cost(drawn, weights))
            .fold(f64::INFINITY, f64::min);
        discount += (run_length.convert_lossy() - best_cost).max(0.0);
    }
    -discount
}

fn disorder(user_stroke_order: &[u8]) -> f64 {
    let drawn: SmallVec<[u8; 32]> = user_stroke_order
        .iter()
        .copied()
        .filter(|index| *index != u8::MAX)
        .collect();
    if drawn.len() < 2 {
        return 0.0;
    }
    let inversions = drawn
        .iter()
        .enumerate()
        .flat_map(|(position, left)| {
            drawn
                .iter()
                .skip(position.saturating_add(1))
                .filter(move |right| left > right)
        })
        .count();
    let count = drawn.len();
    let maximum = count
        .saturating_mul(count.saturating_sub(1))
        .saturating_div(2);
    if maximum == 0 {
        return 0.0;
    }
    inversions.convert_lossy() / maximum.convert_lossy()
}

#[inline]
fn add(accumulator: Vec2, point: Point) -> Vec2 {
    Vec2::new(accumulator.x + point.x, accumulator.y + point.y)
}

#[inline]
fn scaled(accumulator: Vec2, scale: f64) -> Point {
    Point::new(accumulator.x * scale, accumulator.y * scale)
}

#[inline]
fn offset(from: Point, to: Point) -> Vec2 {
    Vec2::new(from.x - to.x, from.y - to.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke_point::{StrokePoint, to_stroke_points};
    use smallvec::smallvec;

    fn stroke(index: u8, points: &[(f64, f64)]) -> AnalyzedKanjiNode {
        let path: Vec<StrokePoint> =
            to_stroke_points(points.iter().map(|&(x, y)| Point::new(x, y)));
        let geometry = StrokeGeometry::from_stroke(&path);
        AnalyzedKanjiNode::Stroke {
            index,
            path,
            geometry,
        }
    }

    fn group(element: char, children: Vec<AnalyzedKanjiNode>) -> AnalyzedKanjiNode {
        AnalyzedKanjiNode::Group { element, children }
    }

    fn geometries(strokes: &[&[(f64, f64)]]) -> Vec<StrokeGeometry> {
        strokes
            .iter()
            .map(|points| {
                let path: Vec<StrokePoint> =
                    to_stroke_points(points.iter().map(|&(x, y)| Point::new(x, y)));
                StrokeGeometry::from_stroke(&path)
            })
            .collect()
    }

    fn approx(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() < tolerance
    }

    fn horizontal(y: f64) -> Vec<(f64, f64)> {
        vec![(0.2, y), (0.8, y)]
    }

    /// Loosely 三: three stacked horizontals as three separate children.
    fn three() -> AnalyzedKanjiNode {
        group(
            '三',
            vec![
                stroke(0, &horizontal(0.2)),
                stroke(1, &horizontal(0.5)),
                stroke(2, &horizontal(0.8)),
            ],
        )
    }

    fn three_geometries() -> Vec<StrokeGeometry> {
        geometries(&[&horizontal(0.2), &horizontal(0.5), &horizontal(0.8)])
    }

    #[test]
    fn a_stroke_node_has_no_group_score() {
        let score = stroke(0, &horizontal(0.2)).group_score(
            &[0],
            &three_geometries(),
            &[],
            &Weights::v1(),
            true,
        );
        assert!(approx(score, 0.0, 1e-12));
    }

    #[test]
    fn a_correct_group_scores_zero() {
        let score = three().group_score(&[0, 1, 2], &three_geometries(), &[], &Weights::v1(), true);
        assert!(approx(score, 0.0, 1e-12), "{score}");
    }

    #[test]
    fn a_shifted_group_keeps_its_relative_terms_at_zero() {
        let shifted = geometries(&[
            &vec![(0.5, 0.4), (1.1, 0.4)],
            &vec![(0.5, 0.7), (1.1, 0.7)],
            &vec![(0.5, 1.0), (1.1, 1.0)],
        ]);
        let features = three().group_features(&[0, 1, 2], &shifted, &[], &Weights::v1());
        for (index, value) in features.iter().enumerate().take(4) {
            assert!(approx(*value, 0.0, 1e-12), "feature {index} moved: {value}");
        }
        let absolute = features.get(4).copied().unwrap_or(0.0);
        assert!(
            absolute > 1e-3,
            "absolute position should notice a shift: {absolute}"
        );
    }

    #[test]
    fn a_shifted_group_scores_zero_below_the_root() {
        let shifted = geometries(&[
            &vec![(0.5, 0.4), (1.1, 0.4)],
            &vec![(0.5, 0.7), (1.1, 0.7)],
            &vec![(0.5, 1.0), (1.1, 1.0)],
        ]);
        let score = three().group_score(&[0, 1, 2], &shifted, &[], &Weights::v1(), false);
        assert!(approx(score, 0.0, 1e-12), "{score}");
    }

    #[test]
    fn swapping_two_siblings_costs_placement_and_order() {
        let weights = Weights::v1();
        let swapped = three().group_score(&[0, 2, 1], &three_geometries(), &[], &weights, true);
        let correct = three().group_score(&[0, 1, 2], &three_geometries(), &[], &weights, true);
        assert!(swapped > correct, "{swapped} vs {correct}");
    }

    #[test]
    fn a_squashed_group_costs_placement() {
        let squashed = geometries(&[&horizontal(0.45), &horizontal(0.5), &horizontal(0.55)]);
        let score = three().group_score(&[0, 1, 2], &squashed, &[], &Weights::v1(), true);
        assert!(score > 1e-3, "{score}");
    }

    #[test]
    fn placement_is_ignored_when_fewer_than_two_children_match() {
        let order = [0, u8::MAX, u8::MAX];
        let score = three().group_score(&order, &three_geometries(), &[], &Weights::v1(), true);
        assert!(approx(score, 0.0, 1e-12), "{score}");
    }

    #[test]
    fn an_undrawn_leaf_does_not_break_the_remaining_offsets() {
        let order = [0, u8::MAX, 2];
        let score = three().group_score(&order, &three_geometries(), &[], &Weights::v1(), true);
        assert!(approx(score, 0.0, 1e-12), "{score}");
    }

    /// Loosely 語: a three-stroke block beside a two-stroke block.
    fn two_blocks() -> AnalyzedKanjiNode {
        group(
            '語',
            vec![
                group(
                    '言',
                    vec![
                        stroke(0, &[(0.1, 0.1), (0.3, 0.1)]),
                        stroke(1, &[(0.1, 0.3), (0.3, 0.3)]),
                        stroke(2, &[(0.1, 0.5), (0.3, 0.5)]),
                    ],
                ),
                group(
                    '吾',
                    vec![
                        stroke(3, &[(0.7, 0.1), (0.9, 0.1)]),
                        stroke(4, &[(0.7, 0.3), (0.9, 0.3)]),
                    ],
                ),
            ],
        )
    }

    fn two_block_geometries() -> Vec<StrokeGeometry> {
        geometries(&[
            &vec![(0.1, 0.1), (0.3, 0.1)],
            &vec![(0.1, 0.3), (0.3, 0.3)],
            &vec![(0.1, 0.5), (0.3, 0.5)],
            &vec![(0.7, 0.1), (0.9, 0.1)],
            &vec![(0.7, 0.3), (0.9, 0.3)],
        ])
    }

    #[test]
    fn blocks_drawn_one_after_the_other_cost_no_contiguity() {
        let ownership: Ownership = smallvec![(0, 0), (1, 0), (2, 0), (3, 1), (4, 1)];
        assert!(approx(contiguity(&ownership), 0.0, 1e-12));
    }

    #[test]
    fn blocks_drawn_in_the_other_order_still_cost_no_contiguity() {
        let ownership: Ownership = smallvec![(3, 0), (4, 0), (0, 1), (1, 1), (2, 1)];
        assert!(approx(contiguity(&ownership), 0.0, 1e-12));
    }

    #[test]
    fn fully_interleaved_blocks_cost_the_most_contiguity() {
        let ownership: Ownership = smallvec![(0, 0), (2, 0), (4, 0), (1, 1), (3, 1)];
        assert!(approx(contiguity(&ownership), 1.0, 1e-12));
    }

    #[test]
    fn one_stroke_taken_out_of_its_block_costs_some_contiguity() {
        let ownership: Ownership = smallvec![(0, 0), (1, 0), (4, 0), (2, 1), (3, 1)];
        let cost = contiguity(&ownership);
        assert!(cost > 0.0 && cost < 1.0, "{cost}");
    }

    #[test]
    fn a_single_block_never_costs_contiguity() {
        let ownership: Ownership = smallvec![(2, 0), (0, 0), (1, 0)];
        assert!(approx(contiguity(&ownership), 0.0, 1e-12));
    }

    #[test]
    fn contiguity_catches_a_split_block_that_order_alone_misses() {
        let mut values = Weights::v1().to_vec();
        if let Some(slot) = values.get_mut(10) {
            *slot = 0.0;
        }
        if let Some(slot) = values.get_mut(11) {
            *slot = 0.0;
        }
        let weights = Weights::try_from(values.as_slice()).expect("weights");
        let clean = two_blocks().group_score(
            &[0, 1, 2, 3, 4],
            &two_block_geometries(),
            &[],
            &weights,
            true,
        );
        let split = two_blocks().group_score(
            &[0, 1, 4, 2, 3],
            &two_block_geometries(),
            &[],
            &weights,
            true,
        );
        assert!(approx(clean, 0.0, 1e-12), "{clean}");
        assert!(split > 0.0, "{split}");
    }

    #[test]
    fn a_reordered_block_is_cheaper_than_a_split_one() {
        let weights = Weights::v1();
        let geometries = two_block_geometries();
        let swapped = two_blocks().group_score(&[1, 0, 2, 3, 4], &geometries, &[], &weights, true);
        let split = two_blocks().group_score(&[0, 1, 4, 2, 3], &geometries, &[], &weights, true);
        assert!(split > swapped, "split {split}, swapped {swapped}");
    }

    #[test]
    fn a_group_score_never_goes_negative() {
        let geometries = two_block_geometries();
        let orders: [[u8; 5]; 4] = [
            [0, 1, 2, 3, 4],
            [4, 3, 2, 1, 0],
            [0, 1, 4, 2, 3],
            [2, 0, 1, 4, 3],
        ];
        for order in orders {
            let score = two_blocks().group_score(&order, &geometries, &[], &Weights::v1(), true);
            assert!(score >= 0.0, "{score}");
        }
    }

    /// 三 with unequal strokes, so relative length has something to say.
    fn stepped() -> AnalyzedKanjiNode {
        group(
            '三',
            vec![
                stroke(0, &[(0.3, 0.2), (0.7, 0.2)]),
                stroke(1, &[(0.25, 0.5), (0.75, 0.5)]),
                stroke(2, &[(0.1, 0.8), (0.9, 0.8)]),
            ],
        )
    }

    fn stepped_geometries() -> Vec<StrokeGeometry> {
        geometries(&[
            &vec![(0.3, 0.2), (0.7, 0.2)],
            &vec![(0.25, 0.5), (0.75, 0.5)],
            &vec![(0.1, 0.8), (0.9, 0.8)],
        ])
    }

    #[test]
    fn relative_length_is_zero_when_the_ordering_of_lengths_matches() {
        let features =
            stepped().group_features(&[0, 1, 2], &stepped_geometries(), &[], &Weights::v1());
        assert!(approx(features.get(3).copied().unwrap_or(1.0), 0.0, 1e-12));
    }

    #[test]
    fn relative_length_notices_two_strokes_traded_by_length() {
        let swapped =
            stepped().group_features(&[2, 1, 0], &stepped_geometries(), &[], &Weights::v1());
        assert!(swapped.get(3).copied().unwrap_or(0.0) > 1e-3);
    }

    #[test]
    fn relative_length_survives_the_writer_working_larger() {
        let doubled = geometries(&[
            &vec![(0.1, 0.2), (0.9, 0.2)],
            &vec![(0.0, 0.5), (1.0, 0.5)],
            &vec![(-0.3, 0.8), (1.3, 0.8)],
        ]);
        let features = stepped().group_features(&[0, 1, 2], &doubled, &[], &Weights::v1());
        assert!(approx(features.get(3).copied().unwrap_or(1.0), 0.0, 1e-9));
    }

    #[test]
    fn relative_length_needs_two_matched_strokes() {
        let order = [0, u8::MAX, u8::MAX];
        let features = stepped().group_features(&order, &stepped_geometries(), &[], &Weights::v1());
        assert!(approx(features.get(3).copied().unwrap_or(1.0), 0.0, 1e-12));
    }

    #[test]
    fn absolute_position_charges_a_swap_the_relative_terms_also_catch() {
        let geometries = three_geometries();
        let correct = three().group_features(&[0, 1, 2], &geometries, &[], &Weights::v1());
        let swapped = three().group_features(&[2, 1, 0], &geometries, &[], &Weights::v1());
        assert!(approx(correct.get(4).copied().unwrap_or(1.0), 0.0, 1e-12));
        assert!(swapped.get(4).copied().unwrap_or(0.0) > 1e-3);
    }

    /// 三 drawn with two strokes: the labels differ only in which reference went undrawn.
    #[test]
    fn absolute_position_separates_two_ways_of_skipping_one_stroke() {
        let drawn = geometries(&[&horizontal(0.2), &horizontal(0.5)]);
        let early = three().group_features(&[0, 1, u8::MAX], &drawn, &[], &Weights::v1());
        let late = three().group_features(&[0, u8::MAX, 1], &drawn, &[], &Weights::v1());
        assert!(approx(
            early.first().copied().unwrap_or(0.0),
            late.first().copied().unwrap_or(1.0),
            1e-12
        ));
        let gap =
            (early.get(4).copied().unwrap_or(0.0) - late.get(4).copied().unwrap_or(0.0)).abs();
        assert!(gap > 1e-3, "absolute position should separate these: {gap}");
    }

    #[test]
    fn absolute_position_is_zero_for_a_perfect_copy() {
        let features = three().group_features(&[0, 1, 2], &three_geometries(), &[], &Weights::v1());
        assert!(approx(features.get(4).copied().unwrap_or(1.0), 0.0, 1e-12));
    }

    #[test]
    fn disorder_is_normalized_between_zero_and_one() {
        assert!(approx(disorder(&[0, 1, 2]), 0.0, 1e-12));
        assert!(approx(disorder(&[2, 1, 0]), 1.0, 1e-12));
        assert!(approx(disorder(&[0]), 0.0, 1e-12));
        assert!(approx(disorder(&[u8::MAX, u8::MAX]), 0.0, 1e-12));
    }

    #[test]
    fn disorder_ignores_undrawn_leaves() {
        assert!(approx(disorder(&[0, u8::MAX, 1]), 0.0, 1e-12));
        assert!(approx(disorder(&[1, u8::MAX, 0]), 1.0, 1e-12));
    }

    #[test]
    fn zero_weights_silence_every_group_channel() {
        let mut values = Weights::v1().to_vec();
        for index in [10, 11, 12, 13, 14] {
            if let Some(slot) = values.get_mut(index) {
                *slot = 0.0;
            }
        }
        let weights = Weights::try_from(values.as_slice()).expect("weights");
        let score = two_blocks().group_score(
            &[2, 0, 1, 4, 3],
            &two_block_geometries(),
            &[],
            &weights,
            true,
        );
        assert!(approx(score, 0.0, 1e-12), "{score}");
    }
}
