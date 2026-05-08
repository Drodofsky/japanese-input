use crate::{
    analyzed_kanji_node::AnalyzedKanjiNode,
    dtw::{DtwWeights, dtw},
    match_node::MISSING_PENALTY,
    point::OrientedPoint,
};
const FRAME_B_WEIGHT: f32 = 0.3;
const FRAME_C_WEIGHT: f32 = 0.3;
const LENGTH_WEIGHT: f32 = 0.4;

/// Precomputed leaf-level match scores. For each (leaf, `user_stroke`) pair plus
/// a MISSING entry per leaf, holds the combined score (frame B DTW + frame C DTW
/// + length difference). Built once per `match_node` call; consumed by `beam_stroke`.
pub struct LeafMatrix {
    n_user: u8,
    n_leaves: u8,
    /// Flat array, row-major. Row = leaf index, column = user stroke index in
    /// `0..n_user`, with column `n_user` holding the `MISSING_PENALTY` for that leaf.
    /// Stride = `n_user` + 1.
    scores: Vec<f32>,
}

impl LeafMatrix {
    #[must_use]
    pub fn create(
        root: &AnalyzedKanjiNode,
        user_b: &[Vec<OrientedPoint>],
        user_c: &[Vec<OrientedPoint>],
    ) -> Self {
        // Find max leaf index so we know how big the matrix needs to be.
        let mut max_idx = 0;
        fn scan(n: &AnalyzedKanjiNode, max_idx: &mut u8) {
            match n {
                AnalyzedKanjiNode::Stroke { index, .. } => {
                    if *index > *max_idx {
                        *max_idx = *index;
                    }
                }
                AnalyzedKanjiNode::Group { children, .. } => {
                    for c in children {
                        scan(c, max_idx);
                    }
                }
            }
        }
        scan(root, &mut max_idx);
        let n_leaves = max_idx + 1;
        let n_user = user_b.len().try_into().unwrap_or(u8::MAX);
        let stride = n_user + 1;

        let weights = DtwWeights::default();
        let user_lens: Vec<f32> = user_b.iter().map(|s| bbox_longer_side(s)).collect();

        let mut scores = vec![0.0f32; usize::from(n_leaves) * usize::from(stride)];

        // Walk leaves and fill rows.
        fn fill(
            n: &AnalyzedKanjiNode,
            user_b: &[Vec<OrientedPoint>],
            user_c: &[Vec<OrientedPoint>],
            user_lens: &[f32],
            weights: DtwWeights,
            stride: u8,
            scores: &mut [f32],
        ) {
            match n {
                AnalyzedKanjiNode::Stroke {
                    index,
                    in_kanji_frame: ref_b,
                    in_stroke_frame: ref_c,
                } => {
                    let ref_len = bbox_longer_side(ref_b);
                    let row_base: u8 = *index * stride;
                    for (u, ub) in user_b.iter().enumerate() {
                        let s_b = dtw(ref_b, ub, weights);
                        let s_c = dtw(ref_c, &user_c[u], weights);
                        let s_len = (ref_len - user_lens[u]).abs();
                        scores[usize::from(row_base) + u] =
                            FRAME_B_WEIGHT * s_b + FRAME_C_WEIGHT * s_c + LENGTH_WEIGHT * s_len;
                    }
                    // MISSING entry in the trailing column.
                    scores[usize::from(row_base) + usize::from(stride) - 1] = MISSING_PENALTY;
                }
                AnalyzedKanjiNode::Group { children, .. } => {
                    for c in children {
                        fill(c, user_b, user_c, user_lens, weights, stride, scores);
                    }
                }
            }
        }
        fill(
            root,
            user_b,
            user_c,
            &user_lens,
            weights,
            stride,
            &mut scores,
        );

        LeafMatrix {
            n_user,
            n_leaves,
            scores,
        }
    }

    /// Score for (`leaf_index`, `user_stroke_index`). Pass `n_user` as `user_idx` for the
    /// MISSING slot.
    #[inline]
    #[must_use]
    pub fn look_up(&self, leaf_index: u8, user_idx: u8) -> f32 {
        let stride = self.n_user + 1;
        self.scores[usize::from(leaf_index) * usize::from(stride) + usize::from(user_idx)]
    }

    #[inline]
    #[must_use]
    pub fn n_user(&self) -> u8 {
        self.n_user
    }
    #[must_use]
    pub fn n_leaves(&self) -> u8 {
        self.n_leaves
    }
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
