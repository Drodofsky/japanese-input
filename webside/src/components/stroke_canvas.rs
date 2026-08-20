use dioxus::prelude::*;

/// The SVG display grid is 109x109 (see `japanese-input/src/gen_svg.rs::init_doc`); used only to scale normalized points up for the on-screen preview path.
pub(crate) const VIEWBOX: f64 = 109.0;
/// The canvas is drawn at a fixed CSS pixel size (see `.stroke-canvas` in style.css), which lets us convert pointer offsets to normalized coordinates without any JS interop to measure the element.
const CANVAS_PX: f64 = 300.0;

/// `Analyzer`/`Recognizer` expect strokes normalized to roughly `0..1` (matching the addon's own `GridDrawViewer._norm`, which divides captured pointer positions by the canvas size in pixels) — *not* the 109x109 units the reference SVGs happen to render at. Storing anything else here makes every analysis/recognition call silently garbage: strokes ~109x too large compared to the reference data, which is what produced "blank" result SVGs (the drawn path scaled miles outside the viewBox).
fn to_normalized_point(evt: &PointerEvent) -> (f32, f32) {
    let p = evt.element_coordinates();
    ((p.x / CANVAS_PX) as f32, (p.y / CANVAS_PX) as f32)
}

/// Builds the `d` attribute for the on-screen preview path, scaling the normalized `0..1` points up to the 109x109 display grid. Also reused by `MultiCharCanvas` to render small previews of already-committed characters.
pub(crate) fn path_d(points: &[(f32, f32)]) -> String {
    let mut d = String::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        let (x, y) = (f64::from(x) * VIEWBOX, f64::from(y) * VIEWBOX);
        if i == 0 {
            d.push_str(&format!("M{x} {y} "));
        } else {
            d.push_str(&format!("L{x} {y} "));
        }
    }
    d
}

/// A drawing surface that captures pointer strokes as `Vec<Vec<(f32, f32)>>` in the addon's own 109x109 coordinate space, with an optional background SVG (a grid, or a grid + hint strokes rendered via `SVGBuilder`).
#[component]
pub fn StrokeCanvas(
    strokes: Signal<Vec<Vec<(f32, f32)>>>,
    #[props(default)] background_svg: Option<String>,
    /// Set to `false` to suppress the built-in Undo/Clear row, e.g. when a caller (like `MultiCharCanvas`) wants full control over its own actions instead.
    #[props(default = true)]
    show_actions: bool,
    /// Extra buttons rendered alongside Undo/Clear, e.g. a hint toggle.
    children: Element,
) -> Element {
    let mut strokes = strokes;
    let mut current = use_signal(Vec::<(f32, f32)>::new);
    let mut drawing = use_signal(|| false);

    let start = move |evt: PointerEvent| {
        evt.stop_propagation();
        drawing.set(true);
        current.set(vec![to_normalized_point(&evt)]);
    };
    let mv = move |evt: PointerEvent| {
        if !drawing() {
            return;
        }
        evt.stop_propagation();
        current.write().push(to_normalized_point(&evt));
    };
    let end = move |_evt: PointerEvent| {
        if !drawing() {
            return;
        }
        drawing.set(false);
        let stroke: Vec<(f32, f32)> = current.write().drain(..).collect();
        if stroke.len() > 1 {
            strokes.write().push(stroke);
        }
    };

    rsx! {
        div { class: "stroke-canvas",
            if let Some(bg) = &background_svg {
                div { class: "stroke-canvas-bg", dangerous_inner_html: "{bg}" }
            }
            svg {
                class: "stroke-canvas-fg",
                view_box: "0 0 {VIEWBOX} {VIEWBOX}",
                onpointerdown: start,
                onpointermove: mv,
                onpointerup: end,
                onpointerleave: end,
                for stroke in strokes() {
                    path {
                        d: "{path_d(&stroke)}",
                        fill: "none",
                        stroke: "var(--ink)",
                        stroke_width: "3",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                    }
                }
                path {
                    d: "{path_d(&current())}",
                    fill: "none",
                    stroke: "var(--accent)",
                    stroke_width: "3",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                }
            }
        }
        if show_actions {
            div { class: "stroke-canvas-actions",
                button {
                    class: "btn",
                    disabled: strokes().is_empty(),
                    onclick: move |_| {
                        strokes.write().pop();
                    },
                    "Undo"
                }
                button {
                    class: "btn",
                    disabled: strokes().is_empty(),
                    onclick: move |_| strokes.write().clear(),
                    "Clear"
                }
                {children}
            }
        }
    }
}
