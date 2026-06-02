use crate::KanjiMap;
use crate::dtw::{Weights, dtw};
use crate::leaf_matrix::LeafMatrix;
use crate::normalize::Normalize as _;
use crate::stroke_point::StrokePoint;
const WEIGHTS: Weights = Weights {
    position: 1.0,
    curvature: 0.0,
    tangent: 0.0,
};
const MISSING_PENALTY: f64 = 1.0;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RecognitionResult {
    pub character: char,
    pub score: f64,
}

pub struct KanjiRecognizer {
    candidates: Vec<(char, Vec<Vec<StrokePoint>>)>,
}

impl KanjiRecognizer {
    #[must_use]
    #[inline]
    pub fn new(kanji_map: &KanjiMap) -> Self {
        let candidates = kanji_map
            .iter()
            .map(|(&c, node)| (c, node.collect_strokes().normalized()))
            .collect();
        Self { candidates }
    }

    #[must_use]
    #[inline]
    pub fn recognize(&self, user_strokes: Vec<Vec<StrokePoint>>) -> Vec<RecognitionResult> {
        if user_strokes.is_empty() {
            return Vec::new();
        }
        let user_normalized = user_strokes.normalized();
        let user_count = user_strokes.len();
        let mut results: Vec<RecognitionResult> = self
            .candidates
            .iter()
            .filter(|(_, data)| user_count == data.len())
            .filter_map(|(c, ref_strokes)| {
                let matrix =
                    LeafMatrix::build(&user_normalized, ref_strokes, MISSING_PENALTY, |a, b| {
                        dtw(a, b, &WEIGHTS)
                    });
                let score = matrix.score().ok()?;
                Some(RecognitionResult {
                    character: *c,
                    score,
                })
            })
            .collect();
        results.sort_by(|a, b| a.score.total_cmp(&b.score));
        results
    }
}
