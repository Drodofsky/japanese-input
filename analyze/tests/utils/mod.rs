use std::path::PathBuf;

use analyze::{KanjiMap, analyze::AnalyzedKanjiNode};
use serde::{Deserialize, Serialize};

/// # Panics
/// Panics if `data/generated/kanji.bin` cannot be read or deserialized.
#[must_use]
pub fn load_kanji_map() -> KanjiMap {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/generated/kanji.bin");
    let bytes = std::fs::read(path).expect("failed to read kanji.bin");
    postcard::from_bytes(&bytes).expect("failed to deserialize kanji map")
}

/// # Panics
/// Panics if `data/generated/hiragana.bin` cannot be read or deserialized.
#[must_use]
pub fn load_hiragana_map() -> KanjiMap {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/generated/hiragana.bin");
    let bytes = std::fs::read(path).expect("failed to read hiragana.bin");
    postcard::from_bytes(&bytes).expect("failed to deserialize hiragana map")
}

/// # Panics
/// Panics if `c` is not present in `map`.
#[must_use]
pub fn analyzed(map: &KanjiMap, c: char) -> AnalyzedKanjiNode {
    let node = map.get(&c).unwrap_or_else(|| panic!("kanji {c} not found"));
    AnalyzedKanjiNode::from_node(node)
}

/// # Panics
/// Panics if `data/test/{name}.bin` cannot be read or deserialized.
#[must_use]
pub fn load_test_file(name: &str) -> Vec<Vec<(f32, f32)>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data")
        .join("test")
        .join(format!("{name}.bin"));
    let bytes = std::fs::read(&path).unwrap_or_else(|_| panic!("failed to read {name}.bin"));
    let file: StrokeFile = postcard::from_bytes(&bytes).expect("failed to deserialize stroke file");
    file.strokes
}

#[derive(Deserialize, Serialize)]
pub struct StrokeFile {
    pub character: char,
    pub strokes: Vec<Vec<(f32, f32)>>,
}

#[must_use]
pub fn match_node(
    reference: &AnalyzedKanjiNode,
    user: &[Vec<(f32, f32)>],
) -> Vec<analyze::match_node::MatchInfo> {
    analyze::match_node::match_node(reference, user)
}
