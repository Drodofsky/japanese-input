use super::AnalyzedKanjiNode;
use crate::point::OrientedPoint;

/// Collects all leaf strokes represented in the kanji (whole-character) coordinate frame.
pub(super) fn collect_kanji_frame_strokes(node: &AnalyzedKanjiNode) -> Vec<Vec<OrientedPoint>> {
    let mut out = Vec::new();
    walk_kanji_frame_strokes(node, &mut out);
    out
}

fn walk_kanji_frame_strokes(node: &AnalyzedKanjiNode, out: &mut Vec<Vec<OrientedPoint>>) {
    match node {
        AnalyzedKanjiNode::Stroke { in_kanji_frame, .. } => {
            out.push(in_kanji_frame.clone());
        }
        AnalyzedKanjiNode::Group { children, .. } => {
            for c in children {
                walk_kanji_frame_strokes(c, out);
            }
        }
    }
}

/// Collects all leaf strokes represented in the individual stroke coordinate frame.
pub(super) fn collect_stroke_frame_strokes(node: &AnalyzedKanjiNode) -> Vec<Vec<OrientedPoint>> {
    let mut out = Vec::new();
    walk_stroke_frame_strokes(node, &mut out);
    out
}

fn walk_stroke_frame_strokes(node: &AnalyzedKanjiNode, out: &mut Vec<Vec<OrientedPoint>>) {
    match node {
        AnalyzedKanjiNode::Stroke {
            in_stroke_frame, ..
        } => out.push(in_stroke_frame.clone()),
        AnalyzedKanjiNode::Group { children, .. } => {
            for c in children {
                walk_stroke_frame_strokes(c, out);
            }
        }
    }
}

pub(super) fn tree_depth(node: &AnalyzedKanjiNode) -> usize {
    match node {
        AnalyzedKanjiNode::Stroke { .. } => 0,
        AnalyzedKanjiNode::Group { children, .. } => {
            1 + children.iter().map(tree_depth).max().unwrap_or(0)
        }
    }
}

pub(super) fn leaf_count(node: &AnalyzedKanjiNode) -> usize {
    match node {
        AnalyzedKanjiNode::Stroke { .. } => 1,
        AnalyzedKanjiNode::Group { children, .. } => children.iter().map(leaf_count).sum(),
    }
}
