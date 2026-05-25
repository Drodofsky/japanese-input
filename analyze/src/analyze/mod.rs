use crate::KanjiNode;
use crate::bbox::GenBBox;
use crate::dtw::dtw_with_path;
use crate::match_node::match_node;
use crate::normalize::Normalize;
use crate::point::ToOriented;

pub mod node;
mod correction;
mod quality;
mod tree;

pub use node::AnalyzedKanjiNode;

use correction::apply_level_correction;
use quality::aggregate_per_user_point;
use tree::{collect_kanji_frame_strokes, collect_stroke_frame_strokes, tree_depth};

#[derive(Debug, Clone, PartialEq)]
pub enum StrokeIssue {
    Missing { ref_index: usize },
    WrongOrder,
    Extra { user_index: usize },
    PositionCorrection { depth: usize },
}

#[derive(Debug, Clone)]
pub struct IssueWithFix {
    pub issue: StrokeIssue,
    pub corrected_strokes: Vec<Vec<(f32, f32)>>,
}

#[derive(Debug, Clone)]
pub struct Analysis {
    pub issues: Vec<IssueWithFix>,
    pub score: f32,
    pub stroke_qualities: Vec<Vec<f32>>,
}

#[must_use]
pub fn analyze(reference: &KanjiNode, user_strokes: &[Vec<(f32, f32)>]) -> Analysis {
    if user_strokes.is_empty() {
        return Analysis {
            issues: vec![],
            score: 0.0,
            stroke_qualities: vec![],
        };
    }

    let analyzed = AnalyzedKanjiNode::from_node(reference);

    // Work in user raw-space throughout. No normalization.
    let mut working: Vec<Vec<(f32, f32)>> = user_strokes.to_vec();

    let results = match_node(&analyzed, user_strokes);

    if results.is_empty() {
        return Analysis {
            issues: vec![],
            score: 0.0,
            stroke_qualities: vec![],
        };
    }
    let best = &results[0];

    let original_indices: Vec<u8> = best
        .user_strokes
        .iter()
        .copied()
        .filter(|&i| i != u8::MAX)
        .collect();
    let was_wrong_order = original_indices.windows(2).any(|w| w[0] > w[1]);

    let mut issues: Vec<IssueWithFix> = Vec::new();

    // User's whole-kanji bbox in raw space — used to map frame-B placeholder
    // coordinates into user-space when inserting missing strokes.
    let user_kanji_bbox = user_strokes.to_vec().gen_bbox();

    // ── Stage 1a: missing strokes ────────────────────────────────────────────
    let ref_leaves = collect_kanji_frame_strokes(&analyzed);
    for (ref_pos, &user_idx) in best.user_strokes.iter().enumerate() {
        if user_idx == u8::MAX {
            // Map the reference stroke (in frame B [0,1]) into user-space
            // through the user's kanji bbox.
            let inserted: Vec<(f32, f32)> = ref_leaves[ref_pos]
                .iter()
                .map(|op| {
                    (
                        user_kanji_bbox.min.x + op.position.x * user_kanji_bbox.width(),
                        user_kanji_bbox.min.y + op.position.y * user_kanji_bbox.height(),
                    )
                })
                .collect();
            working.push(inserted);
            issues.push(IssueWithFix {
                issue: StrokeIssue::Missing { ref_index: ref_pos },
                corrected_strokes: working.clone(),
            });
        }
    }

    // ── Stage 1b: extra strokes ──────────────────────────────────────────────
    let matched: std::collections::HashSet<u8> = best
        .user_strokes
        .iter()
        .copied()
        .filter(|&i| i != u8::MAX)
        .collect();
    let mut extras: Vec<u8> = (0..user_strokes.len())
        .filter(|i| !matched.contains(&((*i).try_into().unwrap_or(u8::MAX))))
        .map(|i| i.try_into().unwrap_or(u8::MAX))
        .collect();
    extras.sort_by(|a, b| b.cmp(a));

    for user_index in extras {
        if (user_index as usize) < working.len() {
            working.remove(user_index.into());
        }
        issues.push(IssueWithFix {
            issue: StrokeIssue::Extra {
                user_index: user_index.into(),
            },
            corrected_strokes: working.clone(),
        });
    }

    // ── Stage 2: position corrections (parent-relative, outer-first) ─────────
    let mid_match = match_node(&analyzed, &working);
    let assignment_for_levels: Vec<_> = if mid_match.is_empty() {
        (0..working.len())
            .map(|i| i.try_into().unwrap_or(u8::MAX))
            .collect()
    } else {
        mid_match[0].user_strokes.to_vec()
    };

    let max_depth = tree_depth(&analyzed);

    // Depth 0 is a no-op (root has no parent above it). Start from depth 1.
    for depth in 0..=max_depth {
        apply_level_correction(&analyzed, &assignment_for_levels, &mut working, depth, 0);
    }
    issues.push(IssueWithFix {
        issue: StrokeIssue::PositionCorrection { depth: max_depth },
        corrected_strokes: working.clone(),
    });

    // ── Stage 3: wrong order ─────────────────────────────────────────────────
    let results2 = match_node(&analyzed, &working);

    let final_score = if results2.is_empty() {
        0.0
    } else {
        results2[0].score
    };

    if !results2.is_empty() {
        let best2 = &results2[0];
        let indices2: Vec<u8> = best2
            .user_strokes
            .iter()
            .copied()
            .filter(|&i| i != u8::MAX)
            .collect();

        if indices2.windows(2).any(|w| w[0] > w[1]) {
            let old = working.clone();
            working = best2
                .user_strokes
                .iter()
                .filter(|&&i| i != u8::MAX)
                .filter_map(|&i| old.get(i as usize).cloned())
                .collect();

            if was_wrong_order {
                issues.push(IssueWithFix {
                    issue: StrokeIssue::WrongOrder,
                    corrected_strokes: working.clone(),
                });
            }
        }
    }

    // ── Stage 4: per-point shape quality ─────────────────────────────────
    let final_match = match_node(&analyzed, &working);

    let final_assignment: Vec<u8> = if final_match.is_empty() {
        vec![u8::MAX; ref_leaves.len()]
    } else {
        final_match[0].user_strokes.to_vec()
    };

    let ref_in_stroke_frame = collect_stroke_frame_strokes(&analyzed);

    let stroke_qualities: Vec<Vec<f32>> = ref_in_stroke_frame
        .iter()
        .zip(final_assignment.iter())
        .map(|(ref_c, &user_idx)| {
            if user_idx == u8::MAX {
                return Vec::new();
            }
            let Some(stroke) = working.get(user_idx as usize) else {
                return Vec::new();
            };

            let oriented = stroke.as_slice().to_oriented();
            let user_c = vec![oriented].normalize().pop().unwrap_or_default();

            let (_score, path) = dtw_with_path(ref_c, &user_c, crate::dtw::DtwWeights::default());
            aggregate_per_user_point(&path, user_c.len())
        })
        .collect();

    Analysis {
        issues,
        score: final_score,
        stroke_qualities,
    }
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

    fn three_kanji() -> KanjiNode {
        KanjiNode::Group {
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
        }
    }

    fn structural_issues(a: &Analysis) -> Vec<&StrokeIssue> {
        a.issues
            .iter()
            .map(|i| &i.issue)
            .filter(|i| !matches!(i, StrokeIssue::PositionCorrection { .. }))
            .collect()
    }

    #[test]
    fn empty_user_returns_no_issues() {
        let result = analyze(&three_kanji(), &[]);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn correct_drawing_has_no_structural_issues() {
        let user = vec![
            user_line(20.0, 20.0, 80.0, 20.0),
            user_line(20.0, 50.0, 80.0, 50.0),
            user_line(20.0, 80.0, 80.0, 80.0),
        ];
        let result = analyze(&three_kanji(), &user);
        assert!(structural_issues(&result).is_empty());
    }

    #[test]
    fn missing_middle_stroke_is_reported() {
        let user = vec![
            user_line(20.0, 20.0, 80.0, 20.0),
            user_line(20.0, 80.0, 80.0, 80.0),
        ];
        let result = analyze(&three_kanji(), &user);
        let structural = structural_issues(&result);
        assert!(matches!(
            structural[0],
            StrokeIssue::Missing { ref_index: 1 }
        ));
    }

    #[test]
    fn extra_stroke_is_reported() {
        let user = vec![
            user_line(20.0, 20.0, 80.0, 20.0),
            user_line(20.0, 50.0, 80.0, 50.0),
            user_line(20.0, 80.0, 80.0, 80.0),
            user_line(50.0, 50.0, 50.0, 90.0),
        ];
        let result = analyze(&three_kanji(), &user);
        let structural = structural_issues(&result);
        assert!(matches!(
            structural[0],
            StrokeIssue::Extra { user_index: 3 }
        ));
    }

    #[test]
    fn wrong_order_is_reported() {
        let user = vec![
            user_line(20.0, 80.0, 80.0, 80.0),
            user_line(20.0, 50.0, 80.0, 50.0),
            user_line(20.0, 20.0, 80.0, 20.0),
        ];
        let result = analyze(&three_kanji(), &user);
        let structural = structural_issues(&result);
        assert!(
            structural
                .iter()
                .any(|i| matches!(i, StrokeIssue::WrongOrder))
        );
    }
}
