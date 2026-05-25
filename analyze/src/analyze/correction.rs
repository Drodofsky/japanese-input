use super::AnalyzedKanjiNode;
use crate::bbox::{BBox, GenBBox};

use super::tree::{collect_kanji_frame_strokes, leaf_count};

/// At `target_depth`, for each Group sitting at depth `target_depth - 1`, transform
/// its children to match truth's relative layout *within the parent's drawn bbox*.
/// Returns the maximum correction score (0.0 = no correction, 1.0 ≈ full-parent-size error)
/// observed at or below this node.
pub(super) fn apply_level_correction(
    node: &AnalyzedKanjiNode,
    assignment: &[u8],
    working: &mut [Vec<(f32, f32)>],
    target_depth: usize,
    current_depth: usize,
) -> f32 {
    // We act on Groups whose children are at target_depth.
    if current_depth + 1 == target_depth {
        if let AnalyzedKanjiNode::Group { children, .. } = node {
            transform_children_relative(node, children, assignment, working)
        } else {
            0.0
        }
    } else if current_depth + 1 < target_depth
        && let AnalyzedKanjiNode::Group { children, .. } = node
    {
        let mut counter = 0;
        let mut max_score = 0.0_f32;
        for child in children {
            let size = leaf_count(child);
            let slice = &assignment[counter..counter + size];
            let score =
                apply_level_correction(child, slice, working, target_depth, current_depth + 1);
            max_score = max_score.max(score);
            counter += size;
        }
        max_score
    } else {
        // current_depth + 1 > target_depth, or node is a Stroke: nothing to do
        0.0
    }
}

/// Transform each child of `parent` according to the truth's layout within the
/// parent's drawn bbox. Returns the max correction score over all children.
fn transform_children_relative(
    parent: &AnalyzedKanjiNode,
    children: &[AnalyzedKanjiNode],
    parent_assignment: &[u8],
    working: &mut [Vec<(f32, f32)>],
) -> f32 {
    // Compute parent bboxes — frozen before any child transformation.
    let parent_t_strokes = collect_kanji_frame_strokes(parent);
    if parent_t_strokes.is_empty() {
        return 0.0;
    }
    let t_parent = parent_t_strokes.gen_bbox();

    let parent_d_strokes: Vec<Vec<(f32, f32)>> = parent_assignment
        .iter()
        .filter_map(|&i| {
            if i == u8::MAX {
                None
            } else {
                working.get(i as usize).cloned()
            }
        })
        .collect();
    if parent_d_strokes.is_empty() {
        return 0.0;
    }
    let d_parent = parent_d_strokes.gen_bbox();

    let t_pw = t_parent.width();
    let t_ph = t_parent.height();
    if t_pw < 1e-6 || t_ph < 1e-6 {
        return 0.0;
    }

    // Size of the drawn parent bbox, used to normalize translation error.
    let d_parent_size = d_parent.width().max(d_parent.height());

    let mut max_score = 0.0_f32;
    let mut counter = 0;

    for child in children {
        let size = leaf_count(child);
        let child_assignment = &parent_assignment[counter..counter + size];
        counter += size;

        let child_t_strokes = collect_kanji_frame_strokes(child);
        if child_t_strokes.is_empty() {
            continue;
        }
        let t_child = child_t_strokes.gen_bbox();

        let child_d_strokes: Vec<Vec<(f32, f32)>> = child_assignment
            .iter()
            .filter_map(|&i| {
                if i == u8::MAX {
                    None
                } else {
                    working.get(i as usize).cloned()
                }
            })
            .collect();
        if child_d_strokes.is_empty() {
            continue;
        }
        let d_child_current = child_d_strokes.gen_bbox();

        // Where the child *should* be in the drawn parent's bbox (relative to truth's layout).
        let target = relative_target(&t_parent, &t_child, &d_parent);

        // Measure how large the correction is before applying it.
        let score = correction_score(&d_child_current, &target, d_parent_size);
        max_score = max_score.max(score);

        // Transform child's user strokes from d_child_current → target.
        transform_strokes(child_assignment, working, &d_child_current, &target);
    }

    max_score
}

