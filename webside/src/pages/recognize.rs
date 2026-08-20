use dioxus::prelude::*;
use japanese_input::gen_svg::SVGBuilder;

use crate::components::StrokeCanvas;
use crate::data::AppDataHandle;

const GRID_COLOR: &str = "var(--border)";
const CORNER_RADIUS: f32 = 6.0;

#[component]
pub fn RecognizePage() -> Element {
    let data = use_context::<AppDataHandle>();

    if data.recognizer.is_none() {
        return rsx! {
            div { class: "card",
                h1 { "Handwriting Recognition" }
                div { class: "notice",
                    "This preview build doesn't have the recognizer model embedded, so this "
                    "demo isn't available here. On the deployed site, CI bakes the trained "
                    "model into the WASM build directly — the raw model file itself is never "
                    "committed to this repository."
                }
            }
        };
    }

    let strokes = use_signal(Vec::<Vec<(f32, f32)>>::new);
    let mut candidates = use_signal(Vec::<(char, f64)>::new);
    let background_svg =
        use_memo(|| SVGBuilder::init().draw_grid(GRID_COLOR, CORNER_RADIUS).to_string());

    let data_for_recognize = data.clone();
    let recognize = move |_| {
        if let Some(recognizer) = data_for_recognize.recognizer.as_ref() {
            let results = recognizer.recognize(&strokes());
            // `recognize` sorts best-first with the lowest raw score winning, and the raw scale isn't meaningful on its own. Shift every score by the same constant so the top candidate always reads as 1, and the rest grow from there by exactly their original spacing from the top.
            let top_score = results.first().map(|r| r.score);
            candidates.set(
                results
                    .into_iter()
                    .take(5)
                    .map(|r| {
                        let shown = top_score.map_or(r.score, |top| r.score - top + 1.0);
                        (r.character, shown)
                    })
                    .collect(),
            );
        }
    };

    rsx! {
        div { class: "card",
            h1 { "Handwriting Recognition" }
            p { "Draw a character and let the recognizer identify it." }

            StrokeCanvas { strokes, background_svg: Some(background_svg()) }

            div { class: "stroke-canvas-actions",
                button {
                    class: "btn-primary",
                    disabled: strokes().is_empty(),
                    onclick: recognize,
                    "Recognize"
                }
            }

            if !candidates().is_empty() {
                div { class: "candidate-list",
                    for (ch , score) in candidates() {
                        div { class: "candidate",
                            span { class: "char", "{ch}" }
                            span { class: "score", "{score:.2}" }
                        }
                    }
                }
            }
        }
    }
}
