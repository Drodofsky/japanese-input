use crate::{
    kanji_node::KanjiNode,
    stroke_geometry::StrokeGeometry,
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
        geometry: StrokeGeometry,
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
                let geometry = StrokeGeometry::from_stroke(&path);
                AnalyzedKanjiNode::Stroke {
                    index,
                    path,
                    geometry,
                }
            }
        }
    }
}
