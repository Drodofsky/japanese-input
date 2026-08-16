use kurbo::Vec2;

use crate::{
    shape::{HARMONICS, Shape},
    weights::Weights,
};

/// One direction feature, one sideways and one along-chord per harmonic, one length feature.
pub const LEAF_FEATURE_COUNT: usize = HARMONICS * 2 + 2;

pub trait LeafScore {
    /// Squared disagreements in this stroke's own chord frame, taking it as the reference.
    fn leaf_features(&self, user: &Self) -> [f64; LEAF_FEATURE_COUNT];

    /// The weighted cost of matching a user stroke here.
    fn leaf_cost(&self, user: &Self, weights: &Weights) -> Option<f64>;

    /// True when this pair has a computable cost at all.
    fn leaf_accepts(&self, user: &Self) -> bool;
}

impl LeafScore for Shape {
    #[inline]
    fn leaf_features(&self, user: &Self) -> [f64; LEAF_FEATURE_COUNT] {
        let frame = self.chord_frame();
        let mut splits = [(0.0_f64, 0.0_f64); HARMONICS];
        let pairs = self.harmonics.iter().zip(user.harmonics.iter());
        for (slot, (expected, drawn)) in splits.iter_mut().zip(pairs) {
            *slot = split(difference(*expected, *drawn), frame);
        }
        let mut features = [0.0_f64; LEAF_FEATURE_COUNT];
        let mut slot = features.iter_mut();
        let mean_gap = difference(self.mean, user.mean);
        write_next(&mut slot, mean_gap.dot(mean_gap));
        for &(sideways, _) in &splits {
            write_next(&mut slot, sideways);
        }
        for &(_, along) in &splits {
            write_next(&mut slot, along);
        }
        write_next(&mut slot, (self.ln_arc_len - user.ln_arc_len).powi(2));
        features
    }

    #[inline]
    fn leaf_accepts(&self, user: &Self) -> bool {
        accepts(self, user)
    }

    #[inline]
    fn leaf_cost(&self, user: &Self, weights: &Weights) -> Option<f64> {
        if !accepts(self, user) {
            return None;
        }
        let cost: f64 = weights
            .leaf()
            .iter()
            .zip(self.leaf_features(user).iter())
            .map(|(weight, feature)| weight * feature)
            .sum();
        (cost.is_finite() && cost >= 0.0).then_some(cost)
    }
}

fn accepts(reference: &Shape, user: &Shape) -> bool {
    reference.is_usable() && user.is_usable()
}

#[inline]
fn split(gap: Vec2, frame: Option<(Vec2, Vec2)>) -> (f64, f64) {
    match frame {
        Some((forward, sideways)) => (gap.dot(sideways).powi(2), gap.dot(forward).powi(2)),
        None => (gap.dot(gap), 0.0),
    }
}

#[inline]
fn difference(left: Vec2, right: Vec2) -> Vec2 {
    Vec2::new(left.x - right.x, left.y - right.y)
}

