use crate::point::OrientedPoint;

#[derive(Debug, Clone, Copy)]
pub struct DtwWeights {
    pub w_pos: f32,
    pub w_dir: f32,
}

impl Default for DtwWeights {
    fn default() -> Self {
        DtwWeights {
            w_pos: 1.0,
            w_dir: 1.0,
        }
    }
}

#[must_use]
pub fn dtw(a: &[OrientedPoint], b: &[OrientedPoint], weights: DtwWeights) -> f32 {
    let n = a.len();
    let m = b.len();
    if n == 0 || m == 0 {
        return f32::INFINITY;
    }

    let mut dp = vec![vec![f32::INFINITY; m + 1]; n + 1];
    dp[0][0] = 0.0;

    for i in 1..=n {
        for j in 1..=m {
            let c = cost(&a[i - 1], &b[j - 1], weights);
            let prev = dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1]);
            dp[i][j] = c + prev;
        }
    }

    dp[n][m] / f32::from((n + m).try_into().unwrap_or(u16::MAX))
}
#[must_use]
pub fn dtw_with_path(
    a: &[OrientedPoint],
    b: &[OrientedPoint],
    weights: DtwWeights,
) -> (f32, Vec<(usize, usize, f32)>) {
    let n = a.len();
    let m = b.len();
    if n == 0 || m == 0 {
        return (f32::INFINITY, Vec::new());
    }

    let mut dp = vec![vec![f32::INFINITY; m + 1]; n + 1];
    dp[0][0] = 0.0;
    for i in 1..=n {
        for j in 1..=m {
            let c = cost(&a[i - 1], &b[j - 1], weights);
            let prev = dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1]);
            dp[i][j] = c + prev;
        }
    }

    let mut path = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        path.push((i - 1, j - 1, cost(&a[i - 1], &b[j - 1], weights)));
        let diag = dp[i - 1][j - 1];
        let up = dp[i - 1][j];
        let left = dp[i][j - 1];
        if diag <= up && diag <= left {
            i -= 1;
            j -= 1;
        } else if up <= left {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    path.reverse();

    (
        dp[n][m] / f32::from((n + m).try_into().unwrap_or(u16::MAX)),
        path,
    )
}
fn cost(p: &OrientedPoint, q: &OrientedPoint, w: DtwWeights) -> f32 {
    let dx = p.position.x - q.position.x;
    let dy = p.position.y - q.position.y;
    let pos_dist = (dx * dx + dy * dy).sqrt();

    let ddx = p.direction.x - q.direction.x;
    let ddy = p.direction.y - q.direction.y;
    let dir_dist = (ddx * ddx + ddy * ddy).sqrt();

    w.w_pos * pos_dist + w.w_dir * dir_dist
}
#[cfg(test)]
mod tests {
    use super::*;
    use lyon_path::math::{point, vector};

    fn op(x: f32, y: f32, dx: f32, dy: f32) -> OrientedPoint {
        OrientedPoint {
            position: point(x, y),
            direction: vector(dx, dy),
        }
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn identical_strokes_score_zero() {
        let a = vec![op(0.0, 0.0, 1.0, 0.0), op(1.0, 0.0, 1.0, 0.0)];
        let score = dtw(&a, &a, DtwWeights::default());
        assert!(approx(score, 0.0));
    }

    #[test]
    fn empty_input_returns_infinity() {
        let a: Vec<OrientedPoint> = vec![];
        let b = vec![op(0.0, 0.0, 1.0, 0.0)];
        assert!(dtw(&a, &b, DtwWeights::default()).is_infinite());
        assert!(dtw(&b, &a, DtwWeights::default()).is_infinite());
    }

    #[test]
    fn position_offset_increases_score() {
        let a = vec![op(0.0, 0.0, 1.0, 0.0), op(1.0, 0.0, 1.0, 0.0)];
        let b = vec![op(0.5, 0.0, 1.0, 0.0), op(1.5, 0.0, 1.0, 0.0)];
        let score = dtw(&a, &b, DtwWeights::default());
        assert!(score > 0.0);
    }

    #[test]
    fn reversed_direction_increases_score() {
        // Same positions, opposite directions.
        let a = vec![op(0.0, 0.0, 1.0, 0.0), op(1.0, 0.0, 1.0, 0.0)];
        let b = vec![op(0.0, 0.0, -1.0, 0.0), op(1.0, 0.0, -1.0, 0.0)];

        let with_dir = dtw(
            &a,
            &b,
            DtwWeights {
                w_pos: 1.0,
                w_dir: 1.0,
            },
        );
        let without_dir = dtw(
            &a,
            &b,
            DtwWeights {
                w_pos: 1.0,
                w_dir: 0.0,
            },
        );

        assert!(with_dir > without_dir);
    }

    #[test]
    fn dir_weight_zero_ignores_direction() {
        let a = vec![op(0.0, 0.0, 1.0, 0.0)];
        let b = vec![op(0.0, 0.0, -1.0, 0.0)]; // same position, opposite direction
        let score = dtw(
            &a,
            &b,
            DtwWeights {
                w_pos: 1.0,
                w_dir: 0.0,
            },
        );
        assert!(approx(score, 0.0));
    }

    #[test]
    fn pos_weight_zero_ignores_position() {
        let a = vec![op(0.0, 0.0, 1.0, 0.0)];
        let b = vec![op(5.0, 5.0, 1.0, 0.0)]; // different position, same direction
        let score = dtw(
            &a,
            &b,
            DtwWeights {
                w_pos: 0.0,
                w_dir: 1.0,
            },
        );
        assert!(approx(score, 0.0));
    }

    #[test]
    fn handles_unequal_lengths() {
        // Same line, different sampling density. DTW should align cleanly.
        let a = vec![op(0.0, 0.0, 1.0, 0.0), op(1.0, 0.0, 1.0, 0.0)];
        let b = vec![
            op(0.0, 0.0, 1.0, 0.0),
            op(0.5, 0.0, 1.0, 0.0),
            op(1.0, 0.0, 1.0, 0.0),
        ];
        let score = dtw(&a, &b, DtwWeights::default());
        assert!(score < 0.2, "got {}", score);
    }
    #[test]
    fn handles_unequal() {
        let a = vec![op(0.0, 0.0, 1.0, 0.0), op(1.0, 0.0, 1.0, 0.0)];
        let b_similar = vec![
            op(0.0, 0.0, 1.0, 0.0),
            op(0.5, 0.0, 1.0, 0.0),
            op(1.0, 0.0, 1.0, 0.0),
        ];
        let b_different = vec![
            op(0.0, 0.0, 1.0, 0.0),
            op(0.5, 1.0, 0.0, 1.0),
            op(1.0, 1.0, -1.0, 0.0),
        ];

        let score_similar = dtw(&a, &b_similar, DtwWeights::default());
        let score_different = dtw(&a, &b_different, DtwWeights::default());

        assert!(score_similar < score_different);
    }

    #[test]
    fn shifted_stroke_scores_higher_than_identical() {
        let identical = vec![op(0.0, 0.0, 1.0, 0.0), op(1.0, 0.0, 1.0, 0.0)];
        let shifted = vec![op(0.3, 0.3, 1.0, 0.0), op(1.3, 0.3, 1.0, 0.0)];
        let s_identical = dtw(&identical, &identical, DtwWeights::default());
        let s_shifted = dtw(&identical, &shifted, DtwWeights::default());
        assert!(s_shifted > s_identical);
    }
}
