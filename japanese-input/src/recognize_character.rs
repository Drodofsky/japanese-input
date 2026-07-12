use crate::KanjiMap;
use crate::bbox::BBox as _;
use crate::dtw::{DTWWeights, dtw};
use crate::leaf_matrix::LeafMatrix;
use crate::normalize::Normalize as _;
use crate::stroke_point::StrokePoint;
const WEIGHTS: DTWWeights = DTWWeights {
    position: 1.0,
    tangent: 1.0,
};
const MISSING_PENALTY: f64 = 1.0;
const SMALL_THRESHOLD: f64 = 0.5;
const SMALL_CHARS: &str = "ぁぃぅぇぉっゃゅょゎゕゖァィゥェォッャュョヮヵヶ";
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RecognitionResult {
    pub character: char,
    pub score: f64,
}

pub struct Recognizer {
    candidates: Vec<(char, Vec<Vec<StrokePoint>>)>,
}

impl Recognizer {
    #[must_use]
    #[inline]
    pub fn new(kanji_map: &KanjiMap) -> Self {
        let candidates = kanji_map
            .iter()
            .filter(|c| !SMALL_CHARS.contains(*c.0))
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
        let is_small = is_drawing_small(&user_strokes);
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
        if is_small {
            for r in &mut results {
                if let Some(small) = to_small(r.character) {
                    r.character = small;
                }
            }
        }
        results
    }
}

#[must_use]
fn to_small(c: char) -> Option<char> {
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
        'か' => Some('ゕ'),
        'け' => Some('ゖ'),
        'ア' => Some('ァ'),
        'イ' => Some('ィ'),
        'ウ' => Some('ゥ'),
        'エ' => Some('ェ'),
        'オ' => Some('ォ'),
        'ツ' => Some('ッ'),
        'ヤ' => Some('ャ'),
        'ユ' => Some('ュ'),
        'ヨ' => Some('ョ'),
        'ワ' => Some('ヮ'),
        'カ' => Some('ヵ'),
        'ケ' => Some('ヶ'),
        _ => None,
    }
}

#[must_use]
fn is_drawing_small(user_strokes: &[Vec<StrokePoint>]) -> bool {
    match user_strokes.bbox() {
        Some(rect) => rect.width().max(rect.height()) <= SMALL_THRESHOLD,
        None => false,
    }
}
