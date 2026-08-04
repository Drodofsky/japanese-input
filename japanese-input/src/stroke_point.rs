use kurbo::{BezPath, ParamCurve as _, ParamCurveArclen as _, PathSeg, Point, Vec2};

use crate::{convert_lossy::ConvertLossy as _, rdp::rdp_slice};
const EPS: f64 = 1e-9;
pub const RDP_EPS: f64 = 0.03;

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct StrokePoint {
    pub position: Point,
    pub displacement: Vec2,
    pub curvature: Vec2,
}

impl StrokePoint {
    #[must_use]
    #[inline]
    pub fn new(position: Point, displacement: Vec2, curvature: Vec2) -> Self {
        Self {
            position,
            displacement,
            curvature,
        }
    }
}

#[inline]
#[must_use]
pub fn to_stroke_points(points: impl Iterator<Item = Point>) -> Vec<StrokePoint> {
    let mut out: Vec<StrokePoint> = Vec::new();
    let mut prev: Option<Point> = None;
    for p in points {
        let displacement = prev.map_or(Vec2::new(0.0, 0.0), |q| Vec2::new(p.x - q.x, p.y - q.y));
        out.push(StrokePoint::new(p, displacement, Vec2::new(1.0, 0.0)));
        prev = Some(p);
    }
    for i in 1..out.len().saturating_sub(1) {
        let (Some(vin), Some(vout)) = (
            out.get(i).map(|s| s.displacement),
            out.get(i.saturating_add(1)).map(|s| s.displacement),
        ) else {
            continue;
        };
        if let Some(slot) = out.get_mut(i) {
            slot.curvature = turn(vin, vout);
        }
    }
    out
}
#[inline]
fn turn(vin: Vec2, vout: Vec2) -> Vec2 {
    let (li, lo) = (vin.hypot(), vout.hypot());
    if li < EPS || lo < EPS {
        return Vec2::new(1.0, 0.0);
    }
    let inv = 1.0_f64 / (li * lo);
    Vec2::new(
        vin.x.mul_add(vout.x, vin.y * vout.y) * inv,
        vin.x.mul_add(vout.y, -(vin.y * vout.x)) * inv,
    )
}

pub trait ToStrokePoint {
    fn to_stroke_points(&self) -> Vec<StrokePoint>;
}
pub trait ToStrokeVector {
    fn to_stroke_vector(&self) -> Vec<Vec<StrokePoint>>;
}

impl<I: ToStrokePoint> ToStrokeVector for &[I] {
    #[inline]
    fn to_stroke_vector(&self) -> Vec<Vec<StrokePoint>> {
        self.iter()
            .map(ToStrokePoint::to_stroke_points)
            .collect::<Vec<Vec<_>>>()
    }
}
impl<I: ToStrokePoint> ToStrokeVector for Vec<I> {
    #[inline]
    fn to_stroke_vector(&self) -> Vec<Vec<StrokePoint>> {
        self.iter()
            .map(ToStrokePoint::to_stroke_points)
            .collect::<Vec<Vec<_>>>()
    }
}

#[must_use]
#[inline]
pub fn bez_to_point(bez: &BezPath) -> Vec<(f32, f32)> {
    const SPACING: f64 = 0.05;
    let points = sample_by_spacing(bez, SPACING);
    rdp_slice(&points, RDP_EPS)
        .map(|p| (p.x.convert_lossy(), p.y.convert_lossy()))
        .collect()
}

impl ToStrokePoint for BezPath {
    #[inline]
    fn to_stroke_points(&self) -> Vec<StrokePoint> {
        const SPACING: f64 = 0.05;
        let points = sample_by_spacing(self, SPACING);
        to_stroke_points(rdp_slice(&points, RDP_EPS))
    }
}

impl ToStrokePoint for Vec<(f32, f32)> {
    #[inline]
    fn to_stroke_points(&self) -> Vec<StrokePoint> {
        let points = rdp_slice(self, RDP_EPS);
        to_stroke_points(points)
    }
}

