use std::path::PathBuf;

use japanese_input::{
    KanjiMap,
    analyze::{AnalyzeResult, Analyzer},
    analyzed_kanji_node::{self, AnalyzedKanjiNode},
    match_strokes::Weights,
    stroke_point::ToStrokeVector,
};
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
/// Panics if `data/generated/kanji.bin` cannot be read or deserialized.
#[must_use]
pub fn load_recognizer_model() -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/generated/recognizer_model.bin");
    std::fs::read(path).expect("failed to read kanji.bin")
}

/// # Panics
/// Panics if `data/generated/reference_data.bin` cannot be read or deserialized.
#[must_use]
pub fn load_reference_map() -> KanjiMap {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/generated/reference_data.bin");
    let bytes = std::fs::read(path).expect("failed to read reference_data.bin");
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
/// # Panics
/// Panics if `c` is not present in `map`.
#[must_use]
pub fn load_kanji_node(map: &KanjiMap, c: char) -> AnalyzedKanjiNode {
    let node = map.get(&c).unwrap_or_else(|| panic!("kanji {c} not found"));
    node.clone().to_analyzed()
}

#[must_use]
pub fn match_strokes(
    reference: AnalyzedKanjiNode,
    user: &[Vec<(f32, f32)>],
) -> Vec<japanese_input::match_strokes::MatchInfo> {
    japanese_input::match_strokes::match_strokes(
        reference,
        user.to_stroke_vector(),
        Weights::default(),
        100,
    )
    .clone()
}

#[must_use]
pub fn analyze(kanji: char, test_strokes: &str) -> AnalyzeResult {
    let kanji_map = load_kanji_map();
    let analyzer = Analyzer::new(kanji_map);
    let user_strokes = load_test_file(test_strokes).to_stroke_vector();
    let grid_color = "gray";
    let corner_radius = 8.0;
    let stroke_color = "darkgrey";
    analyzer
        .analyze_kanji(kanji, user_strokes, grid_color, corner_radius, stroke_color)
        .unwrap()
}

#[derive(Deserialize, Serialize)]
pub struct StrokeFile {
    pub character: char,
    pub strokes: Vec<Vec<(f32, f32)>>,
}
