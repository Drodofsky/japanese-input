use crate::analyzed_kanji_node::AnalyzedKanjiNode;
use crate::dtw::{DtwWeights, dtw};
use crate::normalize::Normalize;
use crate::point::{OrientedPoint, ToOriented};

const MAX_WIDTH: usize = 5000;

pub const MISSING_PENALTY: f32 = 1.0;
const EXTRA_PENALTY: f32 = 1.0;
const FRAME_B_WEIGHT: f32 = 0.3;
const FRAME_C_WEIGHT: f32 = 0.3;
const LENGTH_WEIGHT: f32 = 0.4;
const FRAME_G_WEIGHT: f32 = 0.5;
const ORDER_WEIGHT: f32 = 0.2;

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub user_strokes: Vec<usize>,
    pub score: f32,
}

pub fn match_node(reference: &AnalyzedKanjiNode, user: &[Vec<(f32, f32)>]) -> Vec<MatchInfo> {
    let (user_b, user_c) = prepare_user(user);
    let mut results = beam(reference, &user_b, &user_c, 10);

    let max_matched = results
        .iter()
        .map(|m| m.user_strokes.iter().filter(|&&i| i != usize::MAX).count())
        .max()
        .unwrap_or(0);
    results.retain(|m| m.user_strokes.iter().filter(|&&i| i != usize::MAX).count() == max_matched);
    results.sort_by_key(|m| m.user_strokes.clone());
    results.dedup_by_key(|m| m.user_strokes.clone());
    results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    let user_count = user.len();
    for r in results.iter_mut() {
        let used = r
            .user_strokes
            .iter()
            .copied()
            .filter(|&i| i != usize::MAX)
            .count();
        let extras = user_count.saturating_sub(used);
        r.score += EXTRA_PENALTY * extras as f32;
    }

    results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    results
}

fn prepare_user(user: &[Vec<(f32, f32)>]) -> (Vec<Vec<OrientedPoint>>, Vec<Vec<OrientedPoint>>) {
    let oriented: Vec<Vec<OrientedPoint>> =
        user.iter().map(|s| s.as_slice().to_oriented()).collect();
    let in_kanji_frame = oriented.clone().normalize();
    let in_stroke_frame: Vec<Vec<OrientedPoint>> =
        oriented.into_iter().map(|s| s.normalize()).collect();
    (in_kanji_frame, in_stroke_frame)
}

fn beam(
    node: &AnalyzedKanjiNode,
    user_b: &[Vec<OrientedPoint>],
    user_c: &[Vec<OrientedPoint>],
    width: usize,
) -> Vec<MatchInfo> {
    match node {
        AnalyzedKanjiNode::Stroke {
            in_kanji_frame: ref_b,
            in_stroke_frame: ref_c,
            ..
        } => {
            let weights = DtwWeights::default();
            let ref_len = bbox_longer_side(ref_b);

            let mut candidates: Vec<MatchInfo> = (0..user_b.len())
                .map(|i| {
                    let s_b = dtw(ref_b, &user_b[i], weights);
                    let s_c = dtw(ref_c, &user_c[i], weights);
                    let s_len = (ref_len - bbox_longer_side(&user_b[i])).abs();
                    let combined =
                        FRAME_B_WEIGHT * s_b + FRAME_C_WEIGHT * s_c + LENGTH_WEIGHT * s_len;
                    MatchInfo {
                        user_strokes: vec![i],
                        score: combined,
                    }
                })
                .collect();

            candidates.push(MatchInfo {
                user_strokes: vec![usize::MAX],
                score: MISSING_PENALTY,
            });
            candidates.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            candidates.truncate(width);
            candidates
        }

        AnalyzedKanjiNode::Group { children, .. } => {
            let mut try_width = width;

            let results = loop {
                let child_candidates: Vec<Vec<MatchInfo>> = children
                    .iter()
                    .map(|child| beam(child, user_b, user_c, try_width))
                    .collect();

                let combined = combine_children(&child_candidates, try_width);

                if !combined.is_empty() || try_width >= MAX_WIDTH {
                    break combined;
                }
                try_width *= 2;
            };

            let mut results = results;

            // Group-level extras: frame G + order continuity
            for r in results.iter_mut() {
                let matched: Vec<usize> = r
                    .user_strokes
                    .iter()
                    .copied()
                    .filter(|&i| i != usize::MAX)
                    .collect();
                if matched.len() < 2 {
                    continue;
                }
                let extra = group_extras(node, &r.user_strokes, user_b);
                r.score += extra;
            }

            results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            truncate_with_permutation_cap(results, width)
        }
    }
}

