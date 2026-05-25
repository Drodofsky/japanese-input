use crate::KanjiMap;
use crate::analyze::AnalyzedKanjiNode;
use crate::bbox::GenBBox;
use crate::hungarian_matcher::match_hungarian;
use crate::leaf_matrix::LeafMatrix;
use crate::match_node::prepare_user;

/// Maps a base character to its small variant, when one exists.
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
pub struct RecognitionResult {
    pub character: char,
    pub score: f32,
}

pub struct HiraganaRecognizer {
    candidates: Vec<(char, AnalyzedKanjiNode)>,
}

impl HiraganaRecognizer {
    #[must_use]
    pub fn new(kanji_map: &KanjiMap) -> Self {
        Self {
            candidates: AnalyzedKanjiNode::preprocess_map(kanji_map),
        }
    }

    /// Recognizes a hiragana character from user strokes. Returns ranked candidates,
    /// best first. If the user's drawing bbox max side is ≤ 0.5 (in normalized canvas
    /// space), the result character is mapped to its small variant when one exists.
    #[must_use]
    pub fn recognize(&self, user_strokes: &[Vec<(f32, f32)>]) -> Vec<RecognitionResult> {
        if user_strokes.is_empty() {
            return Vec::new();
        }

        let is_small = is_drawing_small(user_strokes);
        let (user_b, user_c) = prepare_user(user_strokes);

        let mut results: Vec<RecognitionResult> = self
            .candidates
            .iter()
            .filter_map(|(c, node)| {
                let leaf_matrix = LeafMatrix::create(node, &user_b, &user_c);
                let score = match_hungarian(&leaf_matrix).first()?.score;
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

/// Returns `true` if the bounding box of the drawing fits within the "small kana"
/// threshold (max side ≤ 0.5 in normalized canvas space).
fn is_drawing_small(strokes: &[Vec<(f32, f32)>]) -> bool {
    let bbox = strokes.gen_bbox();
    bbox.width().max(bbox.height()) <= 0.5
}
