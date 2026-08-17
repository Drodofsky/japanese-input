use crate::{
    analyzed_kanji_node::AnalyzedKanjiNode,
    convert_lossy::ConvertLossy as _,
    group_score::{GROUP_FEATURE_COUNT, GroupScore as _},
    leaf_score::{LEAF_FEATURE_COUNT, LeafScore as _},
    match_strokes::{FILLER, joined_reference_shape, scoring_order},
    shape::Shape,
    stroke_geometry::StrokeGeometry,
    stroke_point::StrokePoint,
    weights::{WEIGHT_COUNT, Weights},
};

/// Leaf features, the two penalties, the group features, then the merge penalty.
const _: () = assert!(
    WEIGHT_COUNT == LEAF_FEATURE_COUNT + 2 + GROUP_FEATURE_COUNT + 1,
    "weight count and feature cont do not match"
);

pub trait AssignmentFeatures {
    fn assignment_features(
        &self,
        user_stroke_order: &[u8],
        reference_shapes: &[Shape],
        reference_strokes: &[Vec<StrokePoint>],
        user_shapes: &[Shape],
        user_stroke_geometries: &[StrokeGeometry],
        weights: &Weights,
    ) -> Option<[f64; WEIGHT_COUNT]>;

    fn assignment_score(
        &self,
        user_stroke_order: &[u8],
        reference_shapes: &[Shape],
        reference_strokes: &[Vec<StrokePoint>],
        user_shapes: &[Shape],
        user_stroke_geometries: &[StrokeGeometry],
        weights: &Weights,
    ) -> Option<f64>;
}

impl AssignmentFeatures for AnalyzedKanjiNode {
    #[inline]
    fn assignment_features(
        &self,
        user_stroke_order: &[u8],
        reference_shapes: &[Shape],
        reference_strokes: &[Vec<StrokePoint>],
        user_shapes: &[Shape],
        user_stroke_geometries: &[StrokeGeometry],
        weights: &Weights,
    ) -> Option<[f64; WEIGHT_COUNT]> {
        let mut features = [0.0_f64; WEIGHT_COUNT];
        accumulate_leaves(
            user_stroke_order,
            reference_shapes,
            reference_strokes,
            user_shapes,
            &mut features,
        )?;
        accumulate_extras(user_stroke_order, user_shapes.len(), &mut features);
        accumulate_groups(
            self,
            &scoring_order(user_stroke_order),
            user_stroke_geometries,
            user_shapes,
            weights,
            &mut features,
            true,
        );
        Some(features)
    }

    #[inline]
    fn assignment_score(
        &self,
        user_stroke_order: &[u8],
        reference_shapes: &[Shape],
        reference_strokes: &[Vec<StrokePoint>],
        user_shapes: &[Shape],
        user_stroke_geometries: &[StrokeGeometry],
        weights: &Weights,
    ) -> Option<f64> {
        let features = self.assignment_features(
            user_stroke_order,
            reference_shapes,
            reference_strokes,
            user_shapes,
            user_stroke_geometries,
            weights,
        )?;
        Some(dot(&weights.to_array(), &features))
    }
}

#[must_use]
#[inline]
pub fn dot(weights: &[f64; WEIGHT_COUNT], features: &[f64; WEIGHT_COUNT]) -> f64 {
    weights
        .iter()
        .zip(features.iter())
        .map(|(weight, feature)| weight * feature)
        .sum()
}

