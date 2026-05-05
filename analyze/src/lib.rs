pub mod analyze;
pub mod analyzed_kanji_node;
pub mod bbox;
pub mod dtw;
pub mod hungarian_matcher;
pub mod leaf_matrix;
pub mod match_node;
pub mod normalize;
pub mod point;
pub mod recognize_hiragana;
pub mod recognize_kanji;
use serde::{Deserialize, Serialize};

pub type KanjiMap = std::collections::HashMap<char, KanjiNode>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum KanjiNode {
    Group {
        element: Option<char>,
        children: Vec<KanjiNode>,
    },
    Stroke {
        index: usize,
        path: lyon_path::Path,
    },
}
