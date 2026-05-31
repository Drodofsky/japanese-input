use crate::bbox::BBox as _;
use crate::stroke_point::StrokePoint;
use kurbo::Point;

pub trait Normalize {
    #[must_use]
    fn normalized(&self) -> Self;
}

fn transform_point(p: &StrokePoint, center: Point, scale: f64) -> StrokePoint {
    StrokePoint {
        position: Point::new(
            (p.position.x - center.x) * scale,
            (p.position.y - center.y) * scale,
        ),
        tangent: p.tangent,
        curvature: p.curvature / scale,
    }
}

fn normalize_with(points: &[StrokePoint], rect: kurbo::Rect) -> Vec<StrokePoint> {
    let center = rect.center();
    let extent = rect.width().max(rect.height());
    let scale = if extent > f64::EPSILON {
        1.0_f64 / extent
    } else {
        1.0_f64
    };
    points
        .iter()
        .map(|p| transform_point(p, center, scale))
        .collect()
}

impl Normalize for Vec<StrokePoint> {
    #[inline]
    fn normalized(&self) -> Self {
        match self.bbox() {
            Some(rect) => normalize_with(self, rect),
            None => Vec::new(),
        }
    }
}

impl Normalize for Vec<Vec<StrokePoint>> {
    #[inline]
    fn normalized(&self) -> Self {
        match self.bbox() {
            Some(rect) => self
                .iter()
                .map(|stroke| normalize_with(stroke, rect))
                .collect(),
            None => Vec::new(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bbox::BBox;
    use kurbo::Vec2;

    fn sp(x: f64, y: f64) -> StrokePoint {
        StrokePoint {
            position: Point::new(x, y),
            tangent: Vec2::new(1.0, 0.0),
            curvature: 0.0,
        }
    }
    fn sp_c(x: f64, y: f64, curv: f64) -> StrokePoint {
        StrokePoint {
            position: Point::new(x, y),
            tangent: Vec2::new(1.0, 0.0),
            curvature: curv,
        }
    }
    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn square_fills_both_axes_centered() {
        let stroke = vec![sp(0.0, 0.0), sp(1.0, 0.0), sp(1.0, 1.0), sp(0.0, 1.0)];
        let r = stroke.normalized().bbox().unwrap();
        assert!(approx(r.min_x(), -0.5) && approx(r.max_x(), 0.5));
        assert!(approx(r.min_y(), -0.5) && approx(r.max_y(), 0.5));
    }

    #[test]
    fn tall_stroke_preserves_aspect_ratio() {
        let stroke = vec![sp(0.2, 0.0), sp(0.8, 0.0), sp(0.8, 0.3), sp(0.2, 0.3)];
        let r = stroke.normalized().bbox().unwrap();
        assert!(approx(r.min_x(), -0.5) && approx(r.max_x(), 0.5));
        assert!(approx(r.min_y(), -0.25) && approx(r.max_y(), 0.25));
    }

    #[test]
    fn tangent_is_unchanged_by_normalization() {
        let stroke = vec![sp(0.2, 0.2), sp(0.8, 0.8)];
        let n = stroke.normalized();
        for p in &n {
            assert!(approx(p.tangent.x, 1.0) && approx(p.tangent.y, 0.0));
        }
    }

    #[test]
    fn curvature_scales_inversely_with_size() {
        let stroke = vec![sp_c(0.0, 0.0, 4.0), sp_c(0.5, 0.5, 4.0)];
        let n = stroke.normalized();
        for p in &n {
            assert!(approx(p.curvature, 2.0), "curvature was {}", p.curvature);
        }
    }

    #[test]
    fn group_normalization_preserves_inter_stroke_position() {
        let group = vec![vec![sp(0.0, 0.0)], vec![sp(1.0, 1.0)]];
        let n = group.normalized();
        let a = n[0][0].position;
        let b = n[1][0].position;
        assert!(
            !(approx(a.x, b.x) && approx(a.y, b.y)),
            "strokes collapsed onto each other"
        );
        assert!(approx(a.x, -b.x) && approx(a.y, -b.y));
    }

    #[test]
    fn empty_yields_empty() {
        let stroke: Vec<StrokePoint> = vec![];
        assert!(stroke.normalized().is_empty());
        let group: Vec<Vec<StrokePoint>> = vec![];
        assert!(group.normalized().is_empty());
    }
}