#[inline]
fn write_next<'slot>(slots: &mut impl Iterator<Item = &'slot mut f64>, value: f64) {
    if let Some(slot) = slots.next() {
        *slot = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        shape::ToShape as _,
        stroke_point::{StrokePoint, to_stroke_points},
    };
    use kurbo::Point;

    fn shape(points: &[(f64, f64)]) -> Shape {
        let stroke: Vec<StrokePoint> =
            to_stroke_points(points.iter().map(|&(x, y)| Point::new(x, y)));
        stroke.to_shape()
    }

    fn approx(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() < tolerance
    }

    fn otsu() -> Shape {
        shape(&[
            (25.0, 20.0),
            (55.0, 21.0),
            (75.0, 26.0),
            (58.0, 48.0),
            (35.0, 72.0),
            (48.0, 84.0),
            (80.0, 82.0),
        ])
    }

    fn sideways_total(reference: &Shape, user: &Shape) -> f64 {
        reference
            .leaf_features(user)
            .get(1..HARMONICS.saturating_add(1))
            .unwrap_or(&[])
            .iter()
            .sum()
    }

    fn along_total(reference: &Shape, user: &Shape) -> f64 {
        let start = HARMONICS.saturating_add(1);
        reference
            .leaf_features(user)
            .get(start..start.saturating_add(HARMONICS))
            .unwrap_or(&[])
            .iter()
            .sum()
    }

    #[test]
    fn a_stroke_costs_nothing_against_itself() {
        let s = otsu();
        let cost = s.leaf_cost(&s, &Weights::v1()).expect("cost");
        assert!(approx(cost, 0.0, 1e-12), "{cost}");
    }

    #[test]
    fn a_right_angle_of_aim_costs_two_under_unit_weights() {
        let horizontal = shape(&[(0.0, 0.0), (1.0, 0.0)]);
        let vertical = shape(&[(0.0, 0.0), (1.0e-9, 1.0)]);
        let cost = horizontal
            .leaf_cost(&vertical, &Weights::ones())
            .expect("cost");
        assert!(approx(cost, 2.0, 1e-6), "{cost}");
    }

    #[test]
    fn a_straight_stroke_has_no_harmonic_features() {
        let short = shape(&[(0.0, 0.0), (0.5, 0.0)]);
        let long = shape(&[(0.0, 0.0), (1.0, 0.0)]);
        let features = short.leaf_features(&long);
        assert!(approx(features.first().copied().unwrap_or(1.0), 0.0, 1e-12));
        assert!(approx(sideways_total(&short, &long), 0.0, 1e-12));
        assert!(approx(along_total(&short, &long), 0.0, 1e-12));
        assert!(approx(
            features.last().copied().unwrap_or(0.0),
            2.0_f64.ln().powi(2),
            1e-12
        ));
    }

    #[test]
    fn the_length_feature_treats_twice_and_half_alike() {
        let base = shape(&[(0.0, 0.0), (1.0, 0.0)]);
        let double = shape(&[(0.0, 0.0), (2.0, 0.0)]);
        let half = shape(&[(0.0, 0.0), (0.5, 0.0)]);
        assert!(approx(
            base.leaf_features(&double).last().copied().unwrap_or(0.0),
            base.leaf_features(&half).last().copied().unwrap_or(1.0),
            1e-12
        ));
    }

    #[test]
    fn mirroring_a_bow_costs_four_times_flattening_it_sideways() {
        let bowed = shape(&[(0.0, 0.0), (0.5, 0.2), (1.0, 0.0)]);
        let mirrored = shape(&[(0.0, 0.0), (0.5, -0.2), (1.0, 0.0)]);
        let flat = shape(&[(0.0, 0.0), (0.5, 0.0), (1.0, 0.0)]);
        let flattened = sideways_total(&bowed, &flat);
        assert!(flattened > 1e-6, "{flattened}");
        let ratio = sideways_total(&bowed, &mirrored) / flattened;
        assert!(approx(ratio, 4.0, 1e-9), "{ratio}");
    }

    #[test]
    fn a_bow_in_the_wrong_place_lands_on_the_along_chord_channel() {
        let centered = shape(&[(0.0, 0.0), (0.5, 0.2), (1.0, 0.0)]);
        let late = shape(&[(0.0, 0.0), (0.75, 0.2), (1.0, 0.0)]);
        assert!(along_total(&centered, &late) > 1e-4);
    }

    #[test]
    fn severity_ordering_holds_on_a_wandering_stroke() {
        let reference = otsu();
        let shaky = shape(&[
            (25.0, 20.0),
            (56.0, 20.0),
            (74.0, 27.0),
            (59.0, 47.0),
            (34.0, 73.0),
            (49.0, 84.0),
            (80.0, 83.0),
        ]);
        let no_hook = shape(&[
            (25.0, 20.0),
            (55.0, 21.0),
            (75.0, 26.0),
            (58.0, 48.0),
            (40.0, 70.0),
            (52.0, 80.0),
            (60.0, 82.0),
        ]);
        let weights = Weights::v1();
        let shaky_cost = reference.leaf_cost(&shaky, &weights).expect("shaky");
        let no_hook_cost = reference.leaf_cost(&no_hook, &weights).expect("no hook");
        assert!(shaky_cost < 0.05, "{shaky_cost}");
        assert!(no_hook_cost > shaky_cost * 10.0, "{no_hook_cost}");
    }

    #[test]
    fn a_reversed_stroke_still_scores_but_costs_more_than_a_matched_one() {
        let forward = shape(&[(0.0, 0.0), (1.0, 0.0)]);
        let backward = shape(&[(1.0, 0.0), (0.0, 0.0)]);
        let matched = shape(&[(0.0, 0.0), (1.0, 0.0)]);
        let reversed_cost = forward.leaf_cost(&backward, &Weights::v1()).expect("cost");
        let matched_cost = forward.leaf_cost(&matched, &Weights::v1()).expect("cost");
        assert!(reversed_cost > matched_cost, "{reversed_cost} vs {matched_cost}");
    }

    #[test]
    fn a_line_against_a_wandering_stroke_still_scores_but_more_than_a_matched_one() {
        let line = shape(&[(0.0, 0.0), (100.0, 0.0)]);
        let weights = Weights::v1();
        let line_cost = otsu().leaf_cost(&line, &weights).expect("cost");
        let matched_cost = otsu().leaf_cost(&otsu(), &weights).expect("cost");
        assert!(line_cost > matched_cost, "{line_cost} vs {matched_cost}");
    }

    #[test]
    fn a_grossly_longer_stroke_still_scores_but_more_expensively() {
        let short = shape(&[(0.0, 0.0), (1.0, 0.0)]);
        let long = shape(&[(0.0, 0.0), (10.0, 0.0)]);
        let inside = shape(&[(0.0, 0.0), (2.9, 0.0)]);
        let long_cost = short.leaf_cost(&long, &Weights::v1()).expect("cost");
        let inside_cost = short.leaf_cost(&inside, &Weights::v1()).expect("cost");
        assert!(long_cost > inside_cost, "{long_cost} vs {inside_cost}");
    }

    #[test]
    fn an_unusable_stroke_is_never_matched() {
        let line = shape(&[(0.0, 0.0), (1.0, 0.0)]);
        let dot = shape(&[(0.5, 0.5)]);
        assert!(line.leaf_cost(&dot, &Weights::v1()).is_none());
        assert!(dot.leaf_cost(&line, &Weights::v1()).is_none());
    }

    #[test]
    fn a_doubled_back_reference_charges_everything_sideways() {
        let folded = shape(&[(0.0, 0.0), (1.0, 0.0), (0.02, 0.0)]);
        let other = shape(&[(0.0, 0.0), (1.0, 0.0), (0.02, 0.05)]);
        assert!(folded.chord_frame().is_none());
        assert!(approx(along_total(&folded, &other), 0.0, 1e-12));
        assert!(folded.leaf_features(&other).iter().all(|f| f.is_finite()));
    }

    #[test]
    fn zero_weights_silence_a_channel() {
        let bowed = shape(&[(0.0, 0.0), (0.5, 0.2), (1.0, 0.0)]);
        let mirrored = shape(&[(0.0, 0.0), (0.5, -0.2), (1.0, 0.0)]);
        let mut values = vec![0.0_f64; crate::weights::WEIGHT_COUNT];
        if let Some(slot) = values.first_mut() {
            *slot = 1.0;
        }
        let weights = Weights::try_from(values.as_slice()).expect("weights");
        let cost = bowed.leaf_cost(&mirrored, &weights).expect("cost");
        assert!(approx(cost, 0.0, 1e-12), "{cost}");
    }

    #[test]
    fn every_cost_is_non_negative() {
        let strokes = [
            shape(&[(0.0, 0.0), (1.0, 0.0)]),
            shape(&[(0.0, 0.0), (0.5, 0.2), (1.0, 0.0)]),
            otsu(),
        ];
        for reference in &strokes {
            for user in &strokes {
                if let Some(cost) = reference.leaf_cost(user, &Weights::v1()) {
                    assert!(cost >= 0.0, "{cost}");
                }
            }
        }
    }
}