/// Score how large the correction from `current` to `target` is, relative to `parent_size`.
///
/// Returns the max of:
/// - translation error: Euclidean center displacement / `parent_size`
/// - scale error: max(|sx − 1|, |sy − 1|) where sx/sy are the axis scale factors
///
/// A score of 0.0 means no correction was needed; ~1.0 means the center was
/// displaced by roughly one full parent dimension.
fn correction_score(current: &BBox, target: &BBox, parent_size: f32) -> f32 {
    if parent_size < 1e-6 {
        return 0.0;
    }
    let (cx, cy) = current.center();
    let (tx, ty) = target.center();

    let translate_score = (tx - cx).hypot(ty - cy) / parent_size;
    let scale_score = (current.width() - target.width())
        .abs()
        .max((current.height() - target.height()).abs())
        / parent_size;

    translate_score.max(scale_score)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bbox::BBox;
    use lyon_path::math::point;

    fn bbox(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> BBox {
        BBox {
            min: point(min_x, min_y),
            max: point(max_x, max_y),
        }
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn no_correction_scores_zero() {
        let b = bbox(0.0, 0.0, 2.0, 2.0);
        assert!(approx(correction_score(&b, &b, 10.0), 0.0));
    }

    #[test]
    fn pure_translation_full_parent_width() {
        // Center moves exactly parent_size units → score = 1.0.
        let current = bbox(0.0, 0.0, 1.0, 1.0); // center (0.5, 0.5)
        let target = bbox(10.0, 0.0, 11.0, 1.0); // center (10.5, 0.5), moved 10 right
        assert!(approx(correction_score(&current, &target, 10.0), 1.0));
    }

    #[test]
    fn partial_translation_scores_half() {
        let current = bbox(0.0, 0.0, 1.0, 1.0);
        let target = bbox(5.0, 0.0, 6.0, 1.0); // moved 5 right, parent_size = 10
        assert!(approx(correction_score(&current, &target, 10.0), 0.5));
    }

    #[test]
    fn diagonal_translation_uses_euclidean_not_manhattan() {
        // Moved (3, 4) → hypot = 5, not 3+4 = 7. parent_size = 10 → score = 0.5.
        let current = bbox(0.0, 0.0, 1.0, 1.0); // center (0.5, 0.5)
        let target = bbox(3.0, 4.0, 4.0, 5.0); // center (3.5, 4.5), delta = (3, 4)
        assert!(approx(correction_score(&current, &target, 10.0), 0.5));
    }

    #[test]
    fn scale_doubled_scores_point_two() {
        // Same center, target twice as wide/tall: |Δw| = 2, |Δh| = 2, parent_size = 10 → 0.2.
        let current = bbox(0.0, 0.0, 2.0, 2.0); // size 2×2, center (1,1)
        let target = bbox(-1.0, -1.0, 3.0, 3.0); // size 4×4, center (1,1)
        assert!(approx(correction_score(&current, &target, 10.0), 0.2));
    }

    #[test]
    fn scale_difference_half_parent() {
        // |Δw| = 5 on parent_size = 10 → scale_score = 0.5; no translation.
        let current = bbox(2.5, 2.5, 7.5, 5.0); // width 5, center (5, 3.75)
        let target = bbox(0.0, 2.5, 10.0, 5.0); // width 10, center (5, 3.75)
        assert!(approx(correction_score(&current, &target, 10.0), 0.5));
    }

    #[test]
    fn translation_dominates_scale() {
        // translate = 8/10 = 0.8, scale = 0.0 → score = 0.8.
        let current = bbox(0.0, 0.0, 4.0, 4.0); // center (2, 2), size 4×4
        let target = bbox(8.0, 0.0, 12.0, 4.0); // center (10, 2), same size
        assert!(approx(correction_score(&current, &target, 10.0), 0.8));
    }

    #[test]
    fn scale_dominates_translation() {
        // translate = hypot(2,0)/10 = 0.2, scale = |5−2|/10 = 0.3 → score = 0.3.
        let current = bbox(4.5, 4.5, 6.5, 5.5); // center (5.5, 5), width 2
        let target = bbox(5.0, 4.5, 10.0, 5.5); // center (7.5, 5), width 5
        assert!(approx(correction_score(&current, &target, 10.0), 0.3));
    }

    #[test]
    fn zero_parent_size_scores_zero() {
        // When parent_size = 0 both terms are suppressed → score = 0.
        let current = bbox(0.0, 0.0, 1.0, 1.0);
        let target = bbox(100.0, 0.0, 110.0, 5.0); // huge shift and scale change
        assert!(approx(correction_score(&current, &target, 0.0), 0.0));
    }
}

/// Transform the user strokes (indexed by `leaf_indices`) so their bbox goes
/// from `current` to `target`. Per-axis: translate + scale. Identity on degenerate axes.
fn transform_strokes(
    leaf_indices: &[u8],
    working: &mut [Vec<(f32, f32)>],
    current: &BBox,
    target: &BBox,
) {
    let (cx, cy) = current.center();
    let (tx, ty) = target.center();

    let sx = if current.width() > 1e-6 {
        target.width() / current.width()
    } else {
        1.0
    };
    let sy = if current.height() > 1e-6 {
        target.height() / current.height()
    } else {
        1.0
    };

    for &i in leaf_indices {
        if i == u8::MAX {
            continue;
        }
        if let Some(stroke) = working.get_mut(i as usize) {
            for p in stroke.iter_mut() {
                p.0 = (p.0 - cx) * sx + tx;
                p.1 = (p.1 - cy) * sy + ty;
            }
        }
    }
}
