//! Shared plumbing for turning an `Analyzer::analyze_kanji` result into something a page can render, reused by both the Kanji Analysis page and the "kaki" (writing) side of the Vocab Review page.

use dioxus::prelude::*;
use japanese_input::analyze::AnalyzeResult;

#[derive(Clone, PartialEq)]
pub enum Outcome {
    Correct,
    Diff {
        correct_svg: String,
        wrong_svg: String,
    },
    NotInReferenceData,
}

impl Outcome {
    pub fn from_analysis(result: Option<AnalyzeResult>) -> Self {
        match result {
            None => Outcome::NotInReferenceData,
            Some(AnalyzeResult::NoError) => Outcome::Correct,
            Some(
                AnalyzeResult::StrokeOrder { correct, wrong }
                | AnalyzeResult::ExtraOrMissingStrokes { correct, wrong }
                | AnalyzeResult::StrokePositions { correct, wrong }
                | AnalyzeResult::WrongDrawn { correct, wrong },
            ) => Outcome::Diff {
                correct_svg: correct,
                wrong_svg: wrong,
            },
            Some(_) => Outcome::Diff {
                correct_svg: String::new(),
                wrong_svg: String::new(),
            },
        }
    }
}

pub fn view(outcome: &Outcome) -> Element {
    match outcome {
        Outcome::Correct => rsx! {
            p { class: "status-ok", "Looks correct!" }
        },
        Outcome::NotInReferenceData => rsx! {
            p { class: "status-bad", "This character isn't in the reference data." }
        },
        Outcome::Diff {
            correct_svg,
            wrong_svg,
        } => rsx! {
            div { class: "result-svg-row",
                figure {
                    div { dangerous_inner_html: "{wrong_svg}" }
                    figcaption { "What you drew" }
                }
                figure {
                    div { dangerous_inner_html: "{correct_svg}" }
                    figcaption { "Correct" }
                }
            }
        },
    }
}
