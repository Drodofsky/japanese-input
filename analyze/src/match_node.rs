use smallvec::{SmallVec, smallvec};

use crate::analyzed_kanji_node::AnalyzedKanjiNode;
use crate::dtw::{DtwWeights, dtw};
use crate::leaf_matrix::LeafMatrix;
use crate::normalize::Normalize;
use crate::point::{OrientedPoint, ToOriented};

const MAX_WIDTH: usize = 5000;

pub const MISSING_PENALTY: f32 = 1.0;
const EXTRA_PENALTY: f32 = 1.0;
const FRAME_G_WEIGHT: f32 = 0.5;
const ORDER_WEIGHT: f32 = 0.2;

pub type StrokeVec = SmallVec<[u8; 32]>;

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub user_strokes: StrokeVec,
    pub score: f32,
    pub used_mask: u32,
}

#[must_use]
pub fn match_node(reference: &AnalyzedKanjiNode, user: &[Vec<(f32, f32)>]) -> Vec<MatchInfo> {
    let (user_b, user_c) = prepare_user(user);
    let leaf_matrix = LeafMatrix::create(reference, &user_b, &user_c);
    let mut results = beam(reference, &user_b, &leaf_matrix, 10);

    let max_matched = results
        .iter()
        .map(|m| m.user_strokes.iter().filter(|&&i| i != u8::MAX).count())
        .max()
        .unwrap_or(0);
    results.retain(|m| m.user_strokes.iter().filter(|&&i| i != u8::MAX).count() == max_matched);
    results.sort_by_key(|m| m.user_strokes.clone());
    results.dedup_by_key(|m| m.user_strokes.clone());
    results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    let user_count = user.len();
    for r in &mut results {
        let used = r
            .user_strokes
            .iter()
            .copied()
            .filter(|&i| i != u8::MAX)
            .count();
        let extras = user_count.saturating_sub(used);
        r.score += EXTRA_PENALTY * f32::from(extras.try_into().unwrap_or(u16::MAX));
    }

    results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    results
}

#[must_use]
pub fn prepare_user(
    user: &[Vec<(f32, f32)>],
) -> (Vec<Vec<OrientedPoint>>, Vec<Vec<OrientedPoint>>) {
    let oriented: Vec<Vec<OrientedPoint>> =
        user.iter().map(|s| s.as_slice().to_oriented()).collect();
    let in_kanji_frame = oriented.clone().normalize();
    let in_stroke_frame: Vec<Vec<OrientedPoint>> = oriented
        .into_iter()
        .map(super::normalize::Normalize::normalize)
        .collect();
    (in_kanji_frame, in_stroke_frame)
}

fn beam(
    node: &AnalyzedKanjiNode,
    user_b: &[Vec<OrientedPoint>],
    leaf_matrix: &LeafMatrix,
    width: usize,
) -> Vec<MatchInfo> {
    match node {
        AnalyzedKanjiNode::Stroke { index, .. } => {
            let n_user = leaf_matrix.n_user();
            let mut candidates: Vec<MatchInfo> = (0..n_user)
                .map(|i| MatchInfo {
                    user_strokes: smallvec![i],
                    score: leaf_matrix.look_up(*index, i),
                    used_mask: 1u32 << u16::from(i),
                })
                .collect();
            candidates.push(MatchInfo {
                user_strokes: smallvec![u8::MAX],
                score: leaf_matrix.look_up(*index, n_user),
                used_mask: 0,
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
                    .map(|child| beam(child, user_b, leaf_matrix, try_width))
                    .collect();

                let combined = combine_children(&child_candidates, try_width);

                if !combined.is_empty() || try_width >= MAX_WIDTH {
                    break combined;
                }
                try_width *= 2;
            };

            let mut results = results;

            // Group-level extras: frame G + order continuity
            for r in &mut results {
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
    user_strokes: &[u8],
    user_b: &[Vec<OrientedPoint>],
) -> f32 {
    // Walk the subtree, pair each leaf with its assigned user stroke (skip missing).
    let mut leaf_pairs: Vec<(&[OrientedPoint], u8)> = Vec::new();
    let mut idx = 0;
    collect_leaf_pairs(group, user_strokes, &mut idx, &mut leaf_pairs);

    let matched: Vec<(&[OrientedPoint], &Vec<OrientedPoint>)> = leaf_pairs
        .iter()
        .filter(|(_, ui)| *ui != u8::MAX)
        .map(|(rb, ui)| (*rb, &user_b[*ui as usize]))
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
    let dtw_avg = dtw_sum / f32::from(matched.len().try_into().unwrap_or(u16::MAX));

    // Kendall tau on the user-stroke indices.
    let user_indices: Vec<u8> = leaf_pairs
        .iter()
        .filter(|(_, ui)| *ui != u8::MAX)
        .map(|(_, ui)| *ui)
        .collect();
    let tau = kendall_tau(&user_indices);

    FRAME_G_WEIGHT * dtw_avg + ORDER_WEIGHT * tau
}

fn collect_leaf_pairs<'a>(
    node: &'a AnalyzedKanjiNode,
    user_strokes: &[u8],
    idx: &mut usize,
    out: &mut Vec<(&'a [OrientedPoint], u8)>,
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
fn kendall_tau(seq: &[u8]) -> f32 {
    let n = seq.len();
    if n < 2 {
        return 0.0;
    }
    let mut inv: i16 = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            if seq[i] > seq[j] {
                inv += 1;
            }
        }
    }
    let n = n.try_into().unwrap_or(u16::MAX);
    let max = n * (n - 1) / 2;
    inv as f32 / max as f32
}

fn combine_children(child_candidates: &[Vec<MatchInfo>], width: usize) -> Vec<MatchInfo> {
    let mut results: Vec<MatchInfo> = vec![MatchInfo {
        user_strokes: SmallVec::new(),
        score: 0.0,
        used_mask: 0,
    }];

    for cands in child_candidates {
        let mut next: Vec<MatchInfo> = Vec::with_capacity(results.len() * cands.len());
        for partial in &results {
            for candidate in cands {
                if (partial.used_mask & candidate.used_mask) != 0 {
                    continue;
                }
                let mut combined = partial.user_strokes.clone();
                combined.extend(candidate.user_strokes.iter().copied());
                next.push(MatchInfo {
                    user_strokes: combined,
                    score: partial.score + candidate.score,
                    used_mask: partial.used_mask | candidate.used_mask,
                });
            }
        }

        let keep = (width * 4).min(next.len());
        if keep < next.len() {
            next.select_nth_unstable_by(keep, |a, b| a.score.partial_cmp(&b.score).unwrap());
            next.truncate(keep);
        }
        next.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        results = truncate_with_permutation_cap(next, width);
    }

    results
}

fn truncate_with_permutation_cap(entries: Vec<MatchInfo>, width: usize) -> Vec<MatchInfo> {
    let mut group_counts: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut kept: Vec<MatchInfo> = Vec::with_capacity(width);
    for entry in entries {
        let cap = entry.user_strokes.len().max(1);
        let count = group_counts.entry(entry.used_mask).or_insert(0);
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
        assert_eq!(result[0].user_strokes.as_slice(), vec![0, 1, 2]);
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
        assert_eq!(result[0].user_strokes.as_slice(), vec![2, 1, 0]);
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
        assert_eq!(result[0].user_strokes.as_slice(), vec![0, 1, 2]);
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
        assert_eq!(result[0].user_strokes.as_slice(), vec![0, u8::MAX, 1]);
    }
}