fn accumulate_leaves(
    user_stroke_order: &[u8],
    reference_shapes: &[Shape],
    reference_strokes: &[Vec<StrokePoint>],
    user_shapes: &[Shape],
    features: &mut [f64; WEIGHT_COUNT],
) -> Option<()> {
    let mut position = 0_usize;
    while position < user_stroke_order.len() {
        let user_index = *user_stroke_order.get(position)?;
        if user_index == u8::MAX {
            add_at(features, LEAF_FEATURE_COUNT, 1.0);
            position = position.saturating_add(1);
            continue;
        }
        if user_index == FILLER {
            // A FILLER only ever trails the real index that opened its run; one reached on
            // its own means the run's length disagrees with where it starts.
            return None;
        }
        let mut length = 1_usize;
        while matches!(
            user_stroke_order.get(position.saturating_add(length)),
            Some(&FILLER)
        ) {
            length = length.saturating_add(1);
        }
        let drawn = user_shapes.get(usize::from(user_index))?;
        let reference = if length > 1 {
            joined_reference_shape(reference_strokes, position, length)?
        } else {
            *reference_shapes.get(position)?
        };
        if !reference.leaf_accepts(drawn) {
            return None;
        }
        for (slot, feature) in features
            .iter_mut()
            .zip(reference.leaf_features(drawn).iter())
        {
            *slot += feature;
        }
        if length > 1 {
            add_at(features, WEIGHT_COUNT.saturating_sub(1), 1.0);
        }
        position = position.saturating_add(length);
    }
    Some(())
}

fn accumulate_extras(
    user_stroke_order: &[u8],
    user_count: usize,
    features: &mut [f64; WEIGHT_COUNT],
) {
    let used = user_stroke_order
        .iter()
        .filter(|index| **index != u8::MAX && **index != FILLER)
        .count();
    let extras = user_count.saturating_sub(used);
    let slot = LEAF_FEATURE_COUNT.saturating_add(1);
    add_at(features, slot, extras.convert_lossy());
}
fn accumulate_groups(
    node: &AnalyzedKanjiNode,
    user_stroke_order: &[u8],
    user_stroke_geometries: &[StrokeGeometry],
    user_shapes: &[Shape],
    weights: &Weights,
    features: &mut [f64; WEIGHT_COUNT],
    root: bool,
) {
    let children = match node {
        AnalyzedKanjiNode::Group { children, .. } => children,
        AnalyzedKanjiNode::Stroke { .. } => return,
    };
    let own = node.group_features(
        user_stroke_order,
        user_stroke_geometries,
        user_shapes,
        weights,
    );
    let start = LEAF_FEATURE_COUNT.saturating_add(2);
    // Slot 4 (`absolute_position`) is root-only; every other slot, including slot 5
    // (`cross_group_bonus`), is charged at every level. See `group_score`'s own doc comment
    // for why: `absolute_position` is recomputed identically by every ancestor, but
    // `cross_group_bonus` only ever looks at this node's own direct children.
    for (offset, feature) in own
        .iter()
        .enumerate()
        .filter(|(index, _)| root || *index != 4)
    {
        add_at(features, start.saturating_add(offset), *feature);
    }
    let mut cursor = 0_usize;
    for child in children {
        let end = cursor.saturating_add(child.leaf_count());
        if let Some(slice) = user_stroke_order.get(cursor..end) {
            accumulate_groups(
                child,
                slice,
                user_stroke_geometries,
                user_shapes,
                weights,
                features,
                false,
            );
        }
        cursor = end;
    }
}

