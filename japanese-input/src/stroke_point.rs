use kurbo::{
    BezPath, ParamCurve as _, ParamCurveArclen as _, ParamCurveCurvature as _,
    ParamCurveDeriv as _, PathSeg, Point, Vec2,
};

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct StrokePoint {
    pub position: Point,
    pub tangent: Vec2,
    pub curvature: f64,
}

pub trait ToStrokePoint {
    fn to_stroke_points(&self) -> Vec<StrokePoint>;
}

impl ToStrokePoint for BezPath {
    #[inline]
    fn to_stroke_points(&self) -> Vec<StrokePoint> {
        const SPACING: f64 = 0.01;
        sample_by_spacing(self, SPACING)
    }
}

fn sample_by_spacing(path: &BezPath, spacing: f64) -> Vec<StrokePoint> {
    const TOLERANCE: f64 = 1e-4;

    let segments: Vec<PathSeg> = path.segments().collect();
    let segment_lengths: Vec<f64> = segments.iter().map(|s| s.arclen(TOLERANCE)).collect();
    let total_length: f64 = segment_lengths.iter().sum();

    let mut stroke_points = Vec::new();
    let mut sampled_distance = 0.0_f64;
    while sampled_distance <= total_length {
        if let Some((segment, local_arclen)) = locate(&segments, &segment_lengths, sampled_distance)
        {
            let t = segment.inv_arclen(local_arclen, TOLERANCE);
            stroke_points.push(sample_seg(segment, t));
        }
        sampled_distance += spacing;
    }

    // Ensure the endpoint is always included.
    if let Some((segment, local_arclen)) = locate(&segments, &segment_lengths, total_length) {
        let t = segment.inv_arclen(local_arclen, TOLERANCE);
        stroke_points.push(sample_seg(segment, t));
    }
    stroke_points
}

