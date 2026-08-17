use kurbo::Rect;

use crate::{
    arc_len::ArcLen as _,
    bbox::BBox as _,
    shape::{Shape, ToShapes as _},
    stroke_point::StrokePoint,
};

/// Guards a division by a degenerate (zero-size) measurement.
const EPS: f64 = 1e-9;

/// The one place user strokes become shapes, so every caller applies the same correction.
#[must_use]
pub fn user_shapes(reference: &[Vec<StrokePoint>], user: &[Vec<StrokePoint>]) -> Vec<Shape> {
    rescale_lengths(user.to_shapes(), length_scale(reference, user))
}

/// Geometric mean of bbox and longest-stroke ratios, so one blind spot can't sink the estimate.
#[must_use]
pub fn length_scale(reference: &[Vec<StrokePoint>], user: &[Vec<StrokePoint>]) -> f64 {
    let bbox_scale = diagonal(user).zip(diagonal(reference)).map(|(u, r)| u / r);
    let longest_scale = longest(user).zip(longest(reference)).map(|(u, r)| u / r);
    let combined = match (bbox_scale, longest_scale) {
        (Some(b), Some(l)) if b.is_finite() && l.is_finite() && b > EPS && l > EPS => {
            Some((b * l).sqrt())
        }
        _ => None,
    };
    combined
        .filter(|scale| scale.is_finite() && *scale > EPS)
        .unwrap_or(1.0)
}

/// Only the length fields move; `mean`/`harmonics` are already scale-invariant.
fn rescale_lengths(shapes: Vec<Shape>, scale: f64) -> Vec<Shape> {
    if !scale.is_finite() || scale <= EPS {
        return shapes;
    }
    shapes
        .into_iter()
        .map(|shape| Shape {
            arc_len: shape.arc_len / scale,
            ln_arc_len: shape.ln_arc_len - scale.ln(),
            ..shape
        })
        .collect()
}

fn diagonal(strokes: &[Vec<StrokePoint>]) -> Option<f64> {
    let rect: Rect = strokes.bbox()?;
    Some(rect.width().hypot(rect.height()))
}

fn longest(strokes: &[Vec<StrokePoint>]) -> Option<f64> {
    strokes
        .iter()
        .map(|stroke| stroke.arc_len())
        .filter(|len| len.is_finite() && *len > EPS)
        .fold(None, |acc: Option<f64>, len| {
            Some(acc.map_or(len, |a| a.max(len)))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke_point::to_stroke_points;
    use kurbo::Point;

    fn stroke(points: &[(f64, f64)]) -> Vec<StrokePoint> {
        to_stroke_points(points.iter().map(|&(x, y)| Point::new(x, y)))
    }

    #[test]
    fn identical_drawings_scale_to_one() {
        let strokes = vec![
            stroke(&[(0.0, 0.0), (1.0, 0.0)]),
            stroke(&[(0.0, 1.0), (1.0, 1.0)]),
        ];
        assert!((length_scale(&strokes, &strokes) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_uniformly_smaller_drawing_gets_a_scale_below_one() {
        let reference = vec![
            stroke(&[(0.0, 0.0), (1.0, 0.0)]),
            stroke(&[(0.0, 1.0), (1.0, 1.0)]),
        ];
        let smaller = vec![
            stroke(&[(0.0, 0.0), (0.5, 0.0)]),
            stroke(&[(0.0, 0.5), (0.5, 0.5)]),
        ];
        let scale = length_scale(&reference, &smaller);
        assert!((scale - 0.5).abs() < 1e-9, "{scale}");
    }

    #[test]
    fn rescaling_shrinks_a_smaller_drawings_length_gap_to_the_reference() {
        let reference = vec![
            stroke(&[(0.0, 0.0), (1.0, 0.0)]),
            stroke(&[(0.0, 1.0), (1.0, 1.0)]),
        ];
        let smaller = vec![
            stroke(&[(0.0, 0.0), (0.5, 0.0)]),
            stroke(&[(0.0, 0.5), (0.5, 0.5)]),
        ];
        let corrected = user_shapes(&reference, &smaller);
        let plain = smaller.to_shapes();
        let reference_shapes = reference.to_shapes();
        let corrected_gap = (reference_shapes[0].ln_arc_len - corrected[0].ln_arc_len).abs();
        let plain_gap = (reference_shapes[0].ln_arc_len - plain[0].ln_arc_len).abs();
        assert!(corrected_gap < plain_gap, "{corrected_gap} vs {plain_gap}");
        assert!(corrected_gap < 1e-9, "{corrected_gap}");
    }

    #[test]
    fn no_usable_geometry_falls_back_to_no_correction() {
        let empty: Vec<Vec<StrokePoint>> = vec![];
        let reference = vec![stroke(&[(0.0, 0.0), (1.0, 0.0)])];
        assert!((length_scale(&reference, &empty) - 1.0).abs() < 1e-9);
        assert!((length_scale(&empty, &reference) - 1.0).abs() < 1e-9);
    }
}
