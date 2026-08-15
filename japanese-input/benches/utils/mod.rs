use std::path::PathBuf;

use japanese_input::KanjiMap;
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
/// Panics if `data/generated/recognizer2_tuned.bin` cannot be read.
#[must_use]
pub fn load_recognizer_model() -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/generated/recognizer2_tuned.bin");
    std::fs::read(path).expect("failed to read recognizer2_tuned.bin")
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
