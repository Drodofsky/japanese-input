use kurbo::{Point, Rect};

use crate::{
    bbox::BBox as _,
    centroid::Centroid as _,
    kanji_node::KanjiNode,
    stroke_point::{StrokePoint, ToStrokePoint as _},
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AnalyzedKanjiNode {
    Group {
        element: Option<char>,
        children: Vec<AnalyzedKanjiNode>,
    },
    Stroke {
        index: u8,
        path: Vec<StrokePoint>,
        bbox: Option<Rect>,
        centroid: Option<Point>,
    },
}

impl AnalyzedKanjiNode {
    #[must_use]
    #[inline]
    pub fn collect_strokes(&self) -> Vec<Vec<StrokePoint>> {
        let mut strokes = Vec::new();
        collect_into(self, &mut strokes);
        strokes
    }
}

fn collect_into(node: &AnalyzedKanjiNode, out: &mut Vec<Vec<StrokePoint>>) {
    match node {
        AnalyzedKanjiNode::Stroke { path, .. } => {
            out.push(path.clone());
        }
        AnalyzedKanjiNode::Group { children, .. } => {
            for child in children {
                collect_into(child, out);
            }
        }
    }
}

impl KanjiNode {
    #[must_use]
    #[inline]
    pub fn to_analyzed(self) -> AnalyzedKanjiNode {
        match self {
            KanjiNode::Group { element, children } => AnalyzedKanjiNode::Group {
                element,
                children: children.into_iter().map(KanjiNode::to_analyzed).collect(),
            },
            KanjiNode::Stroke { index, path } => {
                let path = path.to_stroke_points();
                let bbox = path.bbox();
                let centroid = path.centroid();
                AnalyzedKanjiNode::Stroke {
                    index,
                    path,
                    bbox,
                    centroid,
                }
            }
        }
    }
}
