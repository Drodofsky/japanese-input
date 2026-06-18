use crate::KanjiMap;
use crate::bbox::BBox as _;
use crate::dtw::{Weights, dtw};
use crate::leaf_matrix::LeafMatrix;
use crate::normalize::Normalize as _;
use crate::stroke_point::StrokePoint;
const WEIGHTS: Weights = Weights {
    position: 1.0,
    tangent: 1.0,
};
const MISSING_PENALTY: f64 = 1.0;
const SMALL_THRESHOLD: f64 = 0.5;
fn small_variant(c: char) -> Option<char> {
    match c {
        'あ' => Some('ぁ'),
        'い' => Some('ぃ'),
        'う' => Some('ぅ'),
        'え' => Some('ぇ'),
        'お' => Some('ぉ'),
        'つ' => Some('っ'),
        'や' => Some('ゃ'),
        'ゆ' => Some('ゅ'),
        'よ' => Some('ょ'),
        'わ' => Some('ゎ'),
        _ => None,
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RecognitionResult {
    pub character: char,
    pub score: f64,
}

pub struct HiraganaRecognizer {
    candidates: Vec<(char, Vec<Vec<StrokePoint>>)>,
}

impl HiraganaRecognizer {
    #[must_use]
    #[inline]
    pub fn new(kanji_map: &KanjiMap) -> Self {
        let candidates = kanji_map
            .iter()
            .map(|(&c, node)| {
                let strokes = node.collect_strokes();
                let normalized = strokes.normalized();
                (c, normalized)
            })
            .collect();
        Self { candidates }
    }

    #[must_use]
    #[inline]
    pub fn recognize(&self, user_strokes: Vec<Vec<StrokePoint>>) -> Vec<RecognitionResult> {
        if user_strokes.is_empty() {
            return Vec::new();
        }
        let is_small = is_drawing_small(&user_strokes);
        let user_normalized = user_strokes.normalized();

        let mut results: Vec<RecognitionResult> = self
            .candidates
            .iter()
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
        if is_small {
            for r in &mut results {
                if let Some(small) = small_variant(r.character) {
                    r.character = small;
                }
            }
        }
        results
    }
}

#[must_use]
fn is_drawing_small(user_strokes: &[Vec<StrokePoint>]) -> bool {
    match user_strokes.bbox() {
        Some(rect) => rect.width().max(rect.height()) <= SMALL_THRESHOLD,
        None => false,
    }
}
