use crate::KanjiNode;
use crate::bbox::GenBBox;
use crate::dtw::{DtwWeights, dtw_with_path};
use crate::match_node::{MatchInfo, match_node};
use crate::normalize::Normalize;
use crate::point::ToOriented;

mod correction;
pub mod node;
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
    PositionCorrection { depth: usize, score: f32 },
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
    pub user_strokes: Vec<Vec<(f32, f32)>>,
    pub corrected_strokes: Vec<Vec<(f32, f32)>>,
}

impl Analysis {
    fn empty() -> Self {
        Analysis {
            issues: vec![],
            score: 0.0,
            stroke_qualities: vec![],
            user_strokes: vec![],
            corrected_strokes: vec![],
        }
    }
}

#[must_use]
pub fn analyze(reference: &KanjiNode, user_strokes: &[Vec<(f32, f32)>]) -> Analysis {
    if user_strokes.is_empty() {
        return Analysis::empty();
    }

    let analyzed = AnalyzedKanjiNode::from_node(reference);
    let mut working: Vec<Vec<(f32, f32)>> = user_strokes.to_vec();

    let Some(first_match) = best_match(&analyzed, user_strokes) else {
        return Analysis::empty();
    };

    let was_wrong_order = is_out_of_order(&first_match.user_strokes);
    let mut issues: Vec<IssueWithFix> = Vec::new();

    // Stage 1: structural corrections (missing + extra strokes)
    insert_missing_strokes(
        &first_match,
        &analyzed,
        user_strokes,
        &mut working,
        &mut issues,
    );
    remove_extra_strokes(&first_match, user_strokes.len(), &mut working, &mut issues);

    // Stage 2: position corrections (parent-relative, outer-first)
    apply_position_corrections(&analyzed, &mut working, &mut issues);

    // Stage 3: wrong order
    let score = fix_wrong_order(was_wrong_order, &analyzed, &mut working, &mut issues);

    // Stage 4: per-point shape quality
    let stroke_qualities = compute_stroke_qualities(&analyzed, &working);

    Analysis {
        user_strokes: user_strokes.to_vec(),
        corrected_strokes: working.clone(),
        issues,
        score,
        stroke_qualities,
    }
}

// ── Private helpers ──────────────────────────────────────────────────────────

fn best_match(analyzed: &AnalyzedKanjiNode, strokes: &[Vec<(f32, f32)>]) -> Option<MatchInfo> {
    match_node(analyzed, strokes).into_iter().next()
}

/// Returns `true` if the matched (non-sentinel) indices are not in ascending order.
fn is_out_of_order(assignment: &[u8]) -> bool {
    let indices: Vec<u8> = assignment
        .iter()
        .copied()
        .filter(|&i| i != u8::MAX)
        .collect();
    indices.windows(2).any(|w| w[0] > w[1])
}

/// Stage 1a — insert placeholder strokes for each reference stroke missing from the drawing.
///
/// Each placeholder is the reference stroke mapped from frame B ([0,1]²) into the
/// user's raw coordinate space via the user's kanji bounding box.
fn insert_missing_strokes(
    best: &MatchInfo,
    analyzed: &AnalyzedKanjiNode,
    user_strokes: &[Vec<(f32, f32)>],
    working: &mut Vec<Vec<(f32, f32)>>,
    issues: &mut Vec<IssueWithFix>,
) {
    let ref_leaves = collect_kanji_frame_strokes(analyzed);
    let user_bbox = user_strokes.gen_bbox();

    for (ref_pos, &user_idx) in best.user_strokes.iter().enumerate() {
        if user_idx == u8::MAX {
            let inserted: Vec<(f32, f32)> = ref_leaves[ref_pos]
                .iter()
                .map(|op| {
                    (
                        user_bbox.min.x + op.position.x * user_bbox.width(),
                        user_bbox.min.y + op.position.y * user_bbox.height(),
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
}

/// Stage 1b — remove strokes that were not matched to any reference stroke.
///
/// Extras are removed highest-index-first so that earlier indices stay stable.
fn remove_extra_strokes(
    best: &MatchInfo,
    user_stroke_count: usize,
    working: &mut Vec<Vec<(f32, f32)>>,
    issues: &mut Vec<IssueWithFix>,
) {
    let matched: std::collections::HashSet<u8> = best
        .user_strokes
        .iter()
        .copied()
        .filter(|&i| i != u8::MAX)
        .collect();
    let mut extras: Vec<u8> = (0..user_stroke_count)
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
}

/// Stage 2 — nudge strokes toward their expected positions, one tree depth at a time.
fn apply_position_corrections(
    analyzed: &AnalyzedKanjiNode,
    working: &mut [Vec<(f32, f32)>],
    issues: &mut Vec<IssueWithFix>,
) {
    let assignment = best_match(analyzed, working).map_or_else(
        || {
            (0..working.len())
                .map(|i| i.try_into().unwrap_or(u8::MAX))
                .collect()
        },
        |m| m.user_strokes.to_vec(),
    );

    let max_depth = tree_depth(analyzed);
    let mut max_score = 0.0_f32;
    for depth in 0..=max_depth {
        let score = apply_level_correction(analyzed, &assignment, working, depth, 0);
        max_score = max_score.max(score);
    }
    // only push big issues
    if max_score > 0.3 {
        issues.push(IssueWithFix {
            issue: StrokeIssue::PositionCorrection {
                depth: max_depth,
                score: max_score,
            },
            corrected_strokes: working.to_owned(),
        });
    }
}

/// Stage 3 — reorder strokes if they are out of order. Returns the final match score.
fn fix_wrong_order(
    was_wrong_order: bool,
    analyzed: &AnalyzedKanjiNode,
    working: &mut Vec<Vec<(f32, f32)>>,
    issues: &mut Vec<IssueWithFix>,
) -> f32 {
    let Some(best) = best_match(analyzed, working) else {
        return 0.0;
    };
    let score = best.score;

    if is_out_of_order(&best.user_strokes) {
        let old = working.clone();
        *working = best
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
    score
}

/// Stage 4 — compute per-point quality scores for each reference stroke.
fn compute_stroke_qualities(
    analyzed: &AnalyzedKanjiNode,
    working: &[Vec<(f32, f32)>],
) -> Vec<Vec<f32>> {
    let ref_leaves = collect_kanji_frame_strokes(analyzed);
    let final_assignment = best_match(analyzed, working).map_or_else(
        || vec![u8::MAX; ref_leaves.len()],
        |m| m.user_strokes.to_vec(),
    );

    let ref_in_stroke_frame = collect_stroke_frame_strokes(analyzed);

    ref_in_stroke_frame
        .iter()
        .zip(final_assignment.iter())
        .map(|(ref_c, &user_idx)| {
            if user_idx == u8::MAX {
                return Vec::new();
            }
            let Some(stroke) = working.get(user_idx as usize) else {
                return Vec::new();
            };
            let user_c = stroke.as_slice().to_oriented().normalize();
            let (_score, path) = dtw_with_path(ref_c, &user_c, DtwWeights::default());
            aggregate_per_user_point(&path, user_c.len())
        })
        .collect()
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
        let n = 20_u8;
        (0..=n)
            .map(|i| {
                let t = f32::from(i) / f32::from(n);
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
