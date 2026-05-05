use crate::KanjiMap;
use crate::analyzed_kanji_node::AnalyzedKanjiNode;
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
    pub fn new(kanji_map: &KanjiMap) -> Self {
        let mut candidates = Vec::new();
        for &c in kanji_map.keys() {
            if let Some(node) = kanji_map.get(&c) {
                candidates.push((c, AnalyzedKanjiNode::from_node(node)));
            }
        }
        Self { candidates }
    }

    /// Recognizes a hiragana character from user strokes. Returns ranked candidates,
    /// best first. If the user's drawing bbox max side is ≤ 0.5 (in normalized canvas
    /// space), the result character is mapped to its small variant when one exists.
    pub fn recognize(&self, user_strokes: &[Vec<(f32, f32)>]) -> Vec<RecognitionResult> {
        if user_strokes.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<RecognitionResult> = self
            .candidates
            .iter()
            .filter(|p| p.1.len() == user_strokes.len())
            .filter_map(|(c, node)| {
                let (user_b, user_c) = prepare_user(user_strokes);

                let leaf_matrix = LeafMatrix::create(node, &user_b, &user_c);

                let matches = match_hungarian(&leaf_matrix);
                let score = matches.first()?.score;
                Some(RecognitionResult {
                    character: *c,
                    score,
                })
            })
            .collect();

        results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());

        results
    }
}
