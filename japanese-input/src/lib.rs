pub mod analyzed_kanji_node;
pub mod bbox;
pub mod centroid;
pub mod convert_lossy;
pub mod convert_stroke_index;
pub mod dtw;
pub mod kanji_node;
pub mod leaf_matrix;
pub mod match_strokes;
pub mod normalize;
pub mod recognize_character;
pub mod recognize_hiragana;
pub mod recognize_kanji;
pub mod stroke_geometry;
pub mod stroke_point;
pub mod to_bez_path;
use std::collections::HashMap;

use crate::kanji_node::KanjiNode;

pub type KanjiMap = HashMap<char, KanjiNode>;
