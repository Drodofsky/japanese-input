use core::iter::{once, successors};
use core::mem::swap;
use kurbo::{
    BezPath, ParamCurve as _, ParamCurveArclen as _, ParamCurveDeriv as _, PathSeg, Point, Vec2,
};

use crate::{stroke_point::StrokePoint, to_index::ToIndex as _};
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub step: f64,
    pub min_gap: f64,
    pub max_gap: f64,
    pub resolution: f64,
    pub accuracy: f64,
}

impl Params {
    #[must_use]
    #[inline]
    pub const fn from_step(step: f64) -> Self {
        Self {
            step,
            min_gap: step / 4.0,
            max_gap: 1.75 * step,
            resolution: step / 20.0,
            accuracy: 1e-6,
        }
    }
}

impl Default for Params {
    #[inline]
    fn default() -> Self {
        Self::from_step(0.1)
    }
}

#[derive(Clone, Copy)]
struct Cand {
    s: f64,
    point: Point,
    seg: PathSeg,
    t: f64,
    join: bool,
}

// Running sums of the geometric moments of a set of points, used to evaluate
// the perpendicular chord error of a span in O(1) via prefix subtraction.
#[derive(Clone, Copy)]
struct Moments {
    n: f64,
    sx: f64,
    sy: f64,
    sxx: f64,
    syy: f64,
    sxy: f64,
}

impl Moments {
    const ZERO: Self = Self {
        n: 0.0_f64,
        sx: 0.0_f64,
        sy: 0.0_f64,
        sxx: 0.0_f64,
        syy: 0.0_f64,
        sxy: 0.0_f64,
    };

    #[inline]
    fn push(self, p: Point) -> Self {
        let (x, y) = (p.x, p.y);
        Self {
            n: self.n + 1.0_f64,
            sx: self.sx + x,
            sy: self.sy + y,
            sxx: self.sxx + x * x,
            syy: self.syy + y * y,
            sxy: self.sxy + x * y,
        }
    }

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            n: self.n - rhs.n,
            sx: self.sx - rhs.sx,
            sy: self.sy - rhs.sy,
            sxx: self.sxx - rhs.sxx,
            syy: self.syy - rhs.syy,
            sxy: self.sxy - rhs.sxy,
        }
    }
}

fn candidates(path: &BezPath, p: &Params) -> Vec<Cand> {
    let (mut cands, mut start, mut end) = (Vec::new(), 0.0_f64, None);
    for seg in path.segments() {
        let len = seg.arclen(p.accuracy);
        if len > 0.0_f64 {
            cands.push(Cand {
                s: start,
                point: seg.eval(0.0_f64),
                seg,
                t: 0.0_f64,
                join: true,
            });
            let first = ((start / p.resolution).floor() + 1.0_f64) * p.resolution;
            for s in
                successors(Some(first), |s| Some(s + p.resolution)).take_while(|s| *s < start + len)
            {
                let t = seg.inv_arclen(s - start, p.accuracy);
                cands.push(Cand {
                    s,
                    point: seg.eval(t),
                    seg,
                    t,
                    join: false,
                });
            }
        }
        start += len;
        end = Some(Cand {
            s: start,
            point: seg.eval(1.0_f64),
            seg,
            t: 1.0_f64,
            join: true,
        });
    }
    cands.extend(end);
    let eps = p.resolution * 1e-3_f64;
    cands.dedup_by(|cur, kept| {
        let same = (cur.s - kept.s).abs() <= eps;
        if same && cur.join {
            *kept = *cur;
        }
        same
    });
    cands
}

fn prefix_moments(cands: &[Cand]) -> Vec<Moments> {
    once(Moments::ZERO)
        .chain(cands.iter().scan(Moments::ZERO, |m, c| {
            *m = m.push(c.point);
            Some(*m)
        }))
        .collect()
}

