pub mod bbox;
pub mod convert_lossy;
pub mod dtw;
pub mod leaf_matrix;
pub mod normalize;
pub mod recognize_hiragana;
pub mod recognize_kanji;
pub mod stroke_point;
use std::collections::HashMap;

use kurbo::BezPath;
use serde::{Deserialize, Serialize};

use crate::stroke_point::{StrokePoint, ToStrokePoint as _};
pub type KanjiMap = HashMap<char, KanjiNode>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub enum KanjiNode {
    Group {
        element: Option<char>,
        children: Vec<KanjiNode>,
    },
    Stroke {
        index: u8,
        path: BezPath,
    },
}
#[must_use]
#[inline]
pub fn collect_strokes(root: &KanjiNode) -> Vec<Vec<StrokePoint>> {
    let mut strokes = Vec::new();
    collect_into(root, &mut strokes);
    strokes
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
