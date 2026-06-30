use kurbo::{BezPath, Point, fit_to_bezpath, simplify::SimplifyBezPath};

use crate::stroke_point::StrokePoint;

pub trait ToBezPathPath {
    fn to_bez_path(&self) -> BezPath;
}

/// # Panics
/// Panics for non regular stroke (len = 1).
impl ToBezPathPath for [StrokePoint] {
    #[inline]
    fn to_bez_path(&self) -> BezPath {
        fit_points_to_bez_path(self.iter().map(|p| p.position))
    }
}

/// # Panics
/// Panics for non regular stroke (len = 1).
impl ToBezPathPath for [(f32, f32)] {
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
#[inline]
fn fit_points_to_bez_path(mut points: impl Iterator<Item = Point>) -> BezPath {
    const FIT_TOLERANCE: f64 = 0.02;

    let mut polyline = BezPath::new();
    if let Some(first) = points.next() {
        polyline.move_to(first);
        for p in points {
            polyline.line_to(p);
        }
    }

    let source = SimplifyBezPath::new(polyline);
    fit_to_bezpath(&source, FIT_TOLERANCE)
}
