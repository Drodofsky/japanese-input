/// Aggregates per-reference-point DTW costs onto the corresponding user points.
/// Returns one quality score per user point (average cost of all reference points
/// that mapped to it), or `0.0` for unmapped points.
pub(super) fn aggregate_per_user_point(path: &[(usize, usize, f32)], user_len: usize) -> Vec<f32> {
    let mut sums = vec![0.0f32; user_len];
    let mut counts = vec![0usize; user_len];
    for &(_a_idx, b_idx, cost) in path {
        if b_idx < user_len {
            sums[b_idx] += cost;
            counts[b_idx] += 1;
        }
    }
    sums.iter()
        .zip(counts.iter())
        .map(|(&s, &c)| {
            if c > 0 {
                s / f32::from(c.try_into().unwrap_or(u16::MAX))
            } else {
                0.0
            }
        })
        .collect()
}
