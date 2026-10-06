//! The `.bin` stroke file format shared with the addon repo's desktop `kanji-draw` tool and its test set: a postcard-encoded `StrokeFile`, strokes normalized to `0..1`. Kept field-for-field identical to `kanji-draw`'s own struct so files round-trip between the two.

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StrokeFile {
    pub character: char,
    pub strokes: Vec<Vec<(f32, f32)>>,
}

impl StrokeFile {
    pub fn encode(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("a StrokeFile always serializes")
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every file `kanji-draw` wrote into the addon repo's test set must decode, and re-encode to the exact same bytes.
    #[test]
    fn kanji_draw_files_round_trip() {
        let Ok(dir) = std::fs::read_dir("../data/test") else {
            return;
        };
        for path in dir.filter_map(Result::ok).map(|e| e.path()) {
            if path.extension().is_none_or(|ext| ext != "bin") {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            let file =
                StrokeFile::decode(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(file.encode(), bytes, "{}", path.display());
        }
    }
}
