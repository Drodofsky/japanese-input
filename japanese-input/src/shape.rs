use core::f64::consts::{PI, SQRT_2};

use kurbo::Vec2;

use crate::{arc_len::ArcLen as _, convert_lossy::ConvertLossy as _, stroke_point::StrokePoint};

/// Cosine harmonics kept per stroke; three is the finest detail a six-vertex polyline carries.
pub const HARMONICS: usize = 3;

/// Guards divisions by an arc length or a chord length.
const EPS: f64 = 1e-12;

/// Straightness below which a stroke has no usable direction and no chord frame.
pub const MIN_DIRECTION: f64 = 0.3;

/// Scale-invariant description of a stroke, built only from `displacement`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Shape {
    /// Chord divided by arc length: direction of travel and straightness in one vector.
    pub mean: Vec2,
    /// Cosine coefficients of the unit tangent field, coarse first.
    pub harmonics: [Vec2; HARMONICS],
    pub arc_len: f64,
    /// Cached so the length feature is a subtraction rather than a logarithm.
    pub ln_arc_len: f64,
}

impl Shape {
    #[must_use]
    #[inline]
    pub fn empty() -> Self {
        Self {
            mean: Vec2::ZERO,
            harmonics: [Vec2::ZERO; HARMONICS],
            arc_len: 0.0,
            ln_arc_len: 0.0,
        }
    }

    #[must_use]
    #[inline]
    pub fn is_usable(&self) -> bool {
        self.arc_len > EPS && self.arc_len.is_finite()
    }

    #[must_use]
    #[inline]
    pub fn straightness(&self) -> f64 {
        self.mean.hypot()
    }

    #[must_use]
    #[inline]
    pub fn direction(&self) -> Option<Vec2> {
        let length = self.straightness();
        if length < MIN_DIRECTION {
            return None;
        }
        Some(Vec2::new(self.mean.x / length, self.mean.y / length))
    }

    /// Forward and sideways axes of the stroke's own chord.
    #[must_use]
    #[inline]
    pub fn chord_frame(&self) -> Option<(Vec2, Vec2)> {
        let forward = self.direction()?;
        Some((forward, Vec2::new(-forward.y, forward.x)))
    }

    /// Signed bow relative to the chord: sign is the side, magnitude is how pronounced.
    #[must_use]
    #[inline]
    pub fn bulge(&self) -> f64 {
        match (self.chord_frame(), self.harmonics.first()) {
            (Some((_, sideways)), Some(first)) => first.dot(sideways),
            _ => 0.0,
        }
    }
}

pub trait ToShape {
    fn to_shape(&self) -> Shape;
}

pub trait ToShapes {
    fn to_shapes(&self) -> Vec<Shape>;
}

impl ToShape for [StrokePoint] {
    #[inline]
    fn to_shape(&self) -> Shape {
        shape_of(self)
    }
}

impl ToShape for Vec<StrokePoint> {
    #[inline]
    fn to_shape(&self) -> Shape {
        shape_of(self)
    }
}

impl<S: ToShape> ToShapes for [S] {
    #[inline]
    fn to_shapes(&self) -> Vec<Shape> {
        self.iter().map(ToShape::to_shape).collect()
    }
}

impl<S: ToShape> ToShapes for Vec<S> {
    #[inline]
    fn to_shapes(&self) -> Vec<Shape> {
        self.iter().map(ToShape::to_shape).collect()
    }
}

fn shape_of(points: &[StrokePoint]) -> Shape {
    let arc_len = points.arc_len();
    if arc_len <= EPS || !arc_len.is_finite() {
        return Shape::empty();
    }
    let mut mean = Vec2::ZERO;
    let mut harmonics = [Vec2::ZERO; HARMONICS];
    let mut span_start = 0.0_f64;
    for point in points.iter().skip(1) {
        let chord = point.displacement.hypot();
        if chord <= EPS || !chord.is_finite() {
            continue;
        }
        let span_end = span_start + chord / arc_len;
        let tangent = Vec2::new(point.displacement.x / chord, point.displacement.y / chord);
        mean = add_scaled(mean, tangent, span_end - span_start);
        for (order, harmonic) in harmonics.iter_mut().enumerate() {
            *harmonic = add_scaled(
                *harmonic,
                tangent,
                cosine_weight(order, span_start, span_end),
            );
        }
        span_start = span_end;
    }
    Shape {
        mean,
        harmonics,
        arc_len,
        ln_arc_len: arc_len.ln(),
    }
}

#[inline]
fn add_scaled(accumulator: Vec2, tangent: Vec2, scale: f64) -> Vec2 {
    Vec2::new(
        tangent.x.mul_add(scale, accumulator.x),
        tangent.y.mul_add(scale, accumulator.y),
    )
}

