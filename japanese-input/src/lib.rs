pub mod analyzed_kanji_node;
pub mod bbox;
pub mod convert_lossy;
pub mod convert_stroke_index;
pub mod dtw;
pub mod kanji_node;
pub mod leaf_matrix;
pub mod normalize;
pub mod recognize_hiragana;
pub mod recognize_kanji;
pub mod stroke_point;
use std::collections::HashMap;

use crate::kanji_node::KanjiNode;

pub type KanjiMap = HashMap<char, KanjiNode>;