/// Computes frame-G DTW + Kendall-tau order penalty, weighted.
fn group_extras(
    group: &AnalyzedKanjiNode,
    user_strokes: &[usize],
    user_b: &[Vec<OrientedPoint>],
) -> f32 {
    // Walk the subtree, pair each leaf with its assigned user stroke (skip missing).
    let mut leaf_pairs: Vec<(&[OrientedPoint], usize)> = Vec::new();
    let mut idx = 0;
    collect_leaf_pairs(group, user_strokes, &mut idx, &mut leaf_pairs);

    let matched: Vec<(&[OrientedPoint], &Vec<OrientedPoint>)> = leaf_pairs
        .iter()
        .filter(|(_, ui)| *ui != usize::MAX)
        .map(|(rb, ui)| (*rb, &user_b[*ui]))
        .collect();
    if matched.len() < 2 {
        return 0.0;
    }

    // Frame G: renormalize ref strokes against ref-group bbox, user strokes against user-group bbox.
    let ref_g: Vec<Vec<OrientedPoint>> = matched
        .iter()
        .map(|(rb, _)| rb.to_vec())
        .collect::<Vec<_>>()
        .normalize();
    let user_g: Vec<Vec<OrientedPoint>> = matched
        .iter()
        .map(|(_, ub)| (*ub).clone())
        .collect::<Vec<_>>()
        .normalize();

    let weights = DtwWeights::default();
    let dtw_sum: f32 = ref_g
        .iter()
        .zip(user_g.iter())
        .map(|(r, u)| dtw(r, u, weights))
        .sum();
    let dtw_avg = dtw_sum / matched.len() as f32;

    // Kendall tau on the user-stroke indices.
    let user_indices: Vec<usize> = leaf_pairs
        .iter()
        .filter(|(_, ui)| *ui != usize::MAX)
        .map(|(_, ui)| *ui)
        .collect();
    let tau = kendall_tau(&user_indices);

    FRAME_G_WEIGHT * dtw_avg + ORDER_WEIGHT * tau
}

fn collect_leaf_pairs<'a>(
    node: &'a AnalyzedKanjiNode,
    user_strokes: &[usize],
    idx: &mut usize,
    out: &mut Vec<(&'a [OrientedPoint], usize)>,
) {
    match node {
        AnalyzedKanjiNode::Stroke { in_kanji_frame, .. } => {
            out.push((in_kanji_frame.as_slice(), user_strokes[*idx]));
            *idx += 1;
        }
        AnalyzedKanjiNode::Group { children, .. } => {
            for c in children {
                collect_leaf_pairs(c, user_strokes, idx, out);
            }
        }
    }
}

/// Normalized inversion count: 0.0 (perfectly sorted) to 1.0 (fully reversed).
fn kendall_tau(seq: &[usize]) -> f32 {
    let n = seq.len();
    if n < 2 {
        return 0.0;
    }
    let mut inv = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            if seq[i] > seq[j] {
                inv += 1;
            }
        }
    }
    let max = n * (n - 1) / 2;
    inv as f32 / max as f32
}

fn bbox_longer_side(stroke: &[OrientedPoint]) -> f32 {
    if stroke.is_empty() {
        return 0.0;
    }
    let mut min_x = stroke[0].position.x;
    let mut max_x = min_x;
    let mut min_y = stroke[0].position.y;
    let mut max_y = min_y;
    for op in &stroke[1..] {
        min_x = min_x.min(op.position.x);
        max_x = max_x.max(op.position.x);
        min_y = min_y.min(op.position.y);
        max_y = max_y.max(op.position.y);
    }
    (max_x - min_x).max(max_y - min_y)
}

