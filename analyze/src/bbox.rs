use crate::point::OrientedPoint;
use lyon_path::math::{Point, point};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    pub min: Point,
    pub max: Point,
}

impl BBox {
    #[must_use]
    pub fn zero() -> Self {
        BBox {
            min: point(0.0, 0.0),
            max: point(0.0, 0.0),
        }
    }

    #[must_use]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }
    #[must_use]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }
}

pub trait GenBBox {
    fn gen_bbox(&self) -> BBox;
}

impl GenBBox for Vec<OrientedPoint> {
    fn gen_bbox(&self) -> BBox {
        let Some(first) = self.first() else {
            return BBox::zero();
        };
        let mut min = first.position;
        let mut max = first.position;
        for op in &self[1..] {
            min.x = min.x.min(op.position.x);
            min.y = min.y.min(op.position.y);
            max.x = max.x.max(op.position.x);
            max.y = max.y.max(op.position.y);
        }
        BBox { min, max }
    }
}

impl GenBBox for Vec<Vec<OrientedPoint>> {
    fn gen_bbox(&self) -> BBox {
        let mut iter = self.iter().filter(|s| !s.is_empty());
        let Some(first_stroke) = iter.next() else {
            return BBox::zero();
        };
        let mut bbox = first_stroke.gen_bbox();
        for stroke in iter {
            let b = stroke.gen_bbox();
            bbox.min.x = bbox.min.x.min(b.min.x);
            bbox.min.y = bbox.min.y.min(b.min.y);
            bbox.max.x = bbox.max.x.max(b.max.x);
            bbox.max.y = bbox.max.y.max(b.max.y);
        }
        bbox
    }
}
impl GenBBox for &[(f32, f32)] {
    fn gen_bbox(&self) -> BBox {
        let Some(&(fx, fy)) = self.first() else {
            return BBox::zero();
        };
        let mut min = point(fx, fy);
        let mut max = min;
        for &(x, y) in &self[1..] {
            min.x = min.x.min(x);
            min.y = min.y.min(y);
            max.x = max.x.max(x);
            max.y = max.y.max(y);
        }
        BBox { min, max }
    }
}

impl GenBBox for Vec<Vec<(f32, f32)>> {
    fn gen_bbox(&self) -> BBox {
        let mut iter = self.iter().filter(|s| !s.is_empty());
        let Some(first_stroke) = iter.next() else {
            return BBox::zero();
        };
        let mut bbox = first_stroke.as_slice().gen_bbox();
        for stroke in iter {
            let b = stroke.as_slice().gen_bbox();
            bbox.min.x = bbox.min.x.min(b.min.x);
            bbox.min.y = bbox.min.y.min(b.min.y);
            bbox.max.x = bbox.max.x.max(b.max.x);
            bbox.max.y = bbox.max.y.max(b.max.y);
        }
        bbox
    }
}
impl GenBBox for &[Vec<(f32, f32)>] {
    fn gen_bbox(&self) -> BBox {
        let mut iter = self.iter().filter(|s| !s.is_empty());
        let Some(first_stroke) = iter.next() else {
            return BBox::zero();
        };
        let mut bbox = first_stroke.as_slice().gen_bbox();
        for stroke in iter {
            let b = stroke.as_slice().gen_bbox();
            bbox.min.x = bbox.min.x.min(b.min.x);
            bbox.min.y = bbox.min.y.min(b.min.y);
            bbox.max.x = bbox.max.x.max(b.max.x);
            bbox.max.y = bbox.max.y.max(b.max.y);
        }
        bbox
    }
}
impl GenBBox for Vec<&Vec<(f32, f32)>> {
    fn gen_bbox(&self) -> BBox {
        let mut iter = self.iter().filter(|s| !s.is_empty());
        let Some(first_stroke) = iter.next() else {
            return BBox::zero();
        };
        let mut bbox = first_stroke.as_slice().gen_bbox();
        for stroke in iter {
            let b = stroke.as_slice().gen_bbox();
            bbox.min.x = bbox.min.x.min(b.min.x);
            bbox.min.y = bbox.min.y.min(b.min.y);
            bbox.max.x = bbox.max.x.max(b.max.x);
            bbox.max.y = bbox.max.y.max(b.max.y);
        }
        bbox
    }
}
impl BBox {
    #[must_use]
    pub fn center(&self) -> (f32, f32) {
        (
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lyon_path::math::vector;

    fn op(x: f32, y: f32) -> OrientedPoint {
        OrientedPoint {
            position: point(x, y),
            direction: vector(0.0, 0.0),
        }
    }

    #[test]
    fn empty_stroke_returns_zero_bbox() {
        let v: Vec<OrientedPoint> = vec![];
        assert_eq!(v.gen_bbox(), BBox::zero());
    }

    #[test]
    fn single_point_stroke_has_degenerate_bbox() {
        let v = vec![op(3.0, 4.0)];
        let b = v.gen_bbox();
        assert_eq!(b.min, point(3.0, 4.0));
        assert_eq!(b.max, point(3.0, 4.0));
        assert_eq!(b.width(), 0.0);
        assert_eq!(b.height(), 0.0);
    }

    #[test]
    fn multi_point_stroke_bbox() {
        let v = vec![op(1.0, 5.0), op(3.0, 2.0), op(2.0, 7.0)];
        let b = v.gen_bbox();
        assert_eq!(b.min, point(1.0, 2.0));
        assert_eq!(b.max, point(3.0, 7.0));
    }

    #[test]
    fn empty_kanji_returns_zero_bbox() {
        let k: Vec<Vec<OrientedPoint>> = vec![];
        assert_eq!(k.gen_bbox(), BBox::zero());
    }

    #[test]
    fn kanji_with_only_empty_strokes_returns_zero() {
        let k: Vec<Vec<OrientedPoint>> = vec![vec![], vec![]];
        assert_eq!(k.gen_bbox(), BBox::zero());
    }

    #[test]
    fn kanji_bbox_covers_all_strokes() {
        let k = vec![
            vec![op(1.0, 1.0), op(2.0, 2.0)],
            vec![op(0.0, 5.0), op(3.0, 3.0)],
        ];
        let b = k.gen_bbox();
        assert_eq!(b.min, point(0.0, 1.0));
        assert_eq!(b.max, point(3.0, 5.0));
    }

    #[test]
    fn kanji_bbox_skips_empty_strokes() {
        let k = vec![vec![], vec![op(2.0, 3.0), op(4.0, 5.0)], vec![]];
        let b = k.gen_bbox();
        assert_eq!(b.min, point(2.0, 3.0));
        assert_eq!(b.max, point(4.0, 5.0));
    }
}
