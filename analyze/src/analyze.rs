use std::vec;

use crate::KanjiNode;
use crate::analyzed_kanji_node::AnalyzedKanjiNode;
use crate::bbox::{BBox, GenBBox};
use crate::dtw::dtw_with_path;
use crate::match_node::match_node;
use crate::normalize::Normalize;
use crate::point::OrientedPoint;
use crate::point::ToOriented;

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

    let original_indices: Vec<usize> = best
        .user_strokes
        .iter()
        .copied()
        .filter(|&i| i != usize::MAX)
        .collect();
    let was_wrong_order = original_indices.windows(2).any(|w| w[0] > w[1]);

    let mut issues: Vec<IssueWithFix> = Vec::new();

    // User's whole-kanji bbox in raw space — used to map frame-B placeholder
    // coordinates into user-space when inserting missing strokes.
    let user_kanji_bbox = user_strokes.to_vec().gen_bbox();

    // ── Stage 1a: missing strokes ────────────────────────────────────────────
    let ref_leaves = collect_ref_leaves(&analyzed);
    for (ref_pos, &user_idx) in best.user_strokes.iter().enumerate() {
        if user_idx == usize::MAX {
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
    let matched: std::collections::HashSet<usize> = best
        .user_strokes
        .iter()
        .copied()
        .filter(|&i| i != usize::MAX)
        .collect();
    let mut extras: Vec<usize> = (0..user_strokes.len())
        .filter(|i| !matched.contains(i))
        .collect();
    extras.sort_by(|a, b| b.cmp(a));

    for user_index in extras {
        if user_index < working.len() {
            working.remove(user_index);
        }
        issues.push(IssueWithFix {
            issue: StrokeIssue::Extra { user_index },
            corrected_strokes: working.clone(),
        });
    }

    // ── Stage 2: position corrections (parent-relative, outer-first) ─────────
    let mid_match = match_node(&analyzed, &working);
    let assignment_for_levels: Vec<usize> = if mid_match.is_empty() {
        (0..working.len()).collect()
    } else {
        mid_match[0].user_strokes.clone()
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
        let indices2: Vec<usize> = best2
            .user_strokes
            .iter()
            .copied()
            .filter(|&i| i != usize::MAX)
            .collect();

        if indices2.windows(2).any(|w| w[0] > w[1]) {
            let old = working.clone();
            working = best2
                .user_strokes
                .iter()
                .filter(|&&i| i != usize::MAX)
                .filter_map(|&i| old.get(i).cloned())
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

    let final_assignment: Vec<usize> = if final_match.is_empty() {
        vec![usize::MAX; ref_leaves.len()]
    } else {
        final_match[0].user_strokes.clone()
    };

    let ref_in_stroke_frame = collect_ref_in_stroke_frame(&analyzed);

    let stroke_qualities: Vec<Vec<f32>> = ref_in_stroke_frame
        .iter()
        .zip(final_assignment.iter())
        .map(|(ref_c, &user_idx)| {
            if user_idx == usize::MAX {
                return Vec::new();
            }
            let Some(stroke) = working.get(user_idx) else {
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

// ── helpers ──────────────────────────────────────────────────────────────────

fn collect_ref_leaves(node: &AnalyzedKanjiNode) -> Vec<Vec<OrientedPoint>> {
    let mut out = Vec::new();
    walk_ref_leaves(node, &mut out);
    out
}

fn walk_ref_leaves(node: &AnalyzedKanjiNode, out: &mut Vec<Vec<OrientedPoint>>) {
    match node {
        AnalyzedKanjiNode::Stroke { in_kanji_frame, .. } => {
            out.push(in_kanji_frame.clone());
        }
        AnalyzedKanjiNode::Group { children, .. } => {
            for c in children {
                walk_ref_leaves(c, out);
            }
        }
    }
}

fn tree_depth(node: &AnalyzedKanjiNode) -> usize {
    match node {
        AnalyzedKanjiNode::Stroke { .. } => 0,
        AnalyzedKanjiNode::Group { children, .. } => {
            1 + children.iter().map(tree_depth).max().unwrap_or(0)
        }
    }
}

fn leaf_count(node: &AnalyzedKanjiNode) -> usize {
    match node {
        AnalyzedKanjiNode::Stroke { .. } => 1,
        AnalyzedKanjiNode::Group { children, .. } => children.iter().map(leaf_count).sum(),
    }
}

/// At `target_depth`, for each Group sitting at depth `target_depth - 1`, transform
/// its children to match truth's relative layout *within the parent's drawn bbox*.
fn apply_level_correction(
    node: &AnalyzedKanjiNode,
    assignment: &[usize],
    working: &mut [Vec<(f32, f32)>],
    target_depth: usize,
    current_depth: usize,
) {
    // We act on Groups whose children are at target_depth.
    if current_depth + 1 == target_depth {
        if let AnalyzedKanjiNode::Group { children, .. } = node {
            transform_children_relative(node, children, assignment, working);
        }
    } else if current_depth + 1 < target_depth {
        if let AnalyzedKanjiNode::Group { children, .. } = node {
            let mut counter = 0;
            for child in children {
                let size = leaf_count(child);
                let slice = &assignment[counter..counter + size];
                apply_level_correction(child, slice, working, target_depth, current_depth + 1);
                counter += size;
            }
        }
    }
    // current_depth + 1 > target_depth: nothing to do, we've passed it
}

/// Transform each child of `parent` according to the truth's layout within the
/// parent's drawn bbox.
fn transform_children_relative(
    parent: &AnalyzedKanjiNode,
    children: &[AnalyzedKanjiNode],
    parent_assignment: &[usize],
    working: &mut [Vec<(f32, f32)>],
) {
    // Compute parent bboxes — frozen before any child transformation.
    let parent_t_strokes = collect_ref_strokes_owned(parent);
    if parent_t_strokes.is_empty() {
        return;
    }
    let t_parent = parent_t_strokes.gen_bbox();

    let parent_d_strokes: Vec<Vec<(f32, f32)>> = parent_assignment
        .iter()
        .filter_map(|&i| {
            if i == usize::MAX {
                None
            } else {
                working.get(i).cloned()
            }
        })
        .collect();
    if parent_d_strokes.is_empty() {
        return;
    }
    let d_parent = parent_d_strokes.gen_bbox();

    let t_pw = t_parent.width();
    let t_ph = t_parent.height();
    if t_pw < 1e-6 || t_ph < 1e-6 {
        return;
    }

    // For each child: figure out target position+size within the parent's drawn bbox.
    let mut counter = 0;
    for child in children {
        let size = leaf_count(child);
        let child_assignment = &parent_assignment[counter..counter + size];
        counter += size;

        let child_t_strokes = collect_ref_strokes_owned(child);
        if child_t_strokes.is_empty() {
            continue;
        }
        let t_child = child_t_strokes.gen_bbox();

        let child_d_strokes: Vec<Vec<(f32, f32)>> = child_assignment
            .iter()
            .filter_map(|&i| {
                if i == usize::MAX {
                    None
                } else {
                    working.get(i).cloned()
                }
            })
            .collect();
        if child_d_strokes.is_empty() {
            continue;
        }
        let d_child_current = child_d_strokes.gen_bbox();

        // Where the child *should* be in the drawn parent's bbox (relative to truth's layout).
        let target = relative_target(&t_parent, &t_child, &d_parent);

        // Transform child's user strokes from d_child_current → target.
        transform_strokes(child_assignment, working, &d_child_current, &target);
    }
}

/// Maps the child's bbox in truth (relative to truth's parent bbox) into drawn space
/// (relative to drawn's parent bbox).
fn relative_target(t_parent: &BBox, t_child: &BBox, d_parent: &BBox) -> BBox {
    let t_pw = t_parent.width().max(1e-6);
    let t_ph = t_parent.height().max(1e-6);
    let rel_min_x = (t_child.min.x - t_parent.min.x) / t_pw;
    let rel_min_y = (t_child.min.y - t_parent.min.y) / t_ph;
    let rel_max_x = (t_child.max.x - t_parent.min.x) / t_pw;
    let rel_max_y = (t_child.max.y - t_parent.min.y) / t_ph;

    let d_pw = d_parent.width();
    let d_ph = d_parent.height();
    BBox {
        min: lyon_path::math::point(
            d_parent.min.x + rel_min_x * d_pw,
            d_parent.min.y + rel_min_y * d_ph,
        ),
        max: lyon_path::math::point(
            d_parent.min.x + rel_max_x * d_pw,
            d_parent.min.y + rel_max_y * d_ph,
        ),
    }
}

/// Transform the user strokes (indexed by `leaf_indices`) so their bbox goes
/// from `current` to `target`. Per-axis: translate + scale. Identity on degenerate axes.
fn transform_strokes(
    leaf_indices: &[usize],
    working: &mut [Vec<(f32, f32)>],
    current: &BBox,
    target: &BBox,
) {
    let cx = (current.min.x + current.max.x) * 0.5;
    let cy = (current.min.y + current.max.y) * 0.5;
    let tx = (target.min.x + target.max.x) * 0.5;
    let ty = (target.min.y + target.max.y) * 0.5;

    let cw = current.width();
    let ch = current.height();
    let tw = target.width();
    let th = target.height();

    let sx = if cw > 1e-6 { tw / cw } else { 1.0 };
    let sy = if ch > 1e-6 { th / ch } else { 1.0 };

    for &i in leaf_indices {
        if i == usize::MAX {
            continue;
        }
        if let Some(stroke) = working.get_mut(i) {
            for p in stroke.iter_mut() {
                p.0 = (p.0 - cx) * sx + tx;
                p.1 = (p.1 - cy) * sy + ty;
            }
        }
    }
}

fn collect_ref_strokes_owned(node: &AnalyzedKanjiNode) -> Vec<Vec<OrientedPoint>> {
    let mut out = Vec::new();
    walk_ref_leaves(node, &mut out);
    out
}
fn collect_ref_in_stroke_frame(node: &AnalyzedKanjiNode) -> Vec<Vec<OrientedPoint>> {
    let mut out = Vec::new();
    walk_in_stroke_frame(node, &mut out);
    out
}

fn walk_in_stroke_frame(node: &AnalyzedKanjiNode, out: &mut Vec<Vec<OrientedPoint>>) {
    match node {
        AnalyzedKanjiNode::Stroke {
            in_stroke_frame, ..
        } => out.push(in_stroke_frame.clone()),
        AnalyzedKanjiNode::Group { children, .. } => {
            for c in children {
                walk_in_stroke_frame(c, out);
            }
        }
    }
}

fn aggregate_per_user_point(path: &[(usize, usize, f32)], user_len: usize) -> Vec<f32> {
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
        .map(|(&s, &c)| if c > 0 { s / c as f32 } else { 0.0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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
