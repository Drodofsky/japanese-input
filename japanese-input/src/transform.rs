use kurbo::Affine;

use crate::stroke_point::StrokePoint;

pub trait Transform {
    type Output;
    #[must_use]
    fn transform(&self, t: Affine) -> Self::Output;
}

impl Transform for StrokePoint {
    type Output = StrokePoint;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "affine * point is float matrix math, no overflow/panic"
    )]
    #[inline]
    fn transform(&self, t: Affine) -> Self::Output {
        Self {
            position: t * self.position,
            displacement: self.displacement,
            curvature: self.curvature,
        }
    }
}
