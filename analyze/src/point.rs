use lyon_path::iterator::PathIterator;
use lyon_path::math::{Point, Vector, vector};
use lyon_path::{Path, PathEvent};
#[derive(Debug, Clone, Copy)]
pub struct OrientedPoint {
    pub position: Point,
    pub direction: Vector,
}

pub trait ToOriented {
    fn to_oriented(&self) -> Vec<OrientedPoint>;
}

impl ToOriented for &[Point] {
    fn to_oriented(&self) -> Vec<OrientedPoint> {
        to_oriented(self)
    }
}
impl ToOriented for &[(f32, f32)] {
    fn to_oriented(&self) -> Vec<OrientedPoint> {
        let points: Vec<Point> = self
            .iter()
            .map(|&(x, y)| lyon_path::math::point(x, y))
            .collect();
        points.as_slice().to_oriented()
    }
}

/// assumes a [0, 109] coordinate system
impl ToOriented for Path {
    fn to_oriented(&self) -> Vec<OrientedPoint> {
        let mut points: Vec<Point> = Vec::new();
        for evt in self.iter().flattened(0.1) {
            match evt {
                PathEvent::Begin { at } => points.push(at),
                PathEvent::Line { to, .. } => points.push(to),
                _ => {}
            }
        }
        points.as_slice().to_oriented()
    }
}

fn compute_directions(points: &[Point]) -> Vec<Vector> {
    let n = points.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![vector(0.0, 0.0)];
    }

    let mut dirs = Vec::with_capacity(n);
    dirs.push(unit(points[1] - points[0]));
    for i in 1..n - 1 {
        dirs.push(unit(points[i + 1] - points[i - 1]));
    }
    dirs.push(unit(points[n - 1] - points[n - 2]));
    dirs
}

fn to_oriented(points: &[Point]) -> Vec<OrientedPoint> {
    let dirs = compute_directions(points);
    points
        .iter()
        .zip(dirs.iter())
        .map(|(&p, &d)| OrientedPoint {
            position: p,
            direction: d,
        })
        .collect()
}

fn unit(v: Vector) -> Vector {
    let len = v.length();
    if len > f32::EPSILON {
        v / len
    } else {
        vector(0.0, 0.0)
    }
}

