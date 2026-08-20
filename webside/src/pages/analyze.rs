use dioxus::prelude::*;
use japanese_input::gen_svg::SVGBuilder;
use japanese_input::stroke_point::ToStrokeVector as _;

use crate::components::StrokeCanvas;
use crate::data::AppDataHandle;
use crate::outcome::Outcome;

const GRID_COLOR: &str = "var(--border)";
const HINT_COLOR: &str = "var(--fg-muted)";
const STROKE_COLOR: &str = "var(--fg)";
const CORNER_RADIUS: f32 = 6.0;

// All Kanken level 10 (grade-1 kyōiku kanji).
const DEMO_CHARS: &[char] = &[
    '一', '二', '三', '人', '山', '川', '水', '大', '小', '本', '日', '月', '火', '木', '金', '土',
    '音', '青',
];

#[component]
pub fn AnalyzePage() -> Element {
    let data = use_context::<AppDataHandle>();
    let mut selected = use_signal(|| None::<char>);
    let mut show_hint = use_signal(|| false);
    let mut strokes = use_signal(Vec::<Vec<(f32, f32)>>::new);
    let mut outcome = use_signal(|| None::<Outcome>);
    let mut error = use_signal(|| None::<&'static str>);

    let data_for_bg = data.clone();
    let background_svg = use_memo(move || {
        let mut svg = SVGBuilder::init().draw_grid(GRID_COLOR, CORNER_RADIUS);
        if show_hint()
            && let Some(ch) = selected()
            && let Some(node) = data_for_bg.kanji_map.get(&ch)
        {
            svg = svg.draw_hint(&node.collect_paths(), HINT_COLOR);
        }
        svg.to_string()
    });

    // Starts the character row pre-scrolled a bit so it visibly reads as a sliding list rather than looking cut off at the left edge. Runs once on mount (no reactive reads in the body, so this effect never re-fires).
    use_effect(move || {
        document::eval(
            "const el = document.querySelector('.char-row');\
             if (el) { el.scrollLeft = el.scrollWidth * 0.1; }",
        );
    });

    let data_for_check = data.clone();
    let check = move |_| {
        let Some(ch) = selected() else {
            error.set(Some("Pick a character first."));
            outcome.set(None);
            return;
        };
        error.set(None);
        let user_strokes = strokes().to_stroke_vector();
        let result = data_for_check.analyzer.analyze_kanji(
            ch,
            user_strokes,
            GRID_COLOR,
            CORNER_RADIUS,
            STROKE_COLOR,
        );
        outcome.set(Some(Outcome::from_analysis(result)));
    };

    rsx! {
        div { class: "card",
            h1 { "Kanji Analysis" }
            p { "Pick a character, draw it, and let the analyzer check your handwriting." }
            div { class: "char-row",
                for c in DEMO_CHARS {
                    button {
                        class: if selected() == Some(*c) { "char-tile active" } else { "char-tile" },
                        onclick: move |_| {
                            selected.set(Some(*c));
                            strokes.write().clear();
                            outcome.set(None);
                            error.set(None);
                        },
                        "{c}"
                    }
                }
            }
            StrokeCanvas { strokes, background_svg: Some(background_svg()),
                button {
                    class: if show_hint() { "btn-primary" } else { "btn" },
                    onclick: move |_| show_hint.set(!show_hint()),
                    "Hint"
                }
            }

            div { class: "stroke-canvas-actions",
                button {
                    class: "btn-primary",
                    disabled: strokes().is_empty(),
                    onclick: check,
                    "Check"
                }
            }

            if let Some(msg) = error() {
                p { class: "status-bad", "{msg}" }
            }

            match outcome() {
                None => rsx! {},
                Some(o) => crate::outcome::view(&o),
            }
        }
    }
}
