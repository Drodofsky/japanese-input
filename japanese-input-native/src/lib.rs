use analyze::KanjiMap;
use analyze::analyze::{Analysis, StrokeIssue};
use analyze::recognize_hiragana::HiraganaRecognizer;
use analyze::recognize_kanji::KanjiRecognizer;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use wana_kana::IsJapaneseChar;
#[pyclass]
#[derive(Clone)]
pub enum PyStrokeIssue {
    Missing { ref_index: usize },
    Extra { user_index: usize },
    WrongOrder {},
    PositionCorrection { depth: usize },
}

impl From<&StrokeIssue> for PyStrokeIssue {
    fn from(s: &StrokeIssue) -> Self {
        match s {
            StrokeIssue::Missing { ref_index } => PyStrokeIssue::Missing {
                ref_index: *ref_index,
            },
            StrokeIssue::Extra { user_index } => PyStrokeIssue::Extra {
                user_index: *user_index,
            },
            StrokeIssue::WrongOrder => PyStrokeIssue::WrongOrder {},
            StrokeIssue::PositionCorrection { depth } => {
                PyStrokeIssue::PositionCorrection { depth: *depth }
            }
        }
    }
}
#[pyclass]
#[derive(Clone)]
pub struct PyIssueWithFix {
    #[pyo3(get)]
    pub issue: PyStrokeIssue,
    #[pyo3(get)]
    pub corrected_strokes: Vec<Vec<(f32, f32)>>,
}

#[pymethods]
impl PyIssueWithFix {
    fn __repr__(&self) -> String {
        format!("IssueWithFix(strokes={})", self.corrected_strokes.len())
    }
}
#[pyclass]
#[derive(Clone)]
pub struct PyAnalysis {
    #[pyo3(get)]
    pub issues: Vec<PyIssueWithFix>,
    #[pyo3(get)]
    pub score: f32,
    #[pyo3(get)]
    pub stroke_qualities: Vec<Vec<f32>>,
    #[pyo3(get)]
    pub strokes: Vec<Vec<(f32, f32)>>,
}

#[pymethods]
impl PyAnalysis {
    fn __repr__(&self) -> String {
        format!(
            "Analysis(score={:.3}, issues={})",
            self.score,
            self.issues.len()
        )
    }
}

impl From<(Analysis, Vec<Vec<(f32, f32)>>)> for PyAnalysis {
    fn from(a: (Analysis, Vec<Vec<(f32, f32)>>)) -> Self {
        PyAnalysis {
            issues: a
                .0
                .issues
                .iter()
                .map(|i| PyIssueWithFix {
                    issue: PyStrokeIssue::from(&i.issue),
                    corrected_strokes: i.corrected_strokes.clone(),
                })
                .collect(),
            score: a.0.score,
            stroke_qualities: a.0.stroke_qualities,
            strokes: a.1,
        }
    }
}
#[pyclass]
pub struct KanjiAnalyzer {
    map: KanjiMap,
}

#[pymethods]
impl KanjiAnalyzer {
    #[new]
    fn new(map_path: &str) -> PyResult<Self> {
        let bytes = std::fs::read(map_path)
            .map_err(|e| PyRuntimeError::new_err(format!("failed to read {map_path}: {e}")))?;
        let map: KanjiMap = postcard::from_bytes(&bytes)
            .map_err(|e| PyRuntimeError::new_err(format!("failed to deserialize: {e}")))?;
        Ok(Self { map })
    }

    fn analyze(&self, committed: Vec<Vec<Vec<(f32, f32)>>>, expected: &str) -> Vec<PyAnalysis> {
        let mut out = Vec::new();
        let mut commits = committed.into_iter();

        for ch in expected.chars() {
            let Some(strokes) = commits.next() else {
                return out;
            };
            if !ch.is_kanji() {
                continue;
            }
            let Some(node) = self.map.get(&ch) else {
                continue;
            };
            let analysis = analyze::analyze::analyze(node, &strokes);
            out.push(PyAnalysis::from((analysis, strokes)));
        }

        out
    }
}
#[pyclass]
pub struct Recognizer {
    hiragana: HiraganaRecognizer,
    kanji: KanjiRecognizer,
}

#[pymethods]
impl Recognizer {
    #[new]
    fn new(hiragana_map_path: &str, kanji_map_path: &str) -> PyResult<Self> {
        let bytes = std::fs::read(hiragana_map_path).map_err(|e| {
            PyRuntimeError::new_err(format!("failed to read {hiragana_map_path}: {e}"))
        })?;
        let map: KanjiMap = postcard::from_bytes(&bytes)
            .map_err(|e| PyRuntimeError::new_err(format!("failed to deserialize: {e}")))?;
        let hiragana = HiraganaRecognizer::new(&map);
        let bytes = std::fs::read(kanji_map_path).map_err(|e| {
            PyRuntimeError::new_err(format!("failed to read {hiragana_map_path}: {e}"))
        })?;
        let map: KanjiMap = postcard::from_bytes(&bytes)
            .map_err(|e| PyRuntimeError::new_err(format!("failed to deserialize: {e}")))?;
        let kanji = KanjiRecognizer::new(&map);
        Ok(Self { hiragana, kanji })
    }

    fn analyze_answer(&self, committed: Vec<Vec<Vec<(f32, f32)>>>, expected: &str) -> String {
        let mut out = String::new();
        let mut commits = committed.into_iter();

        for ch in expected.chars() {
            let Some(strokes) = commits.next() else {
                return out;
            };
            if ch.is_hiragana() {
                if let Some(top) = self.hiragana.recognize(&strokes).into_iter().next() {
                    out.push(top.character);
                }
            } else if ch.is_kanji() {
                if let Some(top) = self.kanji.recognize(&strokes).into_iter().next() {
                    out.push(top.character);
                }
            } else {
                out.push(ch);
            }
        }

        // default try recognize as hiragana
        for strokes in commits {
            if let Some(top) = self.hiragana.recognize(&strokes).into_iter().next() {
                out.push(top.character);
            }
        }
        out
    }
}

#[pymodule]
fn japanese_input_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Recognizer>()?;
    m.add_class::<KanjiAnalyzer>()?;
    m.add_class::<PyAnalysis>()?;
    m.add_class::<PyIssueWithFix>()?;
    m.add_class::<PyStrokeIssue>()?;
    Ok(())
}