fn rdp_simplify(points: &[Point], epsilon: f32) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let (Some(&start), Some(&end)) = (points.first(), points.last()) else {
        return points.to_vec();
    };
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let denom_sq = dx * dx + dy * dy;
    if denom_sq == 0.0 {
        return vec![start, end];
    }

    // Find the point with the largest perpendicular distance from start→end.
    // Compare numerator² vs (epsilon * denom)² to avoid sqrt in the inner loop.
    let cross = end.x * start.y - end.y * start.x;
    let (mut max_num_sq, mut max_idx) = (0.0f32, 0);
    let inner = points.get(1..points.len().saturating_sub(1)).unwrap_or(&[]);
    for (i, &p) in inner.iter().enumerate() {
        let num = (dy * p.x - dx * p.y + cross).abs();
        let num_sq = num * num;
        if num_sq > max_num_sq {
            max_num_sq = num_sq;
            max_idx = i + 1;
        }
    }

    if max_num_sq > epsilon * epsilon * denom_sq {
        let mut left = rdp_simplify(&points[..=max_idx], epsilon);
        let right = rdp_simplify(&points[max_idx..], epsilon);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![start, end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lyon_path::math::point;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    fn vec_eq(a: Vector, b: Vector) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y)
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(compute_directions(&[]).is_empty());
    }

    #[test]
    fn single_point_returns_zero_direction() {
        let dirs = compute_directions(&[point(1.0, 2.0)]);
        assert_eq!(dirs.len(), 1);
        assert!(vec_eq(dirs[0], vector(0.0, 0.0)));
    }

    #[test]
    fn two_points_share_direction() {
        let pts = [point(0.0, 0.0), point(3.0, 4.0)];
        let dirs = compute_directions(&pts);
        let expected = vector(0.6, 0.8); // (3,4) normalized
        assert!(vec_eq(dirs[0], expected));
        assert!(vec_eq(dirs[1], expected));
    }

    #[test]
    fn horizontal_stroke_all_pointing_right() {
        let pts = [
            point(0.0, 0.0),
            point(1.0, 0.0),
            point(2.0, 0.0),
            point(3.0, 0.0),
        ];
        let dirs = compute_directions(&pts);
        for d in dirs {
            assert!(vec_eq(d, vector(1.0, 0.0)));
        }
    }

    #[test]
    fn corner_uses_central_difference() {
        // L-shape: right, then down. The corner point sees both neighbors.
        let pts = [point(0.0, 0.0), point(1.0, 0.0), point(1.0, 1.0)];
        let dirs = compute_directions(&pts);

        assert!(vec_eq(dirs[0], vector(1.0, 0.0))); // start: rightward
        assert!(vec_eq(dirs[2], vector(0.0, 1.0))); // end: downward
        // corner: from (0,0) to (1,1), normalized → (0.707, 0.707)
        let s = std::f32::consts::FRAC_1_SQRT_2;
        assert!(vec_eq(dirs[1], vector(s, s)));
    }

    #[test]
    fn duplicate_neighbors_yield_zero_direction() {
        let pts = [point(0.0, 0.0), point(0.0, 0.0), point(1.0, 0.0)];
        let dirs = compute_directions(&pts);
        // dirs[0] = unit(p1 - p0) = unit((0,0)) = zero
        assert!(vec_eq(dirs[0], vector(0.0, 0.0)));
        // dirs[1] = unit(p2 - p0) = unit((1,0)) = (1,0)
        assert!(vec_eq(dirs[1], vector(1.0, 0.0)));
        // dirs[2] = unit(p2 - p1) = unit((1,0)) = (1,0)
        assert!(vec_eq(dirs[2], vector(1.0, 0.0)));
    }

    #[test]
    fn to_oriented_preserves_positions() {
        let pts = [point(1.0, 2.0), point(3.0, 4.0), point(5.0, 6.0)];
        let oriented = pts.as_slice().to_oriented();
        assert_eq!(oriented.len(), 3);
        for (orig, op) in pts.iter().zip(oriented.iter()) {
            assert_eq!(op.position, *orig);
        }
    }
    #[test]
    fn cubic_curve_is_flattened_into_lines() {
        let mut b = Path::builder();
        b.begin(point(0.0, 0.0));
        b.cubic_bezier_to(point(0.0, 10.0), point(10.0, 10.0), point(10.0, 0.0));
        b.end(false);
        let path = b.build();

        // After flattening, curve becomes many small line segments.
        let oriented = path.to_oriented();
        assert!(oriented.len() > 2, "got {} points", oriented.len());

        // Some interior point should bulge upward (the curve goes up to y=10).
        let max_y = oriented
            .iter()
            .map(|p| p.position.y)
            .fold(0.0_f32, f32::max);
        assert!(max_y > 5.0, "curve should arc upward, max y was {}", max_y);
    }
    #[test]
    fn rdp_keeps_endpoints() {
        let pts = [point(0.0, 0.0), point(1.0, 0.0), point(2.0, 0.0)];
        let simplified = rdp_simplify(&pts, 0.1);
        assert_eq!(simplified.first(), Some(&point(0.0, 0.0)));
        assert_eq!(simplified.last(), Some(&point(2.0, 0.0)));
    }

    #[test]
    fn rdp_collapses_collinear_points() {
        let pts = [
            point(0.0, 0.0),
            point(1.0, 0.0),
            point(2.0, 0.0),
            point(3.0, 0.0),
        ];
        let simplified = rdp_simplify(&pts, 0.1);
        assert_eq!(simplified.len(), 2);
    }

    #[test]
    fn rdp_keeps_a_corner() {
        let pts = [point(0.0, 0.0), point(1.0, 0.0), point(1.0, 1.0)];
        let simplified = rdp_simplify(&pts, 0.1);
        assert_eq!(simplified.len(), 3);
    }

    #[test]
    fn rdp_drops_small_wobble() {
        // Mostly horizontal with a tiny bump — should be flattened away.
        let pts = [point(0.0, 0.0), point(1.0, 0.001), point(2.0, 0.0)];
        let simplified = rdp_simplify(&pts, 0.01);
        assert_eq!(simplified.len(), 2);
    }

    #[test]
    fn rdp_keeps_large_wobble() {
        let pts = [point(0.0, 0.0), point(1.0, 1.0), point(2.0, 0.0)];
        let simplified = rdp_simplify(&pts, 0.1);
        assert_eq!(simplified.len(), 3);
    }

    #[test]
    fn rdp_handles_degenerate_input() {
        let one = [point(5.0, 5.0)];
        assert_eq!(rdp_simplify(&one, 0.1), one.to_vec());
        let empty: [Point; 0] = [];
        assert_eq!(rdp_simplify(&empty, 0.1), Vec::<Point>::new());
    }

    #[test]
    fn rdp_handles_coincident_endpoints() {
        // Closed loop where start == end.
        let pts = [
            point(0.0, 0.0),
            point(1.0, 1.0),
            point(2.0, 0.0),
            point(0.0, 0.0),
        ];
        let simplified = rdp_simplify(&pts, 0.1);
        assert!(simplified.len() >= 2);
    }

    #[test]
    fn path_to_oriented_simplifies_dense_line() {
        // A long straight line — flattening produces many points, RDP should
        // collapse them all into just the endpoints.
        let mut b = Path::builder();
        b.begin(point(0.0, 0.0));
        b.line_to(point(100.0, 0.0));
        b.end(false);
        let path = b.build();

        let oriented = path.to_oriented();
        assert_eq!(oriented.len(), 2, "got {} points", oriented.len());
    }

    #[test]
    fn path_to_oriented_directions_point_along_stroke() {
        let mut b = Path::builder();
        b.begin(point(0.0, 0.0));
        b.line_to(point(100.0, 0.0));
        b.end(false);
        let path = b.build();

        let oriented = path.to_oriented();
        // All directions should point right.
        for op in &oriented {
            assert!(vec_eq(op.direction, vector(1.0, 0.0)));
        }
    }

    #[test]
    fn path_to_oriented_curve_directions_rotate() {
        // A quarter circle from (0,0) curving up to (10,10).
        let mut b = Path::builder();
        b.begin(point(0.0, 0.0));
        b.quadratic_bezier_to(point(10.0, 0.0), point(10.0, 10.0));
        b.end(false);
        let path = b.build();

        let oriented = path.to_oriented();
        assert!(oriented.len() >= 2);
        // First direction should have positive x component (going right initially).
        assert!(oriented.first().unwrap().direction.x > 0.5);
        // Last direction should have positive y component (going down/up at end).
        assert!(oriented.last().unwrap().direction.y.abs() > 0.5);
    }
}