impl ToStrokePoint for &[(f32, f32)] {
    #[inline]
    fn to_stroke_points(&self) -> Vec<StrokePoint> {
        let points = rdp_slice(self, RDP_EPS);
        to_stroke_points(points)
    }
}

fn sample_by_spacing(path: &BezPath, spacing: f64) -> Vec<Point> {
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
fn sample_seg(segment: PathSeg, t: f64) -> Point {
    match segment {
        PathSeg::Line(l) => l.eval(t),
        PathSeg::Quad(q) => q.eval(t),
        PathSeg::Cubic(c) => c.eval(t),
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

    fn assert_all_finite(pts: &[Point]) {
        for (i, p) in pts.iter().enumerate() {
            assert!(p.x.is_finite(), "position.x not finite at {i}");
            assert!(p.y.is_finite(), "position.y not finite at {i}");
        }
    }

    fn features(pts: &[(f64, f64)]) -> Vec<StrokePoint> {
        to_stroke_points(pts.iter().map(|&(x, y)| Point::new(x, y)))
    }

    #[test]
    fn single_point_stroke_yields_one_neutral_point() {
        let stroke = vec![(0.5_f32, 0.5_f32)].to_stroke_points();
        assert_eq!(stroke.len(), 1);
        let p = stroke.first().expect("point");
        assert!(approx(p.displacement.hypot(), 0.0, 1e-12));
        assert!(approx(p.curvature.x, 1.0, 1e-12) && approx(p.curvature.y, 0.0, 1e-12));
    }

    #[test]
    fn empty_stroke_yields_nothing() {
        assert!(to_stroke_points(core::iter::empty()).is_empty());
        assert!(Vec::<(f32, f32)>::new().to_stroke_points().is_empty());
    }

    #[test]
    fn displacement_is_the_backward_difference() {
        let out = features(&[(0.0, 0.0), (0.3, 0.4), (0.3, 0.9)]);
        let d: Vec<Vec2> = out.iter().map(|s| s.displacement).collect();
        assert!(
            approx(d.first().expect("d0").hypot(), 0.0, 1e-12),
            "start has no edge"
        );
        let d1 = d.get(1).expect("d1");
        assert!(approx(d1.x, 0.3, 1e-12) && approx(d1.y, 0.4, 1e-12));
        let d2 = d.get(2).expect("d2");
        assert!(approx(d2.x, 0.0, 1e-12) && approx(d2.y, 0.5, 1e-12));
    }

    #[test]
    fn curvature_is_computed_at_interior_points() {
        let out = features(&[(0.0, 0.0), (0.5, 0.0), (1.0, 0.0), (1.0, 0.5), (1.0, 1.0)]);
        let corner = out.get(2).expect("corner").curvature;
        assert!(approx(corner.x, 0.0, 1e-12), "cos 90 = 0, got {}", corner.x);
        assert!(approx(corner.y, 1.0, 1e-12), "sin 90 = 1, got {}", corner.y);

        for i in [1_usize, 3] {
            let c = out.get(i).expect("straight").curvature;
            assert!(approx(c.x, 1.0, 1e-12) && approx(c.y, 0.0, 1e-12), "at {i}");
        }
    }

    #[test]
    fn curvature_at_the_endpoints_stays_neutral() {
        let out = features(&[(0.0, 0.0), (1.0, 0.0), (0.0, 0.1)]);
        for i in [0_usize, 2] {
            let c = out.get(i).expect("endpoint").curvature;
            assert!(approx(c.x, 1.0, 1e-12) && approx(c.y, 0.0, 1e-12), "at {i}");
        }
        assert!(
            out.get(1).expect("interior").curvature.x < 0.0,
            "interior did bend"
        );
    }

    #[test]
    fn left_and_right_turns_have_opposite_signs() {
        let right = features(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
        let left = features(&[(0.0, 0.0), (1.0, 0.0), (1.0, -1.0)]);
        let (Some(r), Some(l)) = (right.get(1), left.get(1)) else {
            panic!("expected an interior point");
        };
        assert!(approx(r.curvature.y, -l.curvature.y, 1e-12));
        assert!(
            r.curvature.y.abs() > 0.9,
            "a 90 degree turn should be near +/-1"
        );
    }

    #[test]
    fn curvature_is_always_unit_length() {
        let out = features(&[(0.0, 0.0), (0.3, 0.7), (0.9, 0.2), (1.0, 1.0), (0.1, 0.4)]);
        for (i, p) in out.iter().enumerate() {
            assert!(
                approx(p.curvature.hypot(), 1.0, 1e-9),
                "at {i}: {:?}",
                p.curvature
            );
        }
    }

    #[test]
    fn repeated_points_hit_the_degenerate_guard() {
        let out = features(&[(0.0, 0.0), (0.5, 0.5), (0.5, 0.5), (1.0, 0.0)]);
        let c = out.get(2).expect("duplicate").curvature;
        assert!(approx(c.x, 1.0, 1e-12) && approx(c.y, 0.0, 1e-12));
        assert!(
            approx(c.hypot(), 1.0, 1e-12),
            "guard must return a unit vector"
        );
    }

    #[test]
    fn horizontal_line_basics() {
        let pts = sample_by_spacing(&line_path((0.0, 0.0), (1.0, 0.0)), SPACING);

        assert!(!pts.is_empty());
        assert_all_finite(&pts);

        for p in &pts {
            assert!(approx(p.y, 0.0, 1e-9));
        }
        assert!(approx(pts.last().expect("last").x, 1.0, 1e-6));
    }

    #[test]
    fn samples_are_evenly_spaced_in_arclen() {
        let pts = sample_by_spacing(&line_path((0.0, 0.0), (1.0, 0.0)), SPACING);
        let grid: &[Point] = pts.split_last().map_or(&[], |(_, rest)| rest);
        for w in grid.windows(2) {
            let (Some(a), Some(b)) = (w.first(), w.last()) else {
                continue;
            };
            let dx = (b.x - a.x).abs();
            assert!(approx(dx, SPACING, 1e-6), "gap {dx} != {SPACING}");
        }
    }

    #[test]
    fn empty_path_yields_empty() {
        assert!(sample_by_spacing(&BezPath::new(), SPACING).is_empty());
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
        let last = pts.last().expect("last");
        assert!(approx(last.x, 1.0, 1e-6));
        assert!(approx(last.y, 0.0, 1e-9));
    }

    #[test]
    fn degenerate_collapsed_segment_stays_finite() {
        let mut path = BezPath::new();
        path.move_to((0.5, 0.5));
        path.curve_to((0.5, 0.5), (0.5, 0.5), (0.5, 0.5));
        assert_all_finite(&sample_by_spacing(&path, SPACING));
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

        for p in &pts {
            let on_horizontal = approx(p.y, 0.0, 1e-6) && p.x <= 1.0 + 1e-6;
            let on_vertical = approx(p.x, 1.0, 1e-6) && p.y >= -1e-6;
            assert!(on_horizontal || on_vertical, "point {p:?} is off the L");
        }

        let last = pts.last().expect("last");
        assert!(approx(last.x, 1.0, 1e-6));
        assert!(approx(last.y, 1.0, 1e-6));
        assert!(
            pts.len() > 150,
            "too few samples ({}), second segment likely missed",
            pts.len()
        );
    }

    #[test]
    fn bezpath_round_trip_produces_features() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((1.0, 0.0));
        path.line_to((1.0, 1.0));
        let out = path.to_stroke_points();
        assert!(out.len() >= 3, "RDP should keep the corner: {}", out.len());
        for p in &out {
            assert!(approx(p.curvature.hypot(), 1.0, 1e-9));
        }
    }
}
