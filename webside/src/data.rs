use std::rc::Rc;

use japanese_input::KanjiMap;
use japanese_input::analyze::Analyzer;
use japanese_input::recognizer::Recognizer;

/// Never committed — `build.rs` copies it in from `data/generated/reference_data.bin` (generated locally, or freshly in CI, by `cargo test tests::generate_reference_data`), same OUT_DIR pattern as the recognizer model below.
const REFERENCE_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/reference_data.bin"));
const MODEL_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/recognizer_model.bin"));

/// Everything the app needs, parsed once at startup and shared via context.
pub struct AppData {
    pub kanji_map: KanjiMap,
    pub analyzer: Analyzer,
    /// `None` in local/dev builds, where `build.rs` embeds an empty placeholder instead of the real (never-committed) recognizer model.
    pub recognizer: Option<Recognizer>,
}

pub type AppDataHandle = Rc<AppData>;

impl AppData {
    pub fn load() -> Self {
        let kanji_map: KanjiMap =
            postcard::from_bytes(REFERENCE_DATA).expect("reference_data.bin is a valid KanjiMap");
        let analyzer = Analyzer::new(kanji_map.clone());
        let recognizer = Recognizer::load(MODEL_BYTES).ok();
        Self {
            kanji_map,
            analyzer,
            recognizer,
        }
    }
}
