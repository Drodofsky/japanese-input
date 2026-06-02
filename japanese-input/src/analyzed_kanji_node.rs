use crate::{
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
    },
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
            KanjiNode::Stroke { index, path } => AnalyzedKanjiNode::Stroke {
                index,
                path: path.to_stroke_points(),
            },
        }
    }
}
