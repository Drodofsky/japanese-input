use crate::stroke_point::StrokePoint;
use kurbo::Rect;

pub trait BBox {
    fn bbox(&self) -> Option<Rect>;
}

fn bbox_of<'iter>(points: impl Iterator<Item = &'iter StrokePoint>) -> Option<Rect> {
    let mut iter = points;
    let first = iter.next()?.position;
    let mut rect = Rect::from_points(first, first);
    for p in iter {
        rect = rect.union_pt(p.position);
    }
    Some(rect)
}

impl BBox for Vec<StrokePoint> {
    #[inline]
    fn bbox(&self) -> Option<Rect> {
        bbox_of(self.iter())
    }
}

impl BBox for Vec<Vec<StrokePoint>> {
    #[inline]
    fn bbox(&self) -> Option<Rect> {
        self.iter()
            .filter_map(BBox::bbox)
            .reduce(|acc, r| acc.union(r))
    }
}
impl BBox for &[Vec<StrokePoint>] {
    #[inline]
    fn bbox(&self) -> Option<Rect> {
        self.iter()
            .filter_map(BBox::bbox)
            .reduce(|acc, r| acc.union(r))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Point, Vec2};

    fn sp(x: f64, y: f64) -> StrokePoint {
        StrokePoint {
            position: Point::new(x, y),
            tangent: Vec2::new(1.0, 0.0),
            curvature: 0.0,
        }
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn bbox_of_a_rectangle_of_points() {
        let stroke = vec![sp(0.2, 0.1), sp(0.8, 0.1), sp(0.8, 0.5), sp(0.2, 0.5)];
        let r = stroke.bbox().unwrap();
        assert!(approx(r.min_x(), 0.2));
        assert!(approx(r.min_y(), 0.1));
        assert!(approx(r.max_x(), 0.8));
        assert!(approx(r.max_y(), 0.5));
    }

    #[test]
    fn single_point_is_zero_size_box() {
        let stroke = vec![sp(0.3, 0.7)];
        let r = stroke.bbox().unwrap();
        assert!(approx(r.width(), 0.0));
        assert!(approx(r.height(), 0.0));
        assert!(approx(r.center().x, 0.3) && approx(r.center().y, 0.7));
    }

    #[test]
    fn empty_stroke_has_no_bbox() {
        let stroke: Vec<StrokePoint> = vec![];
        assert!(stroke.bbox().is_none());
    }

    #[test]
    fn group_bbox_covers_all_strokes() {
        // two separated strokes; the group box must enclose both, not just one.
        let group = vec![
            vec![sp(0.0, 0.0), sp(0.2, 0.2)], // lower-left stroke
            vec![sp(0.6, 0.6), sp(0.9, 0.8)], // upper-right stroke
        ];
        let r = group.bbox().unwrap();
        assert!(approx(r.min_x(), 0.0));
        assert!(approx(r.min_y(), 0.0));
        assert!(approx(r.max_x(), 0.9));
        assert!(approx(r.max_y(), 0.8));
    }

    #[test]
    fn empty_group_has_no_bbox() {
        let group: Vec<Vec<StrokePoint>> = vec![];
        assert!(group.bbox().is_none());
        // also: a group of empty strokes has nothing to bound
        let group2: Vec<Vec<StrokePoint>> = vec![vec![], vec![]];
        assert!(group2.bbox().is_none());
    }
}
