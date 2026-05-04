use crate::KanjiMap;
use crate::analyzed_kanji_node::AnalyzedKanjiNode;
use crate::hungarian_matcher::match_hungarian;
use crate::leaf_matrix::LeafMatrix;
use crate::match_node::prepare_user;

const HIRAGANA_BASE: &[char] = &[
    'あ', 'い', 'う', 'え', 'お', 'か', 'き', 'く', 'け', 'こ', 'さ', 'し', 'す', 'せ', 'そ', 'た',
    'ち', 'つ', 'て', 'と', 'な', 'に', 'ぬ', 'ね', 'の', 'は', 'ひ', 'ふ', 'へ', 'ほ', 'ま', 'み',
    'む', 'め', 'も', 'や', 'ゆ', 'よ', 'ら', 'り', 'る', 'れ', 'ろ', 'わ', 'を', 'ん',
];

const HIRAGANA_DAKUTEN: &[char] = &[
    'が', 'ぎ', 'ぐ', 'げ', 'ご', 'ざ', 'じ', 'ず', 'ぜ', 'ぞ', 'だ', 'ぢ', 'づ', 'で', 'ど', 'ば',
    'び', 'ぶ', 'べ', 'ぼ',
];

const HIRAGANA_HANDAKUTEN: &[char] = &['ぱ', 'ぴ', 'ぷ', 'ぺ', 'ぽ'];

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
    pub fn new(kanji_map: &KanjiMap) -> Self {
        let mut candidates = Vec::new();
        for &c in HIRAGANA_BASE
            .iter()
            .chain(HIRAGANA_DAKUTEN.iter())
            .chain(HIRAGANA_HANDAKUTEN.iter())
        {
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

        let is_small = is_drawing_small(user_strokes);

        let mut results: Vec<RecognitionResult> = self
            .candidates
            .iter()
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

        if is_small {
            for r in results.iter_mut() {
                if let Some(small) = small_variant(r.character) {
                    r.character = small;
                }
            }
        }

        results
    }
}

fn is_drawing_small(strokes: &[Vec<(f32, f32)>]) -> bool {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for s in strokes {
        for &(x, y) in s {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    if !min_x.is_finite() {
        return false;
    }
    let w = max_x - min_x;
    let h = max_y - min_y;
    w.max(h) <= 0.5
}
