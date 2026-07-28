use core::mem::take;

use kurbo::{Line, ParamCurveNearest as _, Point};

#[inline]
pub fn rdp(points: impl Iterator<Item = Point>, tolerance: f64) -> impl Iterator<Item = Point> {
    Rdp::new(points.collect::<Vec<_>>(), tolerance)
}

#[inline]
pub fn rdp_slice(points: &[Point], tolerance: f64) -> impl Iterator<Item = Point> + '_ {
    Rdp::new(points, tolerance)
}

struct Rdp<S> {
    pts: S,
    stack: Vec<(usize, usize)>,
    tol_sq: f64,
    start: bool,
}

impl<S: AsRef<[Point]>> Rdp<S> {
    fn new(pts: S, tolerance: f64) -> Self {
        let n = pts.as_ref().len();
        Self {
            pts,
            stack: if n > 1 {
                vec![(0, n.saturating_sub(1))]
            } else {
                Vec::new()
            },
            tol_sq: tolerance * tolerance,
            start: n > 0,
        }
    }
}
#[expect(clippy::missing_trait_methods, reason = "only for internal use")]
impl<S: AsRef<[Point]>> Iterator for Rdp<S> {
    type Item = Point;
    fn next(&mut self) -> Option<Point> {
        let pts = self.pts.as_ref();
        if take(&mut self.start) {
            return pts.first().copied();
        }
        while let Some((s, e)) = self.stack.pop() {
            let (Some(&a), Some(&b)) = (pts.get(s), pts.get(e)) else {
                continue;
            };
            let chord = Line::new(a, b);
            let farthest = pts
                .get(s.saturating_add(1)..e)
                .into_iter()
                .flatten()
                .enumerate()
                .map(|(off, p)| {
                    (
                        s.saturating_add(1).saturating_add(off),
                        chord.nearest(*p, 1e-12).distance_sq,
                    )
                })
                .max_by(|x, y| x.1.total_cmp(&y.1));
            match farthest {
                Some((split, dist_sq)) if dist_sq > self.tol_sq => {
                    self.stack.push((split, e));
                    self.stack.push((s, split));
                }
                _ => return Some(b),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn noisy_line_collapses_to_endpoints() {
        let pts =
            (0..=20).map(|i| Point::new(f64::from(i) / 20.0, 0.001 * f64::from(1 - 2 * (i % 2))));
        let out: Vec<Point> = rdp(pts, 0.02).collect();
        assert_eq!(out.len(), 2, "{out:?}");
        assert!(out.first().expect("first").x.abs() < 1e-12);
        assert!((out.last().expect("last").x - 1.0).abs() < 1e-12);
    }
    #[test]
    fn right_angle_keeps_corner_in_order() {
        let pts: Vec<Point> = (0..=10)
            .map(|i| Point::new(f64::from(i) / 10.0, 0.0))
            .chain((1..=10).map(|i| Point::new(1.0, f64::from(i) / 10.0)))
            .collect();
        let out: Vec<Point> = rdp_slice(&pts, 0.02).collect();
        assert_eq!(out.len(), 3, "{out:?}");
        assert!(out.get(1).expect("corner").distance(Point::new(1.0, 0.0)) < 1e-12);
    }
    #[test]
    fn u_shape_interior_survives() {
        let pts = [(0.0, 0.0), (0.0, 0.5), (0.25, 0.7), (0.5, 0.5), (0.5, 0.02)]
            .map(|(x, y)| Point::new(x, y));
        let out: Vec<Point> = rdp_slice(&pts, 0.02).collect();
        assert_eq!(out.len(), 5, "{out:?}");
    }
    #[test]
    fn duplicates_vanish() {
        let pts = [(0.0, 0.0), (0.0, 0.0), (0.5, 0.0), (0.5, 0.0), (1.0, 0.001)]
            .map(|(x, y)| Point::new(x, y));
        assert_eq!(rdp(pts.into_iter(), 0.02).count(), 2);
    }
    #[test]
    fn degenerate_inputs() {
        assert_eq!(rdp(core::iter::empty(), 0.02).count(), 0);
        assert_eq!(rdp(core::iter::once(Point::new(1.0, 2.0)), 0.02).count(), 1);
    }
}
