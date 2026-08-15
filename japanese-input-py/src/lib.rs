use pyo3::prelude::*;
#[pymodule]
pub mod japanese_input_py {
    use japanese_input::analyze::AnalyzeResult as AnalyzeResultNative;
    use japanese_input::analyze::Analyzer as AnalyzerNative;
    use japanese_input::{
        KanjiMap,
        gen_svg::{gen_kanji_grid, gen_kanji_grid_with_hint},
        recognizer::Recognizer as NativeRecognizer,
        stroke_point::ToStrokeVector as _,
    };
    use pyo3::exceptions::PyRuntimeError;
    use pyo3::prelude::*;
    use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
    use std::fs::read;

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
            gen_kanji_grid(grid_color, corner_radius)
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
            Ok(gen_kanji_grid_with_hint(
                grid_color,
                corner_radius,
                &hint.collect_paths(),
                hint_color,
            ))
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
