use pyo3::prelude::*;
const CONFUSABLE_GROUPS: [&str; 13] = [
    "へヘ", "べベ", "ぺペ", "イ亻", "エ工", "カ力", "タ夕", "ト卜", "ニ二", "ネ礻", "ム厶", "ロ口",
    "ー一",
];

fn correct_char(expected_char: char, drawn_chars: [char; 2]) -> char {
    for drawn_char in drawn_chars {
        if drawn_char == expected_char {
            return drawn_char;
        }
        for group in CONFUSABLE_GROUPS {
            if group.contains(expected_char) && group.contains(drawn_char) {
                return expected_char;
            }
        }
    }
    drawn_chars[0]
}
fn correct_answer(expected: &str, drawn: &[[char; 2]]) -> String {
    let mut out: String = drawn
        .iter()
        .zip(expected.chars())
        .map(|(d, e)| correct_char(e, *d))
        .collect();
    if out.len() < drawn.len() {
        drawn.iter().skip(out.len()).for_each(|s| out.push(s[0]));
    }
    out
}

#[pymodule]
pub mod japanese_input_py {
    use japanese_input::analyze::AnalyzeResult as AnalyzeResultNative;
    use japanese_input::analyze::Analyzer as AnalyzerNative;
    use japanese_input::gen_svg::SVGBuilder;
    use japanese_input::{
        KanjiMap, recognizer::Recognizer as NativeRecognizer, stroke_point::ToStrokeVector as _,
    };
    use pyo3::exceptions::PyRuntimeError;
    use pyo3::prelude::*;
    use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
    use std::fs::read;

    use crate::correct_answer;

    #[pyclass]
    pub struct Recognizer {
        recognizer: NativeRecognizer,
    }

    #[pymethods]
    impl Recognizer {
        #[new]
        fn new(map_path: &str) -> PyResult<Self> {
            let model = read(map_path)
                .map_err(|e| PyRuntimeError::new_err(format!("failed to read {map_path}: {e}")))?;
            let recognizer = NativeRecognizer::load(&model)
                .map_err(|e| PyRuntimeError::new_err(format!("failed to load model: {e}")))?;

            Ok(Self { recognizer })
        }

        fn recognize(&self, committed: Vec<Vec<Vec<(f32, f32)>>>) -> String {
            committed
                .par_iter()
                .map(|strokes| {
                    self.recognizer
                        .recognize(strokes)
                        .first()
                        .map_or('-', |r| r.character)
                })
                .collect()
        }
        fn compare_with_target(
            &self,
            committed: Vec<Vec<Vec<(f32, f32)>>>,
            target: String,
        ) -> String {
            let recognized: Vec<_> = committed
                .par_iter()
                .map(|strokes| {
                    let results = self.recognizer.recognize(strokes);
                    [
                        results.first().map_or('-', |r| r.character),
                        results.get(1).map_or('-', |r| r.character),
                    ]
                })
                .collect();
            correct_answer(&target, &recognized)
        }
    }

