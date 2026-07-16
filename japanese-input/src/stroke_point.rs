use kurbo::{BezPath, Point, Vec2};

use crate::{
    resample_path::{Params, resample_path},
    to_bez_path::ToBezPath as _,
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct StrokePoint {
    pub position: Point,
    pub tangent: Vec2,
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

impl ToStrokePoint for BezPath {
    #[inline]
    fn to_stroke_points(&self) -> Vec<StrokePoint> {
        const SPACING: f64 = 0.05;
        resample_path(self, &Params::from_step(SPACING)).unwrap_or_default()
    }
}

/// # Panics
/// Panics for non regular stroke (len = 1).
impl ToStrokePoint for Vec<(f32, f32)> {
    #[inline]
    fn to_stroke_points(&self) -> Vec<StrokePoint> {
        self.to_bez_path().to_stroke_points()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_regular_stroke() {
        let stroke = vec![(0.5f32, 0.5f32)].to_stroke_points();
        assert_eq!(stroke, Vec::new())
    }
}
