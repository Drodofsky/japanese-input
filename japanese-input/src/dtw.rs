use dtw_rs::{Solution as _, dtw_with_distance};

use crate::{convert_lossy::ConvertLossy as _, stroke_point::StrokePoint};
pub type PathStep = (usize, usize, f64);

pub type DtwPath = (f64, Vec<PathStep>);
#[derive(Debug)]
#[non_exhaustive]
pub struct DtwError;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct DTWWeights {
    pub position: f64,
    pub tangent: f64,
}

impl Default for DTWWeights {
    #[inline]
    fn default() -> Self {
        Self {
            position: 1.0,
            tangent: 1.0,
        }
    }
}

#[inline]
fn cost(a: &StrokePoint, b: &StrokePoint, w: &DTWWeights) -> f64 {
    let position = a.position.distance(b.position);
    let tangent = 1.0_f64 - a.tangent.dot(b.tangent);
    w.position * position + w.tangent * tangent
}

#[must_use]
#[inline]
pub fn dtw(a: &[StrokePoint], b: &[StrokePoint], weights: &DTWWeights) -> f64 {
    if a.is_empty() || b.is_empty() {
        return f64::INFINITY;
    }
    let result = dtw_with_distance(a, b, |p, q| cost(p, q, weights));
    normalize(result.distance(), a.len(), b.len())
}

/// # Errors
/// Internal error that should not occur under normal operation.
#[inline]
pub fn dtw_with_path(
    a: &[StrokePoint],
    b: &[StrokePoint],
    weights: &DTWWeights,
) -> Result<DtwPath, DtwError> {
    if a.is_empty() || b.is_empty() {
        return Ok((f64::INFINITY, Vec::new()));
    }
    let result = dtw_with_distance(a, b, |p, q| cost(p, q, weights));

    let path = result
        .path()
        .iter()
        .map(|&(i, j)| {
            let pa = a.get(i).ok_or(DtwError)?;
            let pb = b.get(j).ok_or(DtwError)?;
            Ok((i, j, cost(pa, pb, weights)))
        })
        .collect::<Result<Vec<_>, DtwError>>()?;

    Ok((normalize(result.distance(), a.len(), b.len()), path))
}

#[inline]
fn normalize(distance: f64, n: usize, m: usize) -> f64 {
    let denom = (n.saturating_add(m)).convert_lossy();
    if denom > 0.0 {
        distance / denom
    } else {
        f64::INFINITY
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Point, Vec2};

    fn sp(x: f64, y: f64, tx: f64, ty: f64) -> StrokePoint {
        StrokePoint {
            position: Point::new(x, y),
            tangent: Vec2::new(tx, ty),
        }
    }
    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn identical_strokes_score_zero() {
        let s = vec![sp(0.0, 0.0, 1.0, 0.0), sp(0.5, 0.0, 1.0, 0.0)];
        assert!(approx(dtw(&s, &s, &DTWWeights::default()), 0.0));
    }

    #[test]
    fn empty_input_is_infinite() {
        let s = vec![sp(0.0, 0.0, 1.0, 0.0)];
        assert!(dtw(&[], &s, &DTWWeights::default()).is_infinite());
        assert!(dtw(&s, &[], &DTWWeights::default()).is_infinite());
    }

    #[test]
    fn symmetric() {
        let a = vec![sp(0.0, 0.0, 1.0, 0.0), sp(0.3, 0.1, 1.0, 0.0)];
        let b = vec![sp(0.1, 0.0, 1.0, 0.0), sp(0.5, 0.2, 1.0, 0.0)];
        let ab = dtw(&a, &b, &DTWWeights::default());
        let ba = dtw(&b, &a, &DTWWeights::default());
        assert!(approx(ab, ba), "{ab} vs {ba}");
    }

    #[test]
    fn known_value_no_double_normalization() {
        let a = vec![sp(0.0, 0.0, 1.0, 0.0)];
        let b = vec![sp(1.0, 0.0, 1.0, 0.0)];
        let score = dtw(&a, &b, &DTWWeights::default());
        assert!(
            approx(score, 0.5),
            "erwartet 0.5 (roh 1.0 / 2), bekam {score}"
        );
    }

    #[test]
    fn weights_apply() {
        let a = vec![sp(0.0, 0.0, 1.0, 0.0)];
        let b = vec![sp(0.0, 0.0, 1.0, 0.0)];
        let on = dtw(&a, &b, &DTWWeights::default());
        assert!(approx(on, 1.5), "bekam {on}");
        let off = dtw(
            &a,
            &b,
            &DTWWeights {
                position: 1.0,
                tangent: 1.0,
            },
        );
        assert!(approx(off, 0.0), "bekam {off}");
    }

    #[test]
    fn unequal_lengths_warp_cheaply() {
        let coarse = vec![sp(0.0, 0.0, 1.0, 0.0), sp(1.0, 0.0, 1.0, 0.0)];
        let fine = vec![
            sp(0.0, 0.0, 1.0, 0.0),
            sp(0.5, 0.0, 1.0, 0.0),
            sp(1.0, 0.0, 1.0, 0.0),
        ];
        assert!(dtw(&coarse, &fine, &DTWWeights::default()) < 0.2);
    }

    #[test]
    fn path_of_identical_is_diagonal() {
        let s = vec![
            sp(0.0, 0.0, 1.0, 0.0),
            sp(0.5, 0.0, 1.0, 0.0),
            sp(1.0, 0.0, 1.0, 0.0),
        ];
        let (_, path) = dtw_with_path(&s, &s, &DTWWeights::default()).unwrap();
        for &(i, j, c) in &path {
            assert_eq!(i, j, "diagonaler Pfad: i==j erwartet, ({i},{j})");
            assert!(
                approx(c, 0.0),
                "Schrittkosten auf identischem Pfad sollten 0 sein"
            );
        }
        assert_eq!(path.first().map(|&(i, j, _)| (i, j)), Some((0, 0)));
        assert_eq!(path.last().map(|&(i, j, _)| (i, j)), Some((2, 2)));
    }

    #[test]
    fn path_endpoints_anchored() {
        let a = vec![sp(0.0, 0.0, 1.0, 0.0), sp(1.0, 1.0, 0.0, 1.0)];
        let b = vec![
            sp(0.0, 0.0, 1.0, 0.0),
            sp(0.5, 0.5, 1.0, 1.0),
            sp(1.0, 1.0, 0.0, 1.0),
        ];
        let (_, path) = dtw_with_path(&a, &b, &DTWWeights::default()).unwrap();
        assert_eq!(path.first().map(|&(i, j, _)| (i, j)), Some((0, 0)));
        assert_eq!(
            path.last().map(|&(i, j, _)| (i, j)),
            Some((a.len() - 1, b.len() - 1))
        );
    }

    #[test]
    fn path_score_matches_dtw() {
        let a = vec![sp(0.0, 0.0, 1.0, 0.0), sp(0.5, 0.2, 1.0, 0.0)];
        let b = vec![sp(0.1, 0.0, 1.0, 0.0), sp(0.6, 0.3, 1.0, 0.0)];
        let s1 = dtw(&a, &b, &DTWWeights::default());
        let (s2, _) = dtw_with_path(&a, &b, &DTWWeights::default()).unwrap();
        assert!(approx(s1, s2), "{s1} vs {s2}");
    }
}
