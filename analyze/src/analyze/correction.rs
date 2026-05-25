use super::AnalyzedKanjiNode;
use crate::bbox::{BBox, GenBBox};

use super::tree::{collect_kanji_frame_strokes, leaf_count};

/// At `target_depth`, for each Group sitting at depth `target_depth - 1`, transform
/// its children to match truth's relative layout *within the parent's drawn bbox*.
pub(super) fn apply_level_correction(
    node: &AnalyzedKanjiNode,
    assignment: &[u8],
    working: &mut [Vec<(f32, f32)>],
    target_depth: usize,
    current_depth: usize,
) {
    // We act on Groups whose children are at target_depth.
    if current_depth + 1 == target_depth {
        if let AnalyzedKanjiNode::Group { children, .. } = node {
            transform_children_relative(node, children, assignment, working);
        }
    } else if current_depth + 1 < target_depth
        && let AnalyzedKanjiNode::Group { children, .. } = node
    {
        let mut counter = 0;
        for child in children {
            let size = leaf_count(child);
            let slice = &assignment[counter..counter + size];
            apply_level_correction(child, slice, working, target_depth, current_depth + 1);
            counter += size;
        }
    }
    // current_depth + 1 > target_depth: nothing to do, we've passed it
}

/// Transform each child of `parent` according to the truth's layout within the
/// parent's drawn bbox.
fn transform_children_relative(
    parent: &AnalyzedKanjiNode,
    children: &[AnalyzedKanjiNode],
    parent_assignment: &[u8],
    working: &mut [Vec<(f32, f32)>],
) {
    // Compute parent bboxes — frozen before any child transformation.
    let parent_t_strokes = collect_kanji_frame_strokes(parent);
    if parent_t_strokes.is_empty() {
        return;
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
    leaf_indices: &[u8],
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