    #[pyclass]
    pub struct KanjiGrid {
        kanji_map: KanjiMap,
    }
    #[pymethods]
    impl KanjiGrid {
        #[new]
        fn new(map_path: &str) -> PyResult<Self> {
            let bytes = read(map_path)
                .map_err(|e| PyRuntimeError::new_err(format!("failed to read {map_path}: {e}")))?;
            let map: KanjiMap = postcard::from_bytes(&bytes)
                .map_err(|e| PyRuntimeError::new_err(format!("failed to deserialize: {e}")))?;
            Ok(Self { kanji_map: map })
        }
        #[expect(clippy::unused_self, reason = "pyo3")]
        fn generate(&self, grid_color: &str, corner_radius: f32) -> String {
            SVGBuilder::init()
                .draw_grid(grid_color, corner_radius)
                .to_string()
        }
        fn generate_with_hint(
            &self,
            grid_color: &str,
            corner_radius: f32,
            hint: char,
            hint_color: &str,
        ) -> PyResult<String> {
            let hint = self
                .kanji_map
                .get(&hint)
                .ok_or(PyRuntimeError::new_err(format!(
                    "failed to find hint for '{hint}'"
                )))?;
            Ok(SVGBuilder::init()
                .draw_grid(grid_color, corner_radius)
                .draw_hint(&hint.collect_paths(), hint_color)
                .to_string())
        }
    }
    #[pyclass(skip_from_py_object)]
    #[non_exhaustive]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum ResultKind {
        WrongDrawn,
        StrokeInsertedOrRemoved,
        StrokeMovedOrScaled,
        StrokeOrder,
        NothingFound,
        Unknown,
    }

    #[pyclass]
    pub struct AnalyzeResult {
        kind: ResultKind,
        correct: String,
        wrong: String,
    }
    #[pymethods]
    impl AnalyzeResult {
        #[getter]
        fn kind(&self) -> ResultKind {
            self.kind
        }
        #[getter]
        fn correct(&self) -> &str {
            &self.correct
        }
        #[getter]
        fn wrong(&self) -> &str {
            &self.wrong
        }
    }

    impl From<AnalyzeResultNative> for AnalyzeResult {
        #[inline]
        fn from(analyze_result_native: AnalyzeResultNative) -> Self {
            match analyze_result_native {
                AnalyzeResultNative::StrokeOrder { correct, wrong } => AnalyzeResult {
                    kind: ResultKind::StrokeOrder,
                    correct,
                    wrong,
                },
                AnalyzeResultNative::ExtraOrMissingStrokes { correct, wrong } => AnalyzeResult {
                    kind: ResultKind::StrokeInsertedOrRemoved,
                    correct,
                    wrong,
                },
                AnalyzeResultNative::StrokePositions { correct, wrong } => AnalyzeResult {
                    kind: ResultKind::StrokeMovedOrScaled,
                    correct,
                    wrong,
                },
                AnalyzeResultNative::NoError => AnalyzeResult {
                    kind: ResultKind::NothingFound,
                    correct: String::new(),
                    wrong: String::new(),
                },
                AnalyzeResultNative::WrongDrawn { correct, wrong } => AnalyzeResult {
                    kind: ResultKind::WrongDrawn,
                    correct,
                    wrong,
                },
                _ => AnalyzeResult {
                    kind: ResultKind::Unknown,
                    correct: String::new(),
                    wrong: String::new(),
                },
            }
        }
    }

    #[pyclass]
    pub struct Analyzer {
        native: AnalyzerNative,
    }
    #[pymethods]
    impl Analyzer {
        #[new]
        fn new(map_path: &str) -> PyResult<Self> {
            let bytes = read(map_path)
                .map_err(|e| PyRuntimeError::new_err(format!("failed to read {map_path}: {e}")))?;
            let map: KanjiMap = postcard::from_bytes(&bytes)
                .map_err(|e| PyRuntimeError::new_err(format!("failed to deserialize: {e}")))?;
            Ok(Self {
                native: AnalyzerNative::new(map),
            })
        }
        /// Analyzes the drawn `strokes` for `kanji` and returns a rendered result.
        ///
        /// # Errors
        /// Returns a `PyRuntimeError` if the kanji is not found in the map or the
        /// strokes cannot be analyzed.
        #[inline]
        pub fn analyze(
            &self,
            kanji: char,
            strokes: Vec<Vec<(f32, f32)>>,
            grid_color: &str,
            corner_radius: f32,
            stroke_color: &str,
        ) -> PyResult<AnalyzeResult> {
            let res = self
                .native
                .analyze_kanji(
                    kanji,
                    strokes.to_stroke_vector(),
                    grid_color,
                    corner_radius,
                    stroke_color,
                )
                .map(AnalyzeResult::from);
            res.ok_or(PyRuntimeError::new_err(
                "failed to analyze Kanji".to_owned(),
            ))
        }
    }
}
