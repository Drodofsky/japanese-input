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
    #[must_use]
    #[inline]
    pub fn collect_geometry(&self) -> Vec<StrokeGeometry> {
        let mut geometries = Vec::new();
        collect_into_geometry(self, &mut geometries);
        geometries
    }
    #[must_use]
    #[inline]
    pub fn leaf_count(&self) -> usize {
        match self {
            AnalyzedKanjiNode::Stroke { .. } => 1,
            AnalyzedKanjiNode::Group { children, .. } => {
                children.iter().map(Self::leaf_count).sum()
            }
        }
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
fn collect_into_geometry(node: &AnalyzedKanjiNode, out: &mut Vec<StrokeGeometry>) {
    match node {
        AnalyzedKanjiNode::Stroke { geometry, .. } => {
            out.push(*geometry);
        }
        AnalyzedKanjiNode::Group { children, .. } => {
            for child in children {
                collect_into_geometry(child, out);
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
