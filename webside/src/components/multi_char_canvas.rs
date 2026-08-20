use dioxus::prelude::*;
use japanese_input::gen_svg::SVGBuilder;

use super::StrokeCanvas;
use super::stroke_canvas::{VIEWBOX, path_d};
use crate::data::AppDataHandle;

const GRID_COLOR: &str = "var(--border)";
const HINT_COLOR: &str = "var(--fg-muted)";
const CORNER_RADIUS: f32 = 6.0;
/// However many 44px slots (see `.committed-thumb`/`.committed-preview` in style.css) fit across the 300px canvas width — shown upfront regardless of how many characters the current word actually needs, same idea as the addon's own fixed `INITIAL_SLOTS`, just sized to this layout instead.
const INITIAL_SLOTS: usize = 6;

/// Draws one word/reading character by character, mirroring the addon's own `InputWidget` (`japanese_input_anki/widgets.py`) both in layout — a slot row above the drawing grid, each slot showing the same grid background as the canvas itself, filled or not — and in behavior: Undo (取消) pops a stroke, or the last committed character once the canvas is empty; Next (次へ) commits the current character; Hint (手本) toggles a stroke-order reference for whichever character comes next.
#[component]
pub fn MultiCharCanvas(
    commits: Signal<Vec<Vec<Vec<(f32, f32)>>>>,
    /// The in-progress character's strokes. Owned by the caller (not created here) so it can auto-commit pending strokes before checking, mirroring `InputWidget.auto_commit_pending`.
    current: Signal<Vec<Vec<(f32, f32)>>>,
    target: Vec<char>,
) -> Element {
    let data = use_context::<AppDataHandle>();
    let mut commits = commits;
    let mut current = current;
    let mut show_hint = use_signal(|| false);
    let target_len = target.len();
    let slot_count = commits().len().max(INITIAL_SLOTS);

    // Every slot (empty or filled) shows this same plain grid as its background, exactly like the addon's `GridViewer` — only the active canvas below ever shows the hint overlay on top of it.
    let plain_grid_svg = use_memo(|| {
        SVGBuilder::init()
            .draw_grid(GRID_COLOR, CORNER_RADIUS)
            .to_string()
    });

    let data_for_bg = data.clone();
    let target_for_bg = target.clone();
    let canvas_svg = use_memo(move || {
        let mut svg = SVGBuilder::init().draw_grid(GRID_COLOR, CORNER_RADIUS);
        if show_hint()
            && let Some(&ch) = target_for_bg.get(commits().len())
            && let Some(node) = data_for_bg.kanji_map.get(&ch)
        {
            svg = svg.draw_hint(&node.collect_paths(), HINT_COLOR);
        }
        svg.to_string()
    });

    // Mirrors `InputWidget._on_undo`: pop a stroke, or — once the canvas is empty — the last committed character instead.
    let undo = move |_| {
        if current().is_empty() {
            commits.write().pop();
        } else {
            current.write().pop();
        }
    };

    // Commits the current character, then clears the canvas and any hint, same as the addon's `_on_commit` before the next slot — except an empty canvas is allowed through as-is (an empty commit), letting the user skip a character they don't know how to draw instead of getting stuck on it.
    let commit = move |_| {
        let strokes: Vec<Vec<(f32, f32)>> = current.write().drain(..).collect();
        commits.write().push(strokes);
        show_hint.set(false);
    };

    // Mirrors `_on_hint`: toggle the reference for whichever character is next, i.e. at index `len(committed)`.
    let toggle_hint = move |_| show_hint.set(!show_hint());

    rsx! {
        div { class: "multi-char-canvas",
            div { class: "committed-preview",
                for i in 0..slot_count {
                    div { key: "{i}", class: "committed-thumb",
                        div { class: "committed-thumb-bg", dangerous_inner_html: "{plain_grid_svg}" }
                        if let Some(char_strokes) = commits().get(i) {
                            svg { class: "committed-thumb-fg", view_box: "0 0 {VIEWBOX} {VIEWBOX}",
                                for stroke in char_strokes {
                                    path {
                                        d: "{path_d(stroke)}",
                                        fill: "none",
                                        stroke: "var(--ink)",
                                        stroke_width: "5",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                    }
                                }
                            }
                        }
                    }
                }
            }
            StrokeCanvas {
                strokes: current,
                background_svg: Some(canvas_svg()),
                show_actions: false,
            }
            div { class: "stroke-canvas-actions",
                button {
                    class: "btn",
                    disabled: current().is_empty() && commits().is_empty(),
                    onclick: undo,
                    "取消"
                }
                button { class: "btn", onclick: commit, "次へ" }
                button {
                    class: "btn",
                    disabled: commits().len() >= target_len,
                    onclick: toggle_hint,
                    "手本"
                }
            }
        }
    }
}
