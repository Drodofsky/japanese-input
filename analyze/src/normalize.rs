use crate::bbox::{BBox, GenBBox};
use crate::point::OrientedPoint;
use lyon_path::math::point;

pub trait Normalize {
    #[must_use]
    fn normalize(self) -> Self;
}

impl Normalize for Vec<OrientedPoint> {
    fn normalize(mut self) -> Self {
        let bbox = self.gen_bbox();
        normalize_in_place(&mut self, &bbox);
        self
    }
}

impl Normalize for Vec<Vec<OrientedPoint>> {
    fn normalize(mut self) -> Self {
        let bbox = self.gen_bbox();
        for stroke in &mut self {
            normalize_in_place(stroke, &bbox);
        }
        self
    }
}

fn normalize_in_place(stroke: &mut [OrientedPoint], bbox: &BBox) {
    let scale = bbox.width().max(bbox.height());
    if scale < f32::EPSILON {
        for op in stroke.iter_mut() {
            op.position = point(0.0, 0.0);
        }
        return;
    }

    // Square box centered on the bbox center, side length = scale.
    let cx = (bbox.min.x + bbox.max.x) * 0.5;
    let cy = (bbox.min.y + bbox.max.y) * 0.5;
    let sq_min_x = cx - scale * 0.5;
    let sq_min_y = cy - scale * 0.5;

    for op in stroke.iter_mut() {
        op.position.x = (op.position.x - sq_min_x) / scale;
        op.position.y = (op.position.y - sq_min_y) / scale;
        // direction untouched — uniform scaling preserves angles
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use lyon_path::math::{point, vector};

    fn op(x: f32, y: f32) -> OrientedPoint {
        OrientedPoint {
            position: point(x, y),
            direction: vector(1.0, 0.0),
        }
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn empty_stroke_normalizes_to_empty() {
        let v: Vec<OrientedPoint> = vec![];
        let n = v.normalize();
        assert!(n.is_empty());
    }

    #[test]
    fn single_point_normalizes_to_zero() {
        let v = vec![op(7.0, 9.0)];
        let n = v.normalize();
        assert_eq!(n[0].position, point(0.0, 0.0));
    }

    #[test]
    fn square_stroke_fills_unit_square() {
        // Square 10×10 from (0,0) to (10,10).
        let v = vec![op(0.0, 0.0), op(10.0, 0.0), op(10.0, 10.0), op(0.0, 10.0)];
        let n = v.normalize();
        assert!(approx(n[0].position.x, 0.0) && approx(n[0].position.y, 0.0));
        assert!(approx(n[1].position.x, 1.0) && approx(n[1].position.y, 0.0));
        assert!(approx(n[2].position.x, 1.0) && approx(n[2].position.y, 1.0));
        assert!(approx(n[3].position.x, 0.0) && approx(n[3].position.y, 1.0));
    }

    #[test]
    fn wide_stroke_centered_vertically() {
        // Horizontal line: width 10, height 0. Square box is 10×10.
        // The line sits at the vertical center of the square → y = 0.5.
        let v = vec![op(0.0, 5.0), op(10.0, 5.0)];
        let n = v.normalize();
        assert!(approx(n[0].position.x, 0.0) && approx(n[0].position.y, 0.5));
        assert!(approx(n[1].position.x, 1.0) && approx(n[1].position.y, 0.5));
    }

    #[test]
    fn tall_stroke_centered_horizontally() {
        // Vertical line: width 0, height 10. Square box is 10×10.
        let v = vec![op(5.0, 0.0), op(5.0, 10.0)];
        let n = v.normalize();
        assert!(approx(n[0].position.x, 0.5) && approx(n[0].position.y, 0.0));
        assert!(approx(n[1].position.x, 0.5) && approx(n[1].position.y, 1.0));
    }

    #[test]
    fn directions_preserved() {
        let v = vec![
            OrientedPoint {
                position: point(0.0, 0.0),
                direction: vector(0.6, 0.8),
            },
            OrientedPoint {
                position: point(10.0, 10.0),
                direction: vector(0.6, 0.8),
            },
        ];
        let n = v.normalize();
        assert!(approx(n[0].direction.x, 0.6) && approx(n[0].direction.y, 0.8));
        assert!(approx(n[1].direction.x, 0.6) && approx(n[1].direction.y, 0.8));
    }

    #[test]
    fn kanji_normalize_uses_global_bbox() {
        // Two strokes; kanji bbox is (0,0)..(10,10).
        // Stroke 1 occupies the top-left corner; stroke 2 the bottom-right.
        let k = vec![
            vec![op(0.0, 0.0), op(2.0, 2.0)],
            vec![op(8.0, 8.0), op(10.0, 10.0)],
        ];
        let n = k.normalize();
        assert!(approx(n[0][0].position.x, 0.0) && approx(n[0][0].position.y, 0.0));
        assert!(approx(n[0][1].position.x, 0.2) && approx(n[0][1].position.y, 0.2));
        assert!(approx(n[1][0].position.x, 0.8) && approx(n[1][0].position.y, 0.8));
        assert!(approx(n[1][1].position.x, 1.0) && approx(n[1][1].position.y, 1.0));
    }

    #[test]
    fn kanji_with_non_square_bbox_centers_along_shorter_axis() {
        // Kanji bbox: width=10, height=4 → square box 10×10, vertically centered.
        // The bbox center y = 2, so vertical offset to square: -3.
        // A point at y=2 (the center) should land at y=0.5.
        let k = vec![vec![op(0.0, 2.0), op(10.0, 2.0)]];
        let n = k.normalize();
        assert!(approx(n[0][0].position.y, 0.5));
        assert!(approx(n[0][1].position.y, 0.5));
    }
}
