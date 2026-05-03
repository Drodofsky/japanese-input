use smallvec::{SmallVec, smallvec};

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

pub type StrokeVec = SmallVec<[u8; 32]>;

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub user_strokes: StrokeVec,
    pub score: f32,
}
#[derive(Default)]
struct Scratch {
    /// Reused inside `combine_children` as the next-generation buffer.
    next: Vec<MatchInfo>,
    /// Reused inside `beam`'s group arm to collect children's candidate lists.
    child_candidates: Vec<Vec<MatchInfo>>,
}
pub fn match_node(reference: &AnalyzedKanjiNode, user: &[Vec<(f32, f32)>]) -> Vec<MatchInfo> {
    let (user_b, user_c) = prepare_user(user);
    let mut scratch = Scratch::default();
    let mut results = beam(reference, &user_b, &user_c, 10, &mut scratch);

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
    for r in results.iter_mut() {
        let used = r
            .user_strokes
            .iter()
            .copied()
            .filter(|&i| i != u8::MAX)
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

/// Process a single stroke leaf into beam candidates.
fn beam_stroke(
    ref_b: &[OrientedPoint],
    ref_c: &[OrientedPoint],
    user_b: &[Vec<OrientedPoint>],
    user_c: &[Vec<OrientedPoint>],
    width: usize,
) -> Vec<MatchInfo> {
    let weights = DtwWeights::default();
    let ref_len = bbox_longer_side(ref_b);

    let mut candidates: Vec<MatchInfo> = (0..user_b.len())
        .map(|i| {
            let s_b = dtw(ref_b, &user_b[i], weights);
            let s_c = dtw(ref_c, &user_c[i], weights);
            let s_len = (ref_len - bbox_longer_side(&user_b[i])).abs();
            let combined = FRAME_B_WEIGHT * s_b + FRAME_C_WEIGHT * s_c + LENGTH_WEIGHT * s_len;
            MatchInfo {
                user_strokes: smallvec![i as u8],
                score: combined,
            }
        })
        .collect();

    candidates.push(MatchInfo {
        user_strokes: smallvec![u8::MAX],
        score: MISSING_PENALTY,
    });
    candidates.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    candidates.truncate(width);
    candidates
}

fn beam(
    node: &AnalyzedKanjiNode,
    user_b: &[Vec<OrientedPoint>],
    user_c: &[Vec<OrientedPoint>],
    width: usize,
    scratch: &mut Scratch,
) -> Vec<MatchInfo> {
    // Iterative post-order traversal.
    //
    // Pass 1: build a flat list of nodes in post-order (children before parents).
    // Pass 2: walk the list bottom-up, computing each node's beam candidates from
    //         its already-computed children's candidates.
    //
    // Each node carries the `width` it should be processed with (for the root) or
    // inherits the width passed down from its parent's retry loop.

    // ---- Pass 1: post-order list ----
    // We use a stack of (node, visited?). On first visit, push self back as visited
    // and push children. On second visit, emit.
    // Each entry: the node, plus (for groups) the post-order indices of its children.
    let mut post_order: Vec<&AnalyzedKanjiNode> = Vec::new();
    let mut child_indices: Vec<Vec<usize>> = Vec::new();

    // Recursive helper, but only for the cheap pass-1 walk (no DTW work here).
    fn walk<'a>(
        n: &'a AnalyzedKanjiNode,
        post_order: &mut Vec<&'a AnalyzedKanjiNode>,
        child_indices: &mut Vec<Vec<usize>>,
    ) -> usize {
        let kids = match n {
            AnalyzedKanjiNode::Stroke { .. } => Vec::new(),
            AnalyzedKanjiNode::Group { children, .. } => children
                .iter()
                .map(|c| walk(c, post_order, child_indices))
                .collect(),
        };
        let idx = post_order.len();
        post_order.push(n);
        child_indices.push(kids);
        idx
    }

    walk(node, &mut post_order, &mut child_indices);
    // ---- Pass 2: compute candidates bottom-up ----
    // Map from node pointer to its computed candidates. Using a pointer key works
    // because every node in `post_order` is a distinct borrow within `node`'s tree.
    let mut results_by_idx: Vec<Vec<MatchInfo>> = Vec::with_capacity(post_order.len());
    // The width to apply when processing each node. The root gets `width`; non-root
    // nodes get whatever their parent's retry loop currently demands. We track this
    // via a separate map populated as we descend logically — but since we're going
    // bottom-up, we instead handle the retry locally per group (see below).
    //
    // For non-group leaves and for groups whose initial child width suffices, this
    // matches the recursive version exactly. When a group's combine yields empty,
    // we recompute its children with a doubled width — this is the only place where
    // a child may need to be re-processed, so we do it inline.

    for (i, n) in post_order.iter().enumerate() {
        match n {
            AnalyzedKanjiNode::Stroke {
                in_kanji_frame: ref_b,
                in_stroke_frame: ref_c,
                ..
            } => {
                // Default leaf width is the global `width`. Groups that need a wider
                // beam will recompute their children inline below, overwriting this.
                let cands = beam_stroke(ref_b, ref_c, user_b, user_c, width);
                debug_assert_eq!(results_by_idx.len(), i);
                results_by_idx.push(cands);
            }

            AnalyzedKanjiNode::Group { children, .. } => {
                let mut try_width = width;

                let combined = loop {
                    // Gather child candidates at the current try_width. If try_width
                    // equals the width already used for the children, reuse; otherwise
                    // recompute the children at the new width.
                    let mut child_candidates = std::mem::take(&mut scratch.child_candidates);
                    child_candidates.clear();
                    if try_width == width {
                        for &ci in &child_indices[i] {
                            child_candidates.push(std::mem::take(&mut results_by_idx[ci]));
                        }
                    } else {
                        for child in children.iter() {
                            child_candidates.push(beam(child, user_b, user_c, try_width, scratch));
                        }
                    }

                    let combined =
                        combine_children(&child_candidates, try_width, &mut scratch.next);
                    scratch.child_candidates = child_candidates;
                    if !combined.is_empty() || try_width >= MAX_WIDTH {
                        break combined;
                    }
                    try_width *= 2;
                };

                let mut results = combined;

                // Group-level extras: frame G + order continuity
                for r in results.iter_mut() {
                    let matched: Vec<u8> = r
                        .user_strokes
                        .iter()
                        .copied()
                        .filter(|&i| i != u8::MAX)
                        .collect();
                    if matched.len() < 2 {
                        continue;
                    }
                    let extra = group_extras(n, &r.user_strokes, user_b);
                    r.score += extra;
                }

                results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
                let results = truncate_with_permutation_cap(results, width);
                debug_assert_eq!(results_by_idx.len(), i);
                results_by_idx.push(results);
            }
        }
    }

    // The root is the last node in post-order.
    results_by_idx.pop().unwrap_or_default()
}

/// Computes frame-G DTW + Kendall-tau order penalty, weighted.
fn group_extras(
    group: &AnalyzedKanjiNode,
    user_strokes: &[u8],
    user_b: &[Vec<OrientedPoint>],
) -> f32 {
    // Walk the subtree, pair each leaf with its assigned user stroke (skip missing).
    let mut leaf_pairs: Vec<(&[OrientedPoint], u8)> = Vec::new();
    collect_leaf_pairs(group, user_strokes, &mut leaf_pairs);

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
    let dtw_avg = dtw_sum / matched.len() as f32;

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
    out: &mut Vec<(&'a [OrientedPoint], u8)>,
) {
    // Iterative left-to-right DFS over leaves. We push children in reverse so the
    // leftmost child is processed first; `idx` advances exactly when we emit a leaf,
    // matching the recursive version's traversal order.
    let mut idx: usize = 0;
    let mut stack: Vec<&AnalyzedKanjiNode> = vec![node];
    while let Some(n) = stack.pop() {
        match n {
            AnalyzedKanjiNode::Stroke { in_kanji_frame, .. } => {
                out.push((in_kanji_frame.as_slice(), user_strokes[idx]));
                idx += 1;
            }
            AnalyzedKanjiNode::Group { children, .. } => {
                for c in children.iter().rev() {
                    stack.push(c);
                }
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

fn combine_children(
    child_candidates: &[Vec<MatchInfo>],
    width: usize,
    next: &mut Vec<MatchInfo>,
) -> Vec<MatchInfo> {
    let mut results: Vec<MatchInfo> = vec![MatchInfo {
        user_strokes: SmallVec::new(),
        score: 0.0,
    }];

    for cands in child_candidates {
        next.clear();
        for partial in &results {
            for candidate in cands {
                let overlaps = candidate
                    .user_strokes
                    .iter()
                    .filter(|&&i| i != u8::MAX)
                    .any(|s| partial.user_strokes.contains(s));
                if overlaps {
                    continue;
                }
                let mut combined = partial.user_strokes.clone();
                combined.extend(candidate.user_strokes.clone());
                next.push(MatchInfo {
                    user_strokes: combined,
                    score: partial.score + candidate.score,
                });
            }
        }
        next.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        let taken = std::mem::take(next);
        results = truncate_with_permutation_cap(taken, width);
        // `next` is now empty; capacity will be re-grown as needed on next iteration.
        // (truncate_with_permutation_cap consumes its input, so we can't keep capacity here.)
    }

    results
}

fn truncate_with_permutation_cap(entries: Vec<MatchInfo>, width: usize) -> Vec<MatchInfo> {
    let mut group_counts: std::collections::HashMap<StrokeVec, usize> =
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
