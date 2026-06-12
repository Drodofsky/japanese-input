use kurbo::{Point, Rect};

use crate::{bbox::BBox as _, centroid::Centroid as _, stroke_point::StrokePoint};

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct StrokeGeometry {
    pub bbox: Option<Rect>,
    pub centroid: Option<Point>,
}

impl StrokeGeometry {
    #[must_use]
    #[inline]
    pub fn from_stroke(stroke: &[StrokePoint]) -> Self {
        let bbox = stroke.bbox();
        let centroid = stroke.centroid();
        Self { bbox, centroid }
    }
}
