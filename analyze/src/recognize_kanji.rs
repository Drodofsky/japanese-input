use crate::KanjiMap;
use crate::analyze::AnalyzedKanjiNode;
use crate::hungarian_matcher::match_hungarian;
use crate::leaf_matrix::LeafMatrix;
use crate::match_node::prepare_user;

#[derive(Debug, Clone)]
pub struct RecognitionResult {
    pub character: char,
    pub score: f32,
}

pub struct KanjiRecognizer {
    candidates: Vec<(char, AnalyzedKanjiNode)>,
}

impl KanjiRecognizer {
    #[must_use]
    pub fn new(kanji_map: &KanjiMap) -> Self {
        Self {
            candidates: AnalyzedKanjiNode::preprocess_map(kanji_map),
        }
    }

    /// Recognizes a kanji character from user strokes. Returns ranked candidates,
    /// best first. Only candidates whose stroke count matches the user's are considered.
    #[must_use]
    pub fn recognize(&self, user_strokes: &[Vec<(f32, f32)>]) -> Vec<RecognitionResult> {
        if user_strokes.is_empty() {
            return Vec::new();
        }

        let (user_b, user_c) = prepare_user(user_strokes);

        let mut results: Vec<RecognitionResult> = self
            .candidates
            .iter()
            .filter(|(_, node)| usize::from(node.len()) == user_strokes.len())
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
        results
    }
}
