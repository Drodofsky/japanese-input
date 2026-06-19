use core::f64;
use smallvec::{SmallVec, smallvec};
use std::collections::HashMap;

use crate::{
    analyzed_kanji_node::AnalyzedKanjiNode,
    convert_lossy::ConvertLossy as _,
    convert_stroke_index::ConvertStrokeIndex as _,
    dtw::{DTWWeights, dtw},
    leaf_matrix::LeafMatrix,
    normalize::Normalize as _,
    stroke_geometry::StrokeGeometry,
    stroke_point::StrokePoint,
};
const MAX_WIDTH: usize = 50000;
const PRE_FILTER_WIDTH_MULTIPLIER: usize = 2;
const EXTRA_PENALTY: f64 = 1.0;

#[derive(Debug, Clone, Copy)]
pub struct Weights {
    missing_penalty: f64,
    length_weight: f64,
    order_weight: f64,
    kanji_dtw_weights: DTWWeights,
    stroke_dtw_weights: DTWWeights,
    group_weight: f64,
}

impl TryFrom<&[f64]> for Weights {
    type Error = String;
    fn try_from(value: &[f64]) -> Result<Self, Self::Error> {
        Ok(Weights {
            missing_penalty: *value
                .get(0)
                .ok_or::<String>("weights could not be converted".into())?,
            length_weight: *value
                .get(1)
                .ok_or::<String>("weights could not be converted".into())?,
            order_weight: *value
                .get(2)
                .ok_or::<String>("weights could not be converted".into())?,
            kanji_dtw_weights: DTWWeights {
                position: *value
                    .get(3)
                    .ok_or::<String>("weights could not be converted".into())?,
                tangent: *value
                    .get(4)
                    .ok_or::<String>("weights could not be converted".into())?,
            },
            stroke_dtw_weights: DTWWeights {
                position: *value
                    .get(5)
                    .ok_or::<String>("weights could not be converted".into())?,
                tangent: *value
                    .get(6)
                    .ok_or::<String>("weights could not be converted".into())?,
            },
            group_weight: *value
                .get(7)
                .ok_or::<String>("weights could not be converted".into())?,
        })
    }
}

impl Default for Weights {
    #[inline]
    fn default() -> Self {
        Weights::v1()
    }
}

impl Weights {
    #[must_use]
    #[inline]
    pub fn ones() -> Self {
        Self {
            missing_penalty: 1.0,
            length_weight: 1.0,
            order_weight: 1.0,
            kanji_dtw_weights: DTWWeights {
                position: 1.0,
                tangent: 1.0,
            },
            stroke_dtw_weights: DTWWeights {
                position: 1.0,
                tangent: 1.0,
            },
            group_weight: 1.0,
        }
    }
    #[must_use]
    #[inline]
    pub fn v1() -> Self {
        Self {
            missing_penalty: 2.2505911627725426,
            length_weight: 0.7371296007755844,
            order_weight: 1.0182675079160632,
            kanji_dtw_weights: DTWWeights {
                position: 2.9998551498526016,
                tangent: 0.27604052730957745,
            },
            stroke_dtw_weights: DTWWeights {
                position: 2.908390915339651,
                tangent: 1.1446443685757839,
            },
            group_weight: 3.0010654971186557,
        }
    }
}

pub type StrokeVec = SmallVec<[u8; 32]>;
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub user_stroke_order: StrokeVec,
    pub score: f64,
    pub used_mask: u32,
    pub beam_width: usize,
}

#[inline]
#[must_use]
pub fn match_strokes(
    kanji_tree: AnalyzedKanjiNode,
    user_strokes: Vec<Vec<StrokePoint>>,
    weights: Weights,
    beam_with: usize,
) -> Vec<MatchInfo> {
    let leaf_score = |a: &[StrokePoint], b: &[StrokePoint]| -> f64 {
        dtw(a, b, &weights.kanji_dtw_weights)
            + dtw(
                &a.normalized(),
                &b.normalized(),
                &weights.stroke_dtw_weights,
            )
            + weights.length_weight * (arc_len(a) - arc_len(b)).abs()
    };

    let leaf_matrix = LeafMatrix::build(
        &user_strokes,
        &kanji_tree.collect_strokes(),
        weights.missing_penalty,
        leaf_score,
    );
    let user_stroke_geometries: Vec<StrokeGeometry> = user_strokes
        .iter()
        .map(|s| StrokeGeometry::from_stroke(s))
        .collect();
    let mut results = beam(
        &kanji_tree,
        &user_stroke_geometries,
        &leaf_matrix,
        beam_with,
        weights,
    );
    let user_count = user_strokes.len();
    for r in &mut results {
        let used = r
            .user_stroke_order
            .iter()
            .copied()
            .filter(|&i| i != u8::MAX)
            .count();
        let extras = user_count.saturating_sub(used);
        r.score += EXTRA_PENALTY * f64::from(extras.try_into().unwrap_or(u16::MAX));
    }

    results.sort_by(|a, b| a.score.total_cmp(&b.score));
    results
}

#[inline]
fn arc_len(points: &[StrokePoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().skip(1))
        .map(|(a, b)| a.position.distance(b.position))
        .sum()
}

