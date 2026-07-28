use kurbo::BezPath;
use serde::{Deserialize, Serialize};

use crate::stroke_point::{StrokePoint, ToStrokePoint as _};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub enum KanjiNode {
    Group {
        element: char,
        children: Vec<KanjiNode>,
    },
    Stroke {
        index: u8,
        path: BezPath,
    },
}

impl KanjiNode {
    #[must_use]
    #[inline]
    pub fn collect_strokes(&self) -> Vec<Vec<StrokePoint>> {
        let mut strokes = Vec::new();
        collect_into(self, &mut strokes);
        strokes
    }
    #[must_use]
    #[inline]
    pub fn collect_paths(&self) -> Vec<BezPath> {
        let mut strokes = Vec::new();
        collect_paths(self, &mut strokes);
        strokes
    }
    #[must_use]
    #[inline]
    pub fn leaf_count(&self) -> usize {
        match self {
            KanjiNode::Stroke { .. } => 1,
            KanjiNode::Group { children, .. } => children.iter().map(Self::leaf_count).sum(),
        }
    }
}

fn collect_into(node: &KanjiNode, out: &mut Vec<Vec<StrokePoint>>) {
    match node {
        KanjiNode::Stroke { path, .. } => {
            out.push(path.to_stroke_points());
        }
        KanjiNode::Group { children, .. } => {
            for child in children {
                collect_into(child, out);
            }
        }
    }
}
fn collect_paths(node: &KanjiNode, out: &mut Vec<BezPath>) {
    match node {
        KanjiNode::Stroke { path, .. } => {
            out.push(path.clone());
        }
        KanjiNode::Group { children, .. } => {
            for child in children {
                collect_paths(child, out);
            }
        }
    }
}
