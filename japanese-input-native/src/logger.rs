use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use wana_kana::IsJapaneseChar;

use crate::{PyAnalysis, PyIssueWithFix, PyStrokeIssue};

// ── on-disk types (postcard-serialized) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharType {
    Hiragana,
    Katakana,
    Kanji,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogIssue {
    /// "Missing" | "Extra" | "WrongOrder" | "PositionCorrection"
    pub kind: String,
    /// correction score (PositionCorrection only)
    pub score: Option<f32>,
}

/// One character's worth of practice data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// `japanese-input-native` crate version that produced this entry.
    pub version: String,
    /// Unix time in milliseconds when the answer was shown.
    pub timestamp_ms: u64,
    /// The character the user was supposed to write.
    pub expected: char,
    /// The character the recognizer returned (None if not determined).
    pub recognized: Option<char>,
    /// Whether the recognizer matched the expected character.
    pub correct: bool,
    pub char_type: CharType,
    /// Raw user strokes (normalised [0,1] coordinates).
    pub strokes: Vec<Vec<(f32, f32)>>,
    /// Analysis score (kanji only, None for hiragana/katakana/other).
    pub score: Option<f32>,
    /// Issues found during analysis (kanji only).
    pub issues: Vec<LogIssue>,
    /// Anki ease rating the user pressed: 1=Again, 2=Hard, 3=Good, 4=Easy.
    /// None when not yet rated (should not appear in completed log files).
    pub rating: Option<u8>,
}

// ── file format ──────────────────────────────────────────────────────────────

/// Append one `LogEntry` to `file` as a length-prefixed postcard record.
///
/// Format per entry:
///   [u32 LE length][postcard bytes …]
///
/// This lets a reader stream through the file without a framing header.
fn append_entry(file: &mut impl Write, entry: &LogEntry) -> std::io::Result<()> {
    let bytes = postcard::to_allocvec(entry).map_err(std::io::Error::other)?;
    let len =
        u32::try_from(bytes.len()).map_err(|_| std::io::Error::other("log entry too large"))?;
    file.write_all(&len.to_le_bytes())?;
    file.write_all(&bytes)?;
    Ok(())
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn issue_kind(issue: &PyStrokeIssue) -> String {
    match issue {
        PyStrokeIssue::Missing { .. } => "Missing".into(),
        PyStrokeIssue::Extra { .. } => "Extra".into(),
        PyStrokeIssue::WrongOrder {} => "WrongOrder".into(),
        PyStrokeIssue::PositionCorrection { .. } => "PositionCorrection".into(),
    }
}

fn issue_score(issue: &PyStrokeIssue) -> Option<f32> {
    if let PyStrokeIssue::PositionCorrection { score, .. } = issue {
        Some(*score)
    } else {
        None
    }
}

fn char_type(ch: char) -> CharType {
    if ch.is_hiragana() {
        CharType::Hiragana
    } else if ch.is_katakana() {
        CharType::Katakana
    } else if ch.is_kanji() {
        CharType::Kanji
    } else {
        CharType::Other
    }
}

// ── pyclass ──────────────────────────────────────────────────────────────────

/// Append-only log of practice sessions.
#[pyclass]
pub struct StrokeLogger {
    path: PathBuf,
}

#[pymethods]
impl StrokeLogger {
    /// Open (or create) a log file at `path`.
    #[new]
    pub fn new(path: &str) -> PyResult<Self> {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| {
                PyRuntimeError::new_err(format!("failed to create log directory: {e}"))
            })?;
        }
        Ok(Self { path })
    }

    /// Log one answer attempt.
    ///
    /// `rating` is the Anki ease the user pressed: 1=Again, 2=Hard, 3=Good,
    /// 4=Easy. Pass `None` only if the rating is genuinely unavailable.
    pub fn log(
        &self,
        py: Python<'_>,
        expected: &str,
        committed: Vec<Vec<Vec<(f32, f32)>>>,
        recognized: &str,
        analyses: Vec<Py<PyAnalysis>>,
        rating: Option<u8>,
    ) -> PyResult<()> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let recognized_chars: Vec<char> = recognized.chars().collect();
        let mut commits = committed.into_iter();
        let mut analysis_idx: usize = 0;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| PyRuntimeError::new_err(format!("failed to open log file: {e}")))?;

        for (i, ch) in expected.chars().enumerate() {
            let Some(strokes) = commits.next() else {
                break;
            };

            let recognized_ch = recognized_chars.get(i).copied();
            let correct = recognized_ch == Some(ch);

            // Analyses are produced only for kanji present in the map;
            // consume one per kanji character in `expected`.
            let (score, issues) = if ch.is_kanji() {
                if let Some(analysis_py) = analyses.get(analysis_idx) {
                    analysis_idx += 1;
                    let analysis: PyRef<'_, PyAnalysis> = analysis_py.borrow(py);
                    let issues = analysis
                        .issues
                        .iter()
                        .map(|iw: &PyIssueWithFix| LogIssue {
                            kind: issue_kind(&iw.issue),
                            score: issue_score(&iw.issue),
                        })
                        .collect();
                    (Some(analysis.score), issues)
                } else {
                    (None, vec![])
                }
            } else {
                (None, vec![])
            };

            let entry = LogEntry {
                version: env!("CARGO_PKG_VERSION").to_owned(),
                timestamp_ms: now_ms,
                expected: ch,
                recognized: recognized_ch,
                correct,
                char_type: char_type(ch),
                strokes,
                score,
                issues,
                rating,
            };

            append_entry(&mut file, &entry)
                .map_err(|e| PyRuntimeError::new_err(format!("failed to write log entry: {e}")))?;
        }

        Ok(())
    }
}