fn combine_children(child_candidates: &[Vec<MatchInfo>], width: usize) -> Vec<MatchInfo> {
    let mut results: Vec<MatchInfo> = vec![MatchInfo {
        user_strokes: vec![],
        score: 0.0,
    }];

    for cands in child_candidates {
        let mut next: Vec<MatchInfo> = Vec::new();
        for partial in &results {
            for candidate in cands {
                let overlaps = candidate
                    .user_strokes
                    .iter()
                    .filter(|&&i| i != usize::MAX)
                    .any(|s| partial.user_strokes.contains(s));
                if overlaps {
                    continue;
                }
                let mut combined = partial.user_strokes.clone();
                combined.extend(&candidate.user_strokes);
                next.push(MatchInfo {
                    user_strokes: combined,
                    score: partial.score + candidate.score,
                });
            }
        }
        next.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        results = truncate_with_permutation_cap(next, width);
    }

    results
}

fn truncate_with_permutation_cap(entries: Vec<MatchInfo>, width: usize) -> Vec<MatchInfo> {
    let mut group_counts: std::collections::HashMap<Vec<usize>, usize> =
        std::collections::HashMap::new();
    let mut kept: Vec<MatchInfo> = Vec::with_capacity(width);
    for entry in entries.into_iter() {
        let mut key = entry.user_strokes.clone();
        key.sort_unstable();
        let cap = entry.user_strokes.len().max(1);
        let count = group_counts.entry(key).or_insert(0);
        if *count < cap {
            *count += 1;
            kept.push(entry);
            if kept.len() >= width {
                break;
            }
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KanjiNode;
    use lyon_path::Path;
    use lyon_path::math::point;

    fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
        let mut b = Path::builder();
        b.begin(point(x0, y0));
        b.line_to(point(x1, y1));
        b.end(false);
        b.build()
    }

    fn user_line(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(f32, f32)> {
        let n = 20;
        (0..=n)
            .map(|i| {
                let t = i as f32 / n as f32;
                (x0 + t * (x1 - x0), y0 + t * (y1 - y0))
            })
            .collect()
    }

    fn three_kanji() -> AnalyzedKanjiNode {
        let node = KanjiNode::Group {
            element: Some('三'),
            children: vec![
                KanjiNode::Stroke {
                    index: 0,
                    path: line(20.0, 20.0, 80.0, 20.0),
                },
                KanjiNode::Stroke {
                    index: 1,
                    path: line(20.0, 50.0, 80.0, 50.0),
                },
                KanjiNode::Stroke {
                    index: 2,
                    path: line(20.0, 80.0, 80.0, 80.0),
                },
            ],
        };
        AnalyzedKanjiNode::from_node(&node)
    }

    #[test]
    fn correct_order() {
        let reference = three_kanji();
        let user = vec![
            user_line(20.0, 20.0, 80.0, 20.0),
            user_line(20.0, 50.0, 80.0, 50.0),
            user_line(20.0, 80.0, 80.0, 80.0),
        ];
        let result = match_node(&reference, &user);
        assert!(!result.is_empty());
        assert_eq!(result[0].user_strokes, vec![0, 1, 2]);
    }

    #[test]
    fn reverse_order() {
        let reference = three_kanji();
        // Drawn bottom-to-top.
        let user = vec![
            user_line(20.0, 80.0, 80.0, 80.0),
            user_line(20.0, 50.0, 80.0, 50.0),
            user_line(20.0, 20.0, 80.0, 20.0),
        ];
        let result = match_node(&reference, &user);
        assert!(!result.is_empty());
        assert_eq!(result[0].user_strokes, vec![2, 1, 0]);
    }

    #[test]
    fn extra_stroke_is_not_matched() {
        let reference = three_kanji();
        // Four strokes — one is the "extra."
        let user = vec![
            user_line(20.0, 20.0, 80.0, 20.0),
            user_line(20.0, 50.0, 80.0, 50.0),
            user_line(20.0, 80.0, 80.0, 80.0),
            user_line(50.0, 50.0, 50.0, 90.0), // odd stroke
        ];
        let result = match_node(&reference, &user);
        assert!(!result.is_empty());
        assert_eq!(result[0].user_strokes, vec![0, 1, 2]);
    }

    #[test]
    fn missing_middle_stroke() {
        let reference = three_kanji();
        let user = vec![
            user_line(20.0, 20.0, 80.0, 20.0),
            user_line(20.0, 80.0, 80.0, 80.0),
        ];
        let result = match_node(&reference, &user);
        assert!(!result.is_empty());
        assert_eq!(result[0].user_strokes, vec![0, usize::MAX, 1]);
    }
}