#[inline]
fn add_at(features: &mut [f64; WEIGHT_COUNT], index: usize, value: f64) {
    if let Some(slot) = features.get_mut(index) {
        *slot += value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        match_strokes::match_strokes,
        shape::ToShapes as _,
        stroke_point::{StrokePoint, to_stroke_points},
    };
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

    fn parts(
        tree: &AnalyzedKanjiNode,
        user: &[Vec<StrokePoint>],
    ) -> (
        Vec<Shape>,
        Vec<Vec<StrokePoint>>,
        Vec<Shape>,
        Vec<StrokeGeometry>,
    ) {
        let strokes = tree.collect_strokes();
        let reference = strokes.to_shapes();
        let shapes = crate::length_scale::user_shapes(&strokes, user);
        let geometries = user
            .iter()
            .map(|s| StrokeGeometry::from_stroke(s))
            .collect();
        (reference, strokes, shapes, geometries)
    }

    fn direct(
        tree: &AnalyzedKanjiNode,
        order: &[u8],
        user: &[Vec<StrokePoint>],
        weights: &Weights,
    ) -> Option<f64> {
        let (reference, strokes, shapes, geometries) = parts(tree, user);
        tree.assignment_score(order, &reference, &strokes, &shapes, &geometries, weights)
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn every_beam_result_matches_direct_scoring(
        tree: &AnalyzedKanjiNode,
        user: &[Vec<StrokePoint>],
        weights: &Weights,
    ) {
        let results = match_strokes(tree.clone(), user.to_vec(), *weights, 512);
        assert!(results.len() > 3, "too few candidates to be a real check");
        for result in &results {
            let order = result.user_stroke_order.as_slice();
            let actual = direct(tree, order, user, weights).expect("features");
            assert!(
                approx(actual, result.score),
                "{order:?}: direct {actual} vs beam {}",
                result.score
            );
        }
    }

    #[test]
    fn direct_scoring_agrees_with_the_beam_on_a_flat_group() {
        let user = vec![
            path(&horizontal(0.2)),
            path(&horizontal(0.5)),
            path(&horizontal(0.8)),
        ];
        every_beam_result_matches_direct_scoring(&three(), &user, &Weights::v1());
        every_beam_result_matches_direct_scoring(&three(), &user, &Weights::ones());
    }

    #[test]
    fn direct_scoring_agrees_with_the_beam_on_a_nested_tree() {
        let user = vec![
            path(&[(0.1, 0.1), (0.3, 0.1)]),
            path(&[(0.1, 0.3), (0.3, 0.3)]),
            path(&[(0.7, 0.1), (0.9, 0.1)]),
            path(&[(0.7, 0.3), (0.9, 0.3)]),
        ];
        every_beam_result_matches_direct_scoring(&nested(), &user, &Weights::v1());
    }

    #[test]
    fn the_search_offers_every_ordering_of_three_alike_strokes() {
        let user = vec![
            path(&horizontal(0.2)),
            path(&horizontal(0.5)),
            path(&horizontal(0.8)),
        ];
        let weights = Weights::v1();
        let returned = match_strokes(three(), user.clone(), weights, 512);
        let permutations = [
            [0_u8, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        for order in &permutations {
            assert!(
                returned
                    .iter()
                    .any(|r| r.user_stroke_order.as_slice() == order.as_slice()),
                "the search never offered {order:?}"
            );
            assert!(
                direct(&three(), order, &user, &weights).is_some(),
                "{order:?}"
            );
        }
    }

    #[test]
    fn direct_scoring_agrees_with_the_beam_when_a_stroke_is_spare() {
        let user = vec![
            path(&horizontal(0.2)),
            path(&horizontal(0.5)),
            path(&horizontal(0.8)),
            path(&[(0.5, 0.1), (0.5, 0.9)]),
        ];
        let weights = Weights::v1();
        every_beam_result_matches_direct_scoring(&three(), &user, &weights);
    }

    #[test]
    fn the_score_is_the_dot_product_of_weights_and_features() {
        let user = vec![
            path(&horizontal(0.8)),
            path(&horizontal(0.5)),
            path(&horizontal(0.2)),
        ];
        let (reference, strokes, shapes, geometries) = parts(&three(), &user);
        let features = three()
            .assignment_features(
                &[2, 1, 0],
                &reference,
                &strokes,
                &shapes,
                &geometries,
                &Weights::v1(),
            )
            .expect("features");
        for weights in [Weights::ones(), Weights::v1()] {
            let score = three()
                .assignment_score(
                    &[2, 1, 0],
                    &reference,
                    &strokes,
                    &shapes,
                    &geometries,
                    &weights,
                )
                .expect("score");
            assert!(approx(score, dot(&weights.to_array(), &features)));
        }
    }

    #[test]
    fn features_do_not_depend_on_the_weights() {
        let user = vec![
            path(&horizontal(0.8)),
            path(&horizontal(0.5)),
            path(&horizontal(0.2)),
        ];
        let (reference, strokes, shapes, geometries) = parts(&three(), &user);
        let first = three()
            .assignment_features(
                &[2, 1, 0],
                &reference,
                &strokes,
                &shapes,
                &geometries,
                &Weights::v1(),
            )
            .expect("features");
        let second = three()
            .assignment_features(
                &[2, 1, 0],
                &reference,
                &strokes,
                &shapes,
                &geometries,
                &Weights::v1(),
            )
            .expect("features");
        assert_eq!(first, second);
    }

    #[test]
    fn an_unusable_pair_makes_the_assignment_invalid() {
        let user = vec![
            path(&[(0.5, 0.5)]),
            path(&horizontal(0.5)),
            path(&horizontal(0.8)),
        ];
        let (reference, strokes, shapes, geometries) = parts(&three(), &user);
        assert!(
            three()
                .assignment_features(
                    &[0, 1, 2],
                    &reference,
                    &strokes,
                    &shapes,
                    &geometries,
                    &Weights::v1()
                )
                .is_none()
        );
    }

    #[test]
    fn undrawn_and_spare_strokes_land_in_their_own_slots() {
        let user = vec![path(&horizontal(0.2)), path(&[(0.5, 0.1), (0.5, 0.9)])];
        let (reference, strokes, shapes, geometries) = parts(&three(), &user);
        let features = three()
            .assignment_features(
                &[0, u8::MAX, u8::MAX],
                &reference,
                &strokes,
                &shapes,
                &geometries,
                &Weights::v1(),
            )
            .expect("features");
        assert!(approx(
            features.get(LEAF_FEATURE_COUNT).copied().unwrap_or(0.0),
            2.0
        ));
        assert!(approx(
            features
                .get(LEAF_FEATURE_COUNT.saturating_add(1))
                .copied()
                .unwrap_or(0.0),
            1.0
        ));
    }

    #[test]
    fn every_feature_is_non_negative() {
        let user = vec![
            path(&horizontal(0.8)),
            path(&horizontal(0.2)),
            path(&horizontal(0.5)),
        ];
        let (reference, strokes, shapes, geometries) = parts(&three(), &user);
        let features = three()
            .assignment_features(
                &[1, 2, 0],
                &reference,
                &strokes,
                &shapes,
                &geometries,
                &Weights::v1(),
            )
            .expect("features");
        assert!(features.iter().all(|feature| *feature >= 0.0));
    }

    #[test]
    fn a_merged_run_scores_the_joined_shape_and_charges_the_merge_penalty() {
        let user = vec![
            path(&[(0.2, 0.2), (0.8, 0.2), (0.2, 0.5), (0.8, 0.5)]),
            path(&horizontal(0.8)),
        ];
        let (reference, strokes, shapes, geometries) = parts(&three(), &user);
        let features = three()
            .assignment_features(
                &[0, FILLER, 1],
                &reference,
                &strokes,
                &shapes,
                &geometries,
                &Weights::v1(),
            )
            .expect("features");
        assert!(approx(
            features
                .get(WEIGHT_COUNT.saturating_sub(1))
                .copied()
                .unwrap_or(0.0),
            1.0
        ));
    }

    #[test]
    fn a_filler_with_no_run_leader_is_rejected() {
        let user = vec![path(&horizontal(0.2)), path(&horizontal(0.5))];
        let (reference, strokes, shapes, geometries) = parts(&three(), &user);
        assert!(
            three()
                .assignment_features(
                    &[FILLER, 0, u8::MAX],
                    &reference,
                    &strokes,
                    &shapes,
                    &geometries,
                    &Weights::v1(),
                )
                .is_none()
        );
    }
}
