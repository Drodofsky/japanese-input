use kurbo::{BezPath, Point, Vec2};

use crate::stroke_point::StrokePoint;

/// Guards divisions by a chord length.
const EPS: f64 = 1e-12;

/// Knot spacing exponent. 0.5 is the centripetal parameterization.
const ALPHA: f64 = 0.5;

/// Exponent on cos(φ/2) in the handle length. 1.0 is plain Catmull-Rom.
const SHARPNESS: f64 = 1.0;

const CORNER_RADIUS: f64 = 0.014;

pub trait ToBezPath {
    fn to_bez_path(&self) -> BezPath;
}

pub trait ToBezPathVec {
    fn to_bez_path_vec(self) -> Vec<BezPath>;
}

impl<T> ToBezPathVec for T
where
    T: Iterator,
    T::Item: ToBezPath,
{
    #[inline]
    fn to_bez_path_vec(self) -> Vec<BezPath> {
        self.map(|c| c.to_bez_path()).collect()
    }
}

impl ToBezPath for [StrokePoint] {
    #[inline]
    fn to_bez_path(&self) -> BezPath {
        to_bez_path(self)
    }
}

impl ToBezPath for &Vec<StrokePoint> {
    #[inline]
    fn to_bez_path(&self) -> BezPath {
        to_bez_path(self)
    }
}

/// The points that carry a real chord, so no knot spacing is zero.
fn live(pts: &[StrokePoint]) -> Vec<&StrokePoint> {
    pts.iter()
        .enumerate()
        .filter(|(i, p)| *i == 0 || p.displacement.hypot() > EPS)
        .map(|(_, p)| p)
        .collect()
}

/// Moves a point along a vector, avoiding the operator impls on `Point`.
#[inline]
fn shifted(from: Point, along: Vec2, scale: f64) -> Point {
    Point::new(
        along.x.mul_add(scale, from.x),
        along.y.mul_add(scale, from.y),
    )
}

/// Derivative of the Catmull-Rom spline at vertex `i`, in knot space.
fn velocity(v: &[&StrokePoint], d: &[f64], i: usize) -> Vec2 {
    let back = i
        .checked_sub(1)
        .and_then(|j| Some((v.get(i)?.displacement, *d.get(j)?)));
    let fwd = v
        .get(i.saturating_add(1))
        .map(|n| n.displacement)
        .zip(d.get(i).copied());

    match (back, fwd) {
        (Some((din, da)), Some((dout, db))) => {
            let span = da + db;
            Vec2::new(
                din.x / da - (din.x + dout.x) / span + dout.x / db,
                din.y / da - (din.y + dout.y) / span + dout.y / db,
            )
        }
        (None, Some((dout, db))) => Vec2::new(dout.x / db, dout.y / db),
        (Some((din, da)), None) => Vec2::new(din.x / da, din.y / da),
        (None, None) => Vec2::ZERO,
    }
}

/// Per-vertex handle scale, cos^(SHARPNESS−1)(φ/2), read off the turn rotor.
#[inline]
fn crispness(rotor: Vec2) -> f64 {
    ((1.0_f64 + rotor.x) * 0.5_f64)
        .max(0.0_f64)
        .powf(0.5_f64 * (SHARPNESS - 1.0_f64))
}

/// sin(φ/2) from the turn rotor.
#[inline]
fn sin_half(rotor: Vec2) -> f64 {
    ((1.0_f64 - rotor.x) * 0.5_f64).max(0.0_f64).sqrt()
}

/// Hold the corner's lateral offset near `CORNER_RADIUS`.
fn bound_handle(h: f64, sh: &[f64], i: usize, chord: f64) -> f64 {
    let s = sh.get(i).copied().unwrap_or(0.0_f64);
    let ceiling = if s > EPS {
        let before = i
            .checked_sub(1)
            .and_then(|j| sh.get(j))
            .copied()
            .unwrap_or(0.0_f64);
        let after = sh.get(i.saturating_add(1)).copied().unwrap_or(0.0_f64);
        (CORNER_RADIUS / s) * (1.0_f64 + (before + after) / s)
    } else {
        f64::INFINITY
    };
    h.min(ceiling).max(CORNER_RADIUS.min(chord / 3.0_f64))
}

