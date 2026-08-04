use kurbo::{Point, Rect};

use crate::{
    arc_len::ArcLen as _, bbox::BBox as _, centroid::Centroid as _, stroke_point::StrokePoint,
};

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct StrokeGeometry {
    pub bbox: Option<Rect>,
    pub centroid: Option<Point>,
    pub arc_len: f64,
}

impl StrokeGeometry {
    #[must_use]
    #[inline]
    pub fn from_stroke(stroke: &[StrokePoint]) -> Self {
        let bbox = stroke.bbox();
        let centroid = stroke.centroid();
        let arc_len = stroke.arc_len();
        Self {
            bbox,
            centroid,
            arc_len,
        }
    }
}