#[inline]
fn cosine_weight(order: usize, span_start: f64, span_end: f64) -> f64 {
    let wave = order.saturating_add(1).convert_lossy() * PI;
    if wave <= EPS {
        return 0.0;
    }
    SQRT_2 * ((wave * span_end).sin() - (wave * span_start).sin()) / wave
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke_point::to_stroke_points;
    use kurbo::Point;

    fn shape(points: &[(f64, f64)]) -> Shape {
        to_stroke_points(points.iter().map(|&(x, y)| Point::new(x, y))).to_shape()
    }

    fn approx(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() < tolerance
    }

    fn otsu() -> Vec<(f64, f64)> {
        vec![
            (25.0, 20.0),
            (55.0, 21.0),
            (75.0, 26.0),
            (58.0, 48.0),
            (35.0, 72.0),
            (48.0, 84.0),
            (80.0, 82.0),
        ]
    }

    #[test]
    fn empty_and_single_point_strokes_are_unusable() {
        assert!(!shape(&[]).is_usable());
        assert!(!shape(&[(0.5, 0.5)]).is_usable());
        assert!(!Shape::empty().is_usable());
    }

    #[test]
    fn repeated_points_are_unusable() {
        assert!(!shape(&[(0.4, 0.4), (0.4, 0.4), (0.4, 0.4)]).is_usable());
    }

    #[test]
    fn an_unusable_shape_holds_no_infinities() {
        let s = shape(&[(0.5, 0.5)]);
        assert!(s.arc_len.is_finite() && s.ln_arc_len.is_finite());
        assert!(approx(s.bulge(), 0.0, 1e-12));
    }

    #[test]
    fn a_straight_stroke_is_fully_straight_with_no_harmonics() {
        let s = shape(&[(0.0, 0.0), (0.3, 0.0), (1.0, 0.0)]);
        assert!(approx(s.straightness(), 1.0, 1e-12));
        for harmonic in &s.harmonics {
            assert!(approx(harmonic.hypot(), 0.0, 1e-12), "{harmonic:?}");
        }
    }

    #[test]
    fn direction_points_along_travel() {
        let d = shape(&[(0.0, 0.0), (0.0, 1.0)])
            .direction()
            .expect("direction");
        assert!(approx(d.x, 0.0, 1e-12) && approx(d.y, 1.0, 1e-12));
    }

    #[test]
    fn extra_points_on_a_straight_run_change_nothing() {
        let coarse = shape(&[(0.0, 0.0), (1.0, 0.0)]);
        let fine = shape(&[(0.0, 0.0), (0.1, 0.0), (0.4, 0.0), (1.0, 0.0)]);
        assert!(approx(coarse.mean.x, fine.mean.x, 1e-12));
        for (a, b) in coarse.harmonics.iter().zip(fine.harmonics.iter()) {
            assert!(approx(a.hypot(), b.hypot(), 1e-12));
        }
    }

    #[test]
    fn scaling_a_stroke_leaves_the_descriptor_alone() {
        let small = shape(&otsu());
        let scaled: Vec<(f64, f64)> = otsu().iter().map(|&(x, y)| (x * 7.0, y * 7.0)).collect();
        let large = shape(&scaled);
        assert!(approx(small.straightness(), large.straightness(), 1e-9));
        for (a, b) in small.harmonics.iter().zip(large.harmonics.iter()) {
            assert!(approx(a.x, b.x, 1e-9) && approx(a.y, b.y, 1e-9));
        }
    }

    #[test]
    fn a_wandering_stroke_is_told_from_a_line_by_straightness() {
        let s = shape(&otsu());
        assert!(s.straightness() > MIN_DIRECTION, "{}", s.straightness());
        assert!(s.straightness() < 0.7, "{}", s.straightness());
    }

    #[test]
    fn the_out_and_back_signature_lives_in_the_second_harmonic() {
        let s = shape(&otsu());
        let first = s.harmonics.first().expect("first").hypot();
        let second = s.harmonics.get(1).expect("second").hypot();
        assert!(second > first * 5.0, "first {first}, second {second}");
    }

    #[test]
    fn a_bow_and_its_mirror_have_opposite_bulge() {
        let right = shape(&[(0.0, 0.0), (0.5, 0.2), (1.0, 0.0)]);
        let left = shape(&[(0.0, 0.0), (0.5, -0.2), (1.0, 0.0)]);
        assert!(approx(right.bulge(), -left.bulge(), 1e-12));
        assert!(right.bulge().abs() > 1e-3);
    }

    #[test]
    fn a_straight_stroke_has_no_bulge() {
        assert!(approx(shape(&[(0.0, 0.0), (1.0, 1.0)]).bulge(), 0.0, 1e-12));
    }

    #[test]
    fn a_doubled_back_stroke_has_no_chord_frame() {
        assert!(
            shape(&[(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)])
                .chord_frame()
                .is_none()
        );
    }

    #[test]
    fn arc_len_and_its_logarithm_agree() {
        let s = shape(&[(0.0, 0.0), (3.0, 4.0)]);
        assert!(approx(s.arc_len, 5.0, 1e-12));
        assert!(approx(s.ln_arc_len, 5.0_f64.ln(), 1e-12));
        assert!(s.is_usable());
    }

    #[test]
    fn a_slice_of_strokes_maps_to_shapes_in_order() {
        let strokes = vec![
            to_stroke_points([Point::new(0.0, 0.0), Point::new(1.0, 0.0)].into_iter()),
            to_stroke_points([Point::new(0.0, 0.0), Point::new(0.0, 2.0)].into_iter()),
        ];
        let shapes = strokes.to_shapes();
        assert_eq!(shapes.len(), 2);
        assert!(approx(shapes.first().expect("first").arc_len, 1.0, 1e-12));
        assert!(approx(shapes.get(1).expect("second").arc_len, 2.0, 1e-12));
    }
}