/// Interpolate the stroke points with a centripetal Catmull-Rom spline.
///
/// Each segment takes its direction from the spline and its handle length from
/// the corner bound, so straight runs, gentle bows and hard corners all come out
/// of the same arithmetic.
#[must_use]
#[inline]
pub fn to_bez_path(pts: &[StrokePoint]) -> BezPath {
    let v = live(pts);
    let mut path = BezPath::new();
    let Some(first) = v.first() else {
        return path;
    };
    path.move_to(first.position);

    let d: Vec<f64> = v
        .windows(2)
        .filter_map(|w| w.get(1))
        .map(|p| p.displacement.hypot().powf(ALPHA))
        .collect();
    let crisp: Vec<f64> = v.iter().map(|p| crispness(p.curvature)).collect();
    let sh: Vec<f64> = v.iter().map(|p| sin_half(p.curvature)).collect();

    for (i, w) in v.windows(2).enumerate() {
        let [a, b] = *w else { continue };
        let next = i.saturating_add(1);
        let (Some(&span), Some(&c0), Some(&c1)) = (d.get(i), crisp.get(i), crisp.get(next)) else {
            continue;
        };
        let chord = b.displacement.hypot();
        let t0 = velocity(&v, &d, i);
        let t1 = velocity(&v, &d, next);
        let h0 = bound_handle(t0.hypot() * span * c0 / 3.0_f64, &sh, i, chord);
        let h1 = bound_handle(t1.hypot() * span * c1 / 3.0_f64, &sh, next, chord);
        let (n0, n1) = (t0.hypot().max(EPS), t1.hypot().max(EPS));
        path.curve_to(
            shifted(a.position, t0, h0 / n0),
            shifted(b.position, t1, -h1 / n1),
            b.position,
        );
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke_point::ToStrokePoint;
    use kurbo::ParamCurve as _;
    use kurbo::ParamCurveArclen as _;
    use kurbo::PathSeg;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    const JITTER_ZIGZAG: [(f32, f32); 6] = [
        (3.0 / 32.0, 1.0 / 32.0),
        (4.0 / 32.0, 2.0 / 32.0),
        (2.0 / 32.0, 2.0 / 32.0),
        (4.0 / 32.0, 5.0 / 32.0),
        (1.0 / 32.0, 0.0),
        (0.0, 7.0 / 32.0),
    ];

    #[test]
    fn fit_does_not_hang_on_jittery_stroke() {
        const TIMEOUT: Duration = Duration::from_secs(5);

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(JITTER_ZIGZAG.as_slice().to_stroke_points().to_bez_path());
        });

        let path = rx
            .recv_timeout(TIMEOUT)
            .expect("fit_points_to_bez_path hung on a jittery stroke");

        let total: f64 = path.segments().map(|s| s.arclen(1e-4)).sum();
        assert!(
            total.is_finite() && total < 2.0,
            "fitted path arclen {total} is not sane for a unit-square stroke"
        );
    }

    /// A repeated sample must not survive as a segment of its own, whatever
    /// simplification did to the rest of the stroke.
    #[test]
    fn duplicate_samples_are_dropped() {
        let cases = [
            vec![(0.0f32, 0.0), (0.5, 0.0), (0.5, 0.0), (1.0, 0.0)],
            vec![(0.0f32, 0.0), (0.5, 0.3), (0.5, 0.3), (1.0, 0.0)],
            vec![(0.2f32, 0.2), (0.2, 0.2), (0.8, 0.5)],
        ];
        for case in cases {
            let path = case.as_slice().to_stroke_points().to_bez_path();
            for segment in path.segments() {
                let length = segment.arclen(1e-6);
                assert!(
                    length > 1e-9,
                    "a repeated sample left a segment of length {length} in {case:?}"
                );
            }
        }
    }

    #[test]
    fn collinear_points_stay_on_the_line() {
        let pts = [(0.0f32, 0.0), (0.1, 0.0), (0.6, 0.0), (0.7, 0.0)]
            .as_slice()
            .to_stroke_points();
        let path = pts.to_bez_path();

        for seg in path.segments() {
            let PathSeg::Cubic(c) = seg else { continue };
            for p in [c.p0, c.p1, c.p2, c.p3] {
                assert!(p.y.abs() < 1e-9, "a straight run bowed to y = {}", p.y);
            }
        }
    }

    #[test]
    /// The handle is reported with the chord it belongs to, since simplification may
    /// have joined collinear samples into one longer segment.
    fn corner_radius_is_absolute_not_proportional() {
        let handle = |corner: (f32, f32), scale: f32| -> (f64, f64) {
            let pts = [
                (0.0f32, 0.0),
                (scale, 0.0),
                (corner.0 * scale, corner.1 * scale),
            ]
            .as_slice()
            .to_stroke_points();
            let path = pts.to_bez_path();
            let Some(PathSeg::Cubic(c)) = path.segments().next() else {
                panic!("expected a cubic")
            };
            ((c.p3 - c.p2).hypot(), (c.p3 - c.p0).hypot())
        };

        // A straight run is untouched: the floor never exceeds a third of the chord.
        let (straight, chord) = handle((2.0, 0.0), 0.3);
        assert!(
            (straight - chord / 3.0).abs() < 1e-9,
            "straight run: {straight} against a chord of {chord}"
        );

        // A right angle rounds by the same amount on a long stroke and a short
        // one, which is the whole point of bounding a length instead of a ratio.
        let (long, _) = handle((1.0, 1.0), 0.45);
        let (short, _) = handle((1.0, 1.0), 0.12);
        assert!(
            (long - short).abs() < 1e-9,
            "corner scaled with the stroke: {long} vs {short}"
        );
        let want = CORNER_RADIUS / (0.5f64).sqrt();
        assert!((long - want).abs() < 1e-9, "right angle: {long}");
    }

    /// Simplification runs before the spline, so the curve owes nothing to samples it
    /// removed; what it must hit is every point that survived.
    #[test]
    fn spline_passes_through_every_point() {
        let raw: Vec<(f32, f32)> = (0..=8)
            .map(|i| {
                let a = core::f32::consts::FRAC_PI_2 * (i as f32) / 8.0;
                (a.cos() * 0.5, a.sin() * 0.5)
            })
            .collect();
        let kept = raw.as_slice().to_stroke_points();
        assert!(kept.len() >= 3, "nothing left to check: {}", kept.len());
        let path = kept.to_bez_path();

        for (seg, point) in path.segments().zip(kept.iter().skip(1)) {
            let end = seg.eval(1.0);
            assert!(
                (end - point.position).hypot() < 1e-9,
                "spline missed a kept point at {end:?}"
            );
        }
    }
}