fn beam(
    group_tree: &AnalyzedKanjiNode,
    user_stroke_geometries: &[StrokeGeometry],
    leaf_matrix: &LeafMatrix,
    width: usize,
    weights: Weights,
) -> Vec<MatchInfo> {
    match group_tree {
        AnalyzedKanjiNode::Stroke { index, .. } => {
            let mut candidates: Vec<MatchInfo> = (0..leaf_matrix.user_stroke_count())
                .map(|user_stroke_index| MatchInfo {
                    user_stroke_order: smallvec![user_stroke_index.convert_stroke_index()],
                    score: leaf_matrix.cost((*index).into(), user_stroke_index),
                    used_mask: 1_u32 << user_stroke_index.convert_stroke_index(),
                    beam_width: width,
                })
                .collect();
            // insert ghost stroke
            candidates.push(MatchInfo {
                user_stroke_order: smallvec![u8::MAX],
                score: leaf_matrix.cost((*index).into(), leaf_matrix.user_stroke_count()),
                used_mask: 0,
                beam_width: width,
            });
            candidates.sort_by(|a, b| a.score.total_cmp(&b.score));
            candidates.truncate(width);
            candidates
        }
        AnalyzedKanjiNode::Group { children, .. } => {
            let mut current_width = width;
            let mut results = loop {
                let child_candidates = children.iter().map(|child| {
                    beam(
                        child,
                        user_stroke_geometries,
                        leaf_matrix,
                        current_width,
                        weights,
                    )
                });
                let combined = combine_children(
                    child_candidates,
                    current_width.saturating_mul(PRE_FILTER_WIDTH_MULTIPLIER),
                );
                if !combined.is_empty() || current_width >= MAX_WIDTH {
                    break combined;
                }
                current_width = current_width.saturating_mul(2);
            };
            for result in &mut results {
                let group_score = score_group(
                    group_tree,
                    &result.user_stroke_order,
                    user_stroke_geometries,
                    weights,
                );
                result.score += group_score;
            }
            results.sort_by(|a, b| a.score.total_cmp(&b.score));
            truncate_with_permutation_cap(results, current_width) // TODO: try to change to width later
        }
    }
}
fn score_group(
    group: &AnalyzedKanjiNode,
    user_stroke_order: &[u8],
    user_stroke_geometries: &[StrokeGeometry],
    weights: Weights,
) -> f64 {
    let (reference_stroke_geometries, user_stroke_geometries): (
        Vec<StrokeGeometry>,
        Vec<StrokeGeometry>,
    ) = group
        .collect_geometry()
        .into_iter()
        .enumerate()
        .filter_map(|(local_index, reference_geometries)| {
            if let Some(user_index) = user_stroke_order.get(local_index)
                && *user_index != u8::MAX
            {
                Some((
                    reference_geometries,
                    user_stroke_geometries.get(usize::from(*user_index))?,
                ))
            } else {
                None
            }
        })
        .unzip();
    let reference_stroke_centroids = reference_stroke_geometries.normalized();
    let user_stroke_centroids = user_stroke_geometries.normalized();
    let group_centroid_diff_score: f64 = reference_stroke_centroids
        .iter()
        .zip(user_stroke_centroids.iter())
        .map(|(reference_centroid, user_centroid)| (*reference_centroid - *user_centroid).length())
        .sum();

    let centroid_score = if reference_stroke_centroids.is_empty() {
        0.0_f64
    } else {
        group_centroid_diff_score / reference_stroke_centroids.len().convert_lossy()
    };
    let filtered_user_stroke_order: Vec<u8> = user_stroke_order
        .iter()
        .filter(|i| **i != u8::MAX)
        .copied()
        .collect();
    let order_score = kendall_tau(&filtered_user_stroke_order);

    weights.group_weight * centroid_score + weights.order_weight * order_score
}

fn combine_children(
    mut child_candidates: impl Iterator<Item = Vec<MatchInfo>>,
    width: usize,
) -> Vec<MatchInfo> {
    let mut combined: Vec<MatchInfo> =
        truncate_with_permutation_cap(child_candidates.next().unwrap_or_default(), width);

    for candidates in child_candidates {
        let mut accumulator: Vec<MatchInfo> =
            Vec::with_capacity(combined.len().saturating_mul(candidates.len()));
        for partial in &combined {
            for candidate in &candidates {
                if let Some(merged) = merge_matches(partial, candidate, width) {
                    accumulator.push(merged);
                }
            }
        }
        accumulator.sort_by(|a, b| a.score.total_cmp(&b.score));
        combined = truncate_with_permutation_cap(accumulator, width);
    }

    combined
}

fn truncate_with_permutation_cap(entries: Vec<MatchInfo>, width: usize) -> Vec<MatchInfo> {
    let mut permutation_count: HashMap<u32, usize> = HashMap::new();
    let mut kept: Vec<MatchInfo> = Vec::with_capacity(width);

    for entry in entries {
        if kept.len() >= width {
            break;
        }
        let maximum = entry.user_stroke_order.len().max(1);
        let count = permutation_count.entry(entry.used_mask).or_default();
        if *count < maximum {
            *count = count.saturating_add(1);
            kept.push(entry);
        }
    }

    kept
}
#[inline]
fn merge_matches(left: &MatchInfo, right: &MatchInfo, width: usize) -> Option<MatchInfo> {
    if left.used_mask & right.used_mask != 0 {
        return None;
    }
    let mut user_strokes = left.user_stroke_order.clone();
    user_strokes.extend(right.user_stroke_order.iter().copied());
    Some(MatchInfo {
        user_stroke_order: user_strokes,
        score: left.score + right.score,
        used_mask: left.used_mask | right.used_mask,
        beam_width: width,
    })
}

fn kendall_tau(seq: &[u8]) -> f64 {
    if seq.len() < 2 {
        return 0.0;
    }
    let inversions = seq
        .iter()
        .enumerate()
        .flat_map(|(i, a)| seq.iter().skip(i.saturating_add(1)).filter(move |b| a > b))
        .count();
    let n = seq.len();
    let max = n.saturating_mul(n.saturating_sub(1)).saturating_div(2);
    inversions.convert_lossy() / max.convert_lossy()
}