fn locate(
    segments: &[PathSeg],
    segment_lengths: &[f64],
    sampled_distance: f64,
) -> Option<(PathSeg, f64)> {
    let mut distance = sampled_distance;
    for (seg, &len) in segments.iter().zip(segment_lengths) {
        if distance <= len {
            return Some((*seg, distance));
        }
        distance -= len;
    }
    Some((*segments.last()?, *segment_lengths.last()?))
}
fn sample_seg(segment: PathSeg, t: f64) -> StrokePoint {
    const EPS: f64 = 1e-12;

    let (position, velocity, curvature) = match segment {
        #[expect(clippy::arithmetic_side_effects, reason = "False positive")]
        PathSeg::Line(l) => (l.eval(t), l.p1 - l.p0, 0.0_f64),
        PathSeg::Quad(q) => (q.eval(t), q.deriv().eval(t).to_vec2(), q.curvature(t)),
        PathSeg::Cubic(c) => (c.eval(t), c.deriv().eval(t).to_vec2(), c.curvature(t)),
    };

    let speed = velocity.hypot();
    let (tangent, curvature) = if speed > EPS {
        (velocity.normalize(), curvature)
    } else {
        (Vec2::ZERO, 0.0_f64)
    };

    StrokePoint {
        position,
        tangent,
        curvature,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPACING: f64 = 0.01;

    fn line_path(from: (f64, f64), to: (f64, f64)) -> BezPath {
        let mut p = BezPath::new();
        p.move_to(from);
        p.line_to(to);
        p
    }

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    fn assert_all_finite(pts: &[StrokePoint]) {
        for (i, p) in pts.iter().enumerate() {
            assert!(p.position.x.is_finite(), "position.x not finite at {i}");
            assert!(p.position.y.is_finite(), "position.y not finite at {i}");
            assert!(p.tangent.x.is_finite(), "tangent.x not finite at {i}");
            assert!(p.tangent.y.is_finite(), "tangent.y not finite at {i}");
            assert!(p.curvature.is_finite(), "curvature not finite at {i}");
        }
    }

    fn assert_unit_tangents(pts: &[StrokePoint]) {
        for (i, p) in pts.iter().enumerate() {
            let len = p.tangent.hypot();
            assert!(
                approx(len, 1.0, 1e-6),
                "tangent not unit length ({len}) at {i}"
            );
        }
    }

    #[test]
    fn horizontal_line_basics() {
        let pts = sample_by_spacing(&line_path((0.0, 0.0), (1.0, 0.0)), SPACING);

        assert!(!pts.is_empty());
        assert_all_finite(&pts);
        assert_unit_tangents(&pts);

        for p in &pts {
            assert!(
                approx(p.curvature, 0.0, 1e-9),
                "curvature {} != 0",
                p.curvature
            );
            assert!(approx(p.tangent.x, 1.0, 1e-9));
            assert!(approx(p.tangent.y, 0.0, 1e-9));
            assert!(approx(p.position.y, 0.0, 1e-9));
        }

        let last = pts.last().unwrap();
        assert!(approx(last.position.x, 1.0, 1e-6));
    }

    #[test]
    fn diagonal_line_tangent_is_normalized() {
        let pts = sample_by_spacing(&line_path((0.0, 0.0), (1.0, 1.0)), SPACING);

        assert_unit_tangents(&pts);
        let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();
        for p in &pts {
            assert!(approx(p.tangent.x, inv_sqrt2, 1e-9));
            assert!(approx(p.tangent.y, inv_sqrt2, 1e-9));
            assert!(approx(p.curvature, 0.0, 1e-9));
        }
    }

    #[test]
    fn samples_are_evenly_spaced_in_arclen() {
        let pts = sample_by_spacing(&line_path((0.0, 0.0), (1.0, 0.0)), SPACING);

        for w in pts[..pts.len() - 1].windows(2) {
            let dx = (w[1].position.x - w[0].position.x).abs();
            assert!(approx(dx, SPACING, 1e-6), "gap {dx} != {SPACING}");
        }
    }

    #[test]
    fn curved_segment_has_nonzero_curvature() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.curve_to((0.25, 0.5), (0.75, 0.5), (1.0, 0.0));
        let pts = sample_by_spacing(&path, SPACING);

        assert_all_finite(&pts);
        assert_unit_tangents(&pts);
        let max_curv = pts.iter().map(|p| p.curvature.abs()).fold(0.0, f64::max);
        assert!(
            max_curv > 0.0,
            "expected some curvature on a curved segment"
        );
    }

    #[test]
    fn empty_path_yields_empty() {
        let pts = sample_by_spacing(&BezPath::new(), SPACING);
        assert!(pts.is_empty());
    }

    #[test]
    fn stroke_shorter_than_spacing_still_produces_points() {
        let pts = sample_by_spacing(&line_path((0.0, 0.0), (0.005, 0.0)), SPACING);
        assert!(!pts.is_empty(), "short stroke produced no samples");
        assert_all_finite(&pts);
    }

    #[test]
    fn endpoint_is_included() {
        let pts = sample_by_spacing(&line_path((0.0, 0.0), (1.0, 0.0)), SPACING);
        let last = pts.last().unwrap();
        assert!(approx(last.position.x, 1.0, 1e-6));
        assert!(approx(last.position.y, 0.0, 1e-9));
    }

    #[test]
    fn degenerate_collapsed_segment_stays_finite() {
        let mut path = BezPath::new();
        path.move_to((0.5, 0.5));
        path.curve_to((0.5, 0.5), (0.5, 0.5), (0.5, 0.5));
        let pts = sample_by_spacing(&path, SPACING);

        assert_all_finite(&pts);
    }
    #[test]
    fn multi_segment_l_shape() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((1.0, 0.0));
        path.line_to((1.0, 1.0));
        let pts = sample_by_spacing(&path, SPACING);

        assert!(!pts.is_empty());
        assert_all_finite(&pts);
        assert_unit_tangents(&pts);

        for p in &pts {
            let on_horizontal = approx(p.position.y, 0.0, 1e-6) && p.position.x <= 1.0 + 1e-6;
            let on_vertical = approx(p.position.x, 1.0, 1e-6) && p.position.y >= -1e-6;
            assert!(
                on_horizontal || on_vertical,
                "point {:?} is off the L",
                p.position
            );
        }

        let has_horizontal_dir = pts.iter().any(|p| approx(p.tangent.x, 1.0, 1e-6));
        let has_vertical_dir = pts.iter().any(|p| approx(p.tangent.y, 1.0, 1e-6));
        assert!(has_horizontal_dir, "no sample with horizontal tangent");
        assert!(has_vertical_dir, "no sample with vertical tangent");

        for p in &pts {
            assert!(
                approx(p.curvature, 0.0, 1e-9),
                "curvature {} != 0",
                p.curvature
            );
        }

        let last = pts.last().unwrap();
        assert!(approx(last.position.x, 1.0, 1e-6));
        assert!(approx(last.position.y, 1.0, 1e-6));

        assert!(
            pts.len() > 150,
            "too few samples ({}), second segment likely missed",
            pts.len()
        );
    }
}
