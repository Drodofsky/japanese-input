use kurbo::{Point, Vec2};

use crate::{convert_lossy::ConvertLossy as _, stroke_point::StrokePoint};

pub type Centroid2D = Point;
pub trait Centroid {
    fn centroid(&self) -> Option<Centroid2D>;
}

impl Centroid for [StrokePoint] {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "data is normalized, division by zero is handled"
    )]
    #[inline]
    fn centroid(&self) -> Option<Centroid2D> {
        if self.is_empty() {
            None
        } else {
            let sum = self
                .iter()
                .map(|p| p.position.to_vec2())
                .fold(Vec2::ZERO, |acc, v| acc + v);
            Some((sum / self.len().convert_lossy()).to_point())
        }
    }
}