fn chord_error(pre: &[Moments], i: usize, j: usize, a: Point, b: Point) -> f64 {
    let (Some(hi), Some(lo)) = (pre.get(j), pre.get(i.saturating_add(1))) else {
        return f64::INFINITY;
    };
    let Moments {
        n,
        sx,
        sy,
        sxx,
        syy,
        sxy,
    } = hi.sub(*lo);
    if n <= 0.0_f64 {
        return 0.0_f64;
    }
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 <= f64::EPSILON {
        return f64::INFINITY;
    }
    let (u, v, w) = (-dy, dx, dy * a.x - dx * a.y);
    ((u * u * sxx + v * v * syy + 2.0_f64 * (u * v * sxy + u * w * sx + v * w * sy) + n * w * w)
        / len2)
        .max(0.0_f64)
}
#[expect(clippy::arithmetic_side_effects, reason = "checked with len > epsilon")]
fn sample(c: &Cand) -> StrokePoint {
    let d = match c.seg {
        PathSeg::Line(s) => s.deriv().eval(c.t).to_vec2(),
        PathSeg::Quad(s) => s.deriv().eval(c.t).to_vec2(),
        PathSeg::Cubic(s) => s.deriv().eval(c.t).to_vec2(),
    };
    let len = d.hypot();
    let tangent = if len > f64::EPSILON {
        d / len
    } else {
        Vec2::ZERO
    };
    StrokePoint {
        position: c.point,
        tangent,
    }
}

fn less(a: [f64; 2], b: [f64; 2]) -> bool {
    a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])).is_lt()
}

#[must_use]
#[inline]
pub fn resample_path(path: &BezPath, p: &Params) -> Option<Vec<StrokePoint>> {
    if !(0.0_f64 < p.min_gap
        && p.min_gap < p.step
        && p.step <= p.max_gap
        && 0.0_f64 < p.resolution
        && p.resolution <= p.min_gap
        && p.accuracy > 0.0_f64)
    {
        return None;
    }
    let cands = candidates(path, p);
    let total = cands.last()?.s;
    if total <= p.min_gap {
        return Some([cands.first()?, cands.last()?].map(sample).to_vec());
    }
    let n_f = (total / p.step)
        .round()
        .clamp(
            (total / p.max_gap).ceil() + 1.0_f64,
            (total / p.min_gap).floor() + 1.0_f64,
        )
        .max(2.0_f64);
    let n = (n_f.to_index()).min(cands.len());

    let (m, slack) = (cands.len(), p.resolution * 1e-3_f64);
    let pre = prefix_moments(&cands);
    let mut col = vec![[f64::INFINITY; 2]; m];
    *col.first_mut()? = [0.0_f64, 0.0_f64];
    let mut next = vec![[f64::INFINITY; 2]; m];
    let mut back = vec![vec![None::<usize>; n.saturating_add(1)]; m];

    for k in 2..=n {
        next.fill([f64::INFINITY; 2]);
        let (mut lo, mut hi) = (0_usize, 0_usize);
        for (j, cand_j) in cands.iter().enumerate().skip(k.saturating_sub(1)) {
            let (floor_s, ceil_s) = (cand_j.s - p.max_gap - slack, cand_j.s - p.min_gap + slack);
            while cands.get(lo).is_some_and(|c| c.s < floor_s) {
                lo = lo.saturating_add(1);
            }
            hi = hi.max(lo);
            while cands.get(hi).is_some_and(|c| c.s <= ceil_s) {
                hi = hi.saturating_add(1);
            }
            let (mut cell, mut from) = ([f64::INFINITY; 2], None);
            for (i, cand_i) in cands.iter().enumerate().take(hi).skip(lo) {
                let prior = *col.get(i)?;
                if !prior[0].is_finite() {
                    continue;
                }
                let gap = cand_j.s - cand_i.s;
                let cost = [
                    prior[0] + chord_error(&pre, i, j, cand_i.point, cand_j.point),
                    prior[1] + gap * gap,
                ];
                if less(cost, cell) {
                    (cell, from) = (cost, Some(i));
                }
            }
            *next.get_mut(j)? = cell;
            *back.get_mut(j)?.get_mut(k)? = from;
        }
        swap(&mut col, &mut next);
    }
    if !col.last()?[0].is_finite() {
        return None;
    }
    let mut picked: Vec<usize> = successors(Some((m.saturating_sub(1), n)), |&(j, k)| {
        (k > 1)
            .then(|| back.get(j)?.get(k)?.map(|i| (i, k.saturating_sub(1))))
            .flatten()
    })
    .map(|(j, _)| j)
    .collect();
    picked.reverse();
    (picked.len() == n).then(|| {
        picked
            .iter()
            .filter_map(|&i| cands.get(i).map(sample))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_gets_a_sample() {
        let mut path = BezPath::new();
        path.move_to(Point::ZERO);
        path.line_to(Point::new(0.5, 0.0_f64));
        path.line_to(Point::new(0.5, 0.5));
        let s = resample_path(&path, &Params::default()).expect("resample");
        assert!(
            s.iter()
                .any(|s| s.position.distance(Point::new(0.5, 0.0_f64)) < 1e-9)
        );
    }
}
