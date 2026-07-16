use kurbo::{
    BezPath, Point,
    simplify::{SimplifyOptions, simplify_bezpath},
};

use crate::{rdp::rdp, stroke_point::StrokePoint};

pub trait ToBezPath {
    fn to_bez_path(&self) -> BezPath;
}

pub trait ToBezPathVec {
    fn to_bez_path_vec(self) -> Vec<BezPath>;
}

/// # Panics
/// Panics for non regular stroke (len = 1).
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

/// # Panics
/// Panics for non regular stroke (len = 1).
impl ToBezPath for [StrokePoint] {
    #[inline]
    fn to_bez_path(&self) -> BezPath {
        fit_points_to_bez_path(self.iter().map(|p| p.position))
    }
}

/// # Panics
/// Panics for non regular stroke (len = 1).
impl ToBezPath for &Vec<StrokePoint> {
    #[inline]
    fn to_bez_path(&self) -> BezPath {
        fit_points_to_bez_path(self.iter().map(|p| p.position))
    }
}

/// # Panics
/// Panics for non regular stroke (len = 1).
impl ToBezPath for [(f32, f32)] {
    #[inline]
    fn to_bez_path(&self) -> BezPath {
        fit_points_to_bez_path(
            self.iter()
                .map(|(x, y)| Point::new((*x).into(), (*y).into())),
        )
    }
}

/// # Panics
/// Panics for non regular stroke (len = 1).
fn fit_points_to_bez_path(points: impl Iterator<Item = Point>) -> BezPath {
    const FIT_TOLERANCE: f64 = 0.02;
    const RDP_TOLERANCE: f64 = 0.005;
    let mut points = rdp(points, RDP_TOLERANCE);

    let mut polyline = BezPath::new();
    if let Some(first) = points.next() {
        polyline.move_to(first);
        for p in points {
            polyline.line_to(p);
        }
    }

    simplify_bezpath(polyline, FIT_TOLERANCE, &SimplifyOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::ParamCurveArclen as _;
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
            let _ = tx.send(JITTER_ZIGZAG.to_bez_path());
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
}
