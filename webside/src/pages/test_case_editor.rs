//! The Test Case Editor: create and edit `.bin` handwriting test cases — open them from a local folder (or draw a new one), check what `match_strokes` and the analyzer make of them, and hand-write the mapping a test should assert. Files stay byte-compatible with the addon repo's desktop `kanji-draw` tool (see `stroke_file.rs`), and every stroke-order picture is drawn by the library's own `SVGBuilder::draw_stroke_order`.

use dioxus::prelude::*;
use japanese_input::analyze::AnalyzeResult;
use japanese_input::gen_svg::SVGBuilder;
use japanese_input::match_strokes::{MISSING, match_strokes};
use japanese_input::stroke_point::ToStrokeVector as _;
use japanese_input::to_bez_path::ToBezPathVec as _;
use japanese_input::weights::Weights;

use crate::components::{VIEWBOX, path_d};
use crate::data::{AppData, AppDataHandle};
use crate::stroke_file::StrokeFile;

type Stroke = Vec<(f32, f32)>;

const GRID_COLOR: &str = "var(--border)";
const HINT_COLOR: &str = "var(--hint)";
const INK_COLOR: &str = "var(--ink)";
const CORNER_RADIUS: f32 = 6.0;
/// Same beam width `Analyzer::analyze_kanji` and `kanji-draw` match with, so the order shown here is the one the analyzer actually judged.
const BEAM_WIDTH: usize = 100;
const MAX_CANDIDATES: usize = 5;
const FOLDER_INPUT_ID: &str = "test-case-editor-folder-input";
const FILES_INPUT_ID: &str = "test-case-editor-files-input";

/// `strokes` numbered in the order `order` lists them, via the library's renderer.
fn stroke_order_svg(strokes: &[Stroke], order: &[u8], grid: bool) -> String {
    let paths = strokes.to_stroke_vector().iter().to_bez_path_vec();
    let mut svg = SVGBuilder::init();
    if grid {
        svg = svg.draw_grid(GRID_COLOR, CORNER_RADIUS);
    }
    svg.draw_stroke_order(&paths, order).to_string()
}

fn drawn_order(count: usize) -> Vec<u8> {
    (0..count)
        .map(|i| u8::try_from(i).unwrap_or(u8::MAX))
        .collect()
}

/// One undoable editing session over a drawing. All `Copy` signals, so event handlers can capture it freely.
#[derive(Clone, Copy)]
struct Editor {
    strokes: Signal<Vec<Stroke>>,
    undo_stack: Signal<Vec<Vec<Stroke>>>,
    redo_stack: Signal<Vec<Vec<Stroke>>>,
}

impl Editor {
    fn use_editor() -> Self {
        Self {
            strokes: use_signal(Vec::new),
            undo_stack: use_signal(Vec::new),
            redo_stack: use_signal(Vec::new),
        }
    }

    fn edit(mut self, f: impl FnOnce(&mut Vec<Stroke>)) {
        let before = self.strokes.peek().clone();
        self.undo_stack.write().push(before);
        self.redo_stack.write().clear();
        f(&mut self.strokes.write());
    }

    fn undo(mut self) {
        let Some(previous) = self.undo_stack.write().pop() else {
            return;
        };
        let current = std::mem::replace(&mut *self.strokes.write(), previous);
        self.redo_stack.write().push(current);
    }

    fn redo(mut self) {
        let Some(next) = self.redo_stack.write().pop() else {
            return;
        };
        let current = std::mem::replace(&mut *self.strokes.write(), next);
        self.undo_stack.write().push(current);
    }

    /// Starts a fresh session (no history) on `strokes`, e.g. after opening a file.
    fn replace(mut self, strokes: Vec<Stroke>) {
        self.strokes.set(strokes);
        self.undo_stack.write().clear();
        self.redo_stack.write().clear();
    }
}

#[derive(Clone, PartialEq)]
enum Verdict {
    NotInReference,
    Correct,
    Issue {
        label: &'static str,
        correct: String,
        wrong: String,
    },
}

impl Verdict {
    fn from_analysis(analysis: Option<AnalyzeResult>) -> Self {
        match analysis {
            None => Verdict::NotInReference,
            Some(AnalyzeResult::NoError) => Verdict::Correct,
            Some(result) => {
                let label = match &result {
                    AnalyzeResult::StrokeOrder { .. } => "Wrong stroke order",
                    AnalyzeResult::ExtraOrMissingStrokes { .. } => "Extra or missing strokes",
                    AnalyzeResult::StrokePositions { .. } => "Stroke positions off",
                    AnalyzeResult::WrongDrawn { .. } => "Drawing doesn't match",
                    _ => "Analysis issue",
                };
                Verdict::Issue {
                    label,
                    correct: result.correct().unwrap_or_default().to_owned(),
                    wrong: result.wrong().unwrap_or_default().to_owned(),
                }
            }
        }
    }
}

/// One mapping of the drawn strokes onto the reference, with everything shown for it.
#[derive(Clone, PartialEq)]
struct Candidate {
    order: Vec<u8>,
    /// `match_strokes`' score; `None` for a hand-written mapping, which the matcher never scored.
    score: Option<f64>,
    /// The drawn strokes renumbered into the order this mapping assigns them.
    svg: String,
    /// What the analyzer makes of the drawing under this mapping.
    verdict: Verdict,
}

impl Candidate {
    fn new(
        data: &AppData,
        character: char,
        drawn: &[Stroke],
        order: Vec<u8>,
        score: Option<f64>,
    ) -> Self {
        let verdict = Verdict::from_analysis(data.analyzer.analyze_kanji_with_mapping(
            character,
            drawn.to_stroke_vector(),
            order.clone(),
            GRID_COLOR,
            CORNER_RADIUS,
            INK_COLOR,
        ));
        Self {
            svg: stroke_order_svg(drawn, &order, true),
            order,
            score,
            verdict,
        }
    }
}

#[derive(Clone, PartialEq)]
struct Report {
    character: char,
    /// The drawing this report was computed from, to flag it stale once the strokes change.
    drawn: Vec<Stroke>,
    /// How many strokes the reference has, i.e. how long a mapping must be. 0 if the character isn't in the reference data.
    reference_count: usize,
    /// Best-first, from `match_strokes`.
    candidates: Vec<Candidate>,
    recognized: Vec<(char, f64)>,
}

fn build_report(data: &AppData, character: char, drawn: &[Stroke]) -> Report {
    let reference_count = data.kanji_map.get(&character).map_or(0, |n| n.leaf_count());
    let candidates = data
        .kanji_map
        .get(&character)
        .map(|node| {
            let mut matches = match_strokes(
                node.clone().to_analyzed(),
                drawn.to_stroke_vector(),
                Weights::default(),
                BEAM_WIDTH,
            );
            // Different search paths can end in the same mapping; list each one once.
            matches.dedup_by(|a, b| a.user_stroke_order == b.user_stroke_order);
            matches
                .into_iter()
                .take(MAX_CANDIDATES)
                .map(|m| {
                    Candidate::new(
                        data,
                        character,
                        drawn,
                        m.user_stroke_order.to_vec(),
                        Some(m.score),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    // Same display shift as the Recognition page: top candidate reads as 1, the rest keep their spacing from it.
    let recognized = data
        .recognizer
        .as_ref()
        .map(|recognizer| {
            let results = recognizer.recognize(drawn);
            let top = results.first().map(|r| r.score);
            results
                .into_iter()
                .take(MAX_CANDIDATES)
                .map(|r| (r.character, top.map_or(r.score, |t| r.score - t + 1.0)))
                .collect()
        })
        .unwrap_or_default();

    Report {
        character,
        drawn: drawn.to_vec(),
        reference_count,
        candidates,
        recognized,
    }
}

/// `[1,2,2,255,4]`-style, like `kanji-draw`'s mapping line: reference-position order, 1-indexed drawn strokes, a stroke joined into the one before it repeating that stroke's number, and `MISSING` passed through as 255.
fn format_mapping(order: &[u8]) -> String {
    let entries: Vec<String> = order
        .iter()
        .map(|&s| {
            if s != MISSING {
                (u16::from(s) + 1).to_string()
            } else {
                s.to_string()
            }
        })
        .collect();
    format!("[{}]", entries.join(","))
}

/// Parses a hand-written mapping in the same notation `format_mapping` prints (brackets optional): 1-indexed drawn strokes, repeated for a joined stroke, and `255` or `-` for a missing one.
fn parse_mapping(
    text: &str,
    drawn_count: usize,
    reference_count: usize,
) -> Result<Vec<u8>, String> {
    let order = text
        .split(|c: char| c == ',' || c == '[' || c == ']' || c.is_whitespace())
        .filter(|token| !token.is_empty())
        .map(|token| match token {
            "-" | "255" => Ok(MISSING),
            _ => token
                .parse::<usize>()
                .ok()
                .filter(|n| (1..=drawn_count).contains(n))
                .and_then(|n| u8::try_from(n - 1).ok())
                .ok_or_else(|| {
                    format!("“{token}” isn't one of your strokes (1–{drawn_count}) or - / 255")
                }),
        })
        .collect::<Result<Vec<u8>, String>>()?;
    if order.len() != reference_count {
        return Err(format!(
            "Needs {reference_count} entries, one per reference stroke — has {}.",
            order.len()
        ));
    }
    Ok(order)
}

/// The mapping as the addon repo's tests assert it: 0-indexed, joins repeated, `MISSING` by name.
fn rust_mapping(order: &[u8]) -> String {
    let entries: Vec<String> = order
        .iter()
        .map(|&s| match s {
            MISSING => "MISSING".to_owned(),
            s => s.to_string(),
        })
        .collect();
    format!("[{}]", entries.join(", "))
}

/// Strips anything that could break out of a filename or the JS string literal it's embedded into.
fn sanitize_filename(name: &str) -> String {
    let name = name.trim().trim_end_matches(".bin");
    name.chars()
        .filter(|c| !c.is_control() && !matches!(c, '/' | '\\' | '"' | '\'' | '`' | '<' | '>'))
        .collect()
}

fn download(filename: &str, bytes: &[u8]) {
    // Fully synchronous (bytes inlined, no `dioxus.recv`) so the click still counts as user-initiated on mobile browsers.
    document::eval(&format!(
        "const blob = new Blob([new Uint8Array({bytes:?})], {{ type: 'application/octet-stream' }});\
         const a = document.createElement('a');\
         a.href = URL.createObjectURL(blob);\
         a.download = {:?};\
         document.body.appendChild(a); a.click(); a.remove();\
         setTimeout(() => URL.revokeObjectURL(a.href), 1000);",
        format!("{filename}.bin"),
    ));
}

/// Every `.bin` the given file input currently holds, as `(name without extension, bytes)` sorted by name. Clears the input so picking the same folder again still fires `change`.
async fn read_picked_files(input_id: &str) -> Result<Vec<(String, Vec<u8>)>, document::EvalError> {
    let mut files: Vec<(String, Vec<u8>)> = document::eval(&format!(
        "const input = document.getElementById('{input_id}');\
         const picked = Array.from((input && input.files) || []).filter(f => f.name.endsWith('.bin'));\
         const out = [];\
         for (const f of picked) {{ out.push([f.name.slice(0, -4), Array.from(new Uint8Array(await f.arrayBuffer()))]); }}\
         if (input) input.value = '';\
         return out;"
    ))
    .join()
    .await?;
    files.sort();
    Ok(files)
}

#[derive(Clone, PartialEq)]
enum Status {
    Ok(String),
    Bad(String),
}

#[component]
pub fn TestCaseEditorPage() -> Element {
    let data = use_context::<AppDataHandle>();
    let editor = Editor::use_editor();
    let strokes = editor.strokes;

    let mut character = use_signal(String::new);
    let mut filename = use_signal(String::new);
    let mut show_hint = use_signal(|| false);
    let mut report = use_signal(|| None::<Report>);
    let mut candidate = use_signal(|| 0_usize);
    let mut custom_mapping = use_signal(String::new);
    let mut custom_draft = use_signal(String::new);
    let mut custom_result = use_signal(|| None::<Result<Candidate, String>>);
    let mut status = use_signal(|| None::<Status>);
    let mut folder = use_signal(Vec::<(String, Vec<u8>)>::new);
    let mut filter = use_signal(String::new);

    let target = use_memo(move || character().chars().next());

    let data_for_info = data.clone();
    let reference_count = use_memo(move || {
        target()
            .and_then(|c| data_for_info.kanji_map.get(&c))
            .map(|n| n.leaf_count())
    });

    let data_for_bg = data.clone();
    let background_svg = use_memo(move || {
        let mut svg = SVGBuilder::init().draw_grid(GRID_COLOR, CORNER_RADIUS);
        if show_hint()
            && let Some(c) = target()
            && let Some(node) = data_for_bg.kanji_map.get(&c)
        {
            svg = svg.draw_hint(&node.collect_paths(), HINT_COLOR);
        }
        svg.to_string()
    });
    let strokes_svg = use_memo(move || {
        let list = strokes();
        stroke_order_svg(&list, &drawn_order(list.len()), false)
    });

    let mut open_bytes = move |name: &str, bytes: &[u8]| match StrokeFile::decode(bytes) {
        Ok(file) => {
            character.set(file.character.to_string());
            filename.set(sanitize_filename(name));
            editor.replace(file.strokes);
            report.set(None);
            status.set(None);
        }
        Err(e) => status.set(Some(Status::Bad(format!(
            "{name} isn't a stroke file: {e}"
        )))),
    };

    let pick = move |input_id: &'static str| {
        spawn(async move {
            match read_picked_files(input_id).await {
                Ok(files) if files.is_empty() => {
                    status.set(Some(Status::Bad("No .bin files found there.".into())));
                }
                Ok(files) => {
                    if let [(name, bytes)] = files.as_slice() {
                        open_bytes(name, bytes);
                    } else {
                        status.set(Some(Status::Ok(format!("{} files loaded", files.len()))));
                    }
                    folder.set(files);
                    filter.set(String::new());
                }
                Err(e) => status.set(Some(Status::Bad(format!("Couldn't read the files: {e}")))),
            }
        });
    };

    let new_file = move |_| {
        editor.replace(Vec::new());
        character.set(String::new());
        filename.set(String::new());
        report.set(None);
        status.set(None);
    };

    let save_download = move |_| {
        let Some(c) = target() else {
            status.set(Some(Status::Bad("Set the target character first.".into())));
            return;
        };
        if strokes.peek().is_empty() {
            status.set(Some(Status::Bad("Nothing drawn yet.".into())));
            return;
        }
        let name = match sanitize_filename(&filename.peek()) {
            n if n.is_empty() => c.to_string(),
            n => n,
        };
        let file = StrokeFile {
            character: c,
            strokes: strokes.peek().clone(),
        };
        download(&name, &file.encode());
        status.set(Some(Status::Ok(format!("Downloaded {name}.bin"))));
    };

    let data_for_analyze = data.clone();
    let analyze = move |_| {
        let Some(c) = target() else {
            status.set(Some(Status::Bad("Set the target character first.".into())));
            return;
        };
        if strokes.peek().is_empty() {
            status.set(Some(Status::Bad("Nothing drawn yet.".into())));
            return;
        }
        status.set(None);
        candidate.set(0);
        let built = build_report(&data_for_analyze, c, &strokes.peek());
        let best = built
            .candidates
            .first()
            .map(|c| format_mapping(&c.order))
            .unwrap_or_default();
        custom_draft.set(best.clone());
        custom_mapping.set(best);
        custom_result.set(None);
        report.set(Some(built));
    };

    let filter_text = filter();
    let visible_files: Vec<(String, Vec<u8>)> = folder()
        .into_iter()
        .filter(|(name, _)| name.contains(filter_text.as_str()))
        .collect();
    let stroke_count = strokes().len();
    let is_stale = report().is_some_and(|r| r.drawn != strokes() || Some(r.character) != target());

    rsx! {
        div { class: "case-editor",
            div { class: "case-editor-head",
                h1 { "Test Case Editor" }
                p {
                    "Create and edit handwriting test cases: open them from a folder or draw a new one, and check how "
                    code { "match_strokes" }
                    " and the analyzer read it. Files are compatible with "
                    code { "kanji-draw" }
                    "."
                }
            }

            div { class: "case-editor-grid",
                section { class: "card case-editor-main",
                    div { class: "case-editor-files",
                        button { class: "btn", onclick: new_file, "New" }
                        label { class: "btn", r#for: FOLDER_INPUT_ID, "Open folder…" }
                        label { class: "btn", r#for: FILES_INPUT_ID, "Open files…" }
                        input {
                            id: FOLDER_INPUT_ID,
                            class: "visually-hidden",
                            r#type: "file",
                            "webkitdirectory": "true",
                            multiple: true,
                            onchange: move |_| pick(FOLDER_INPUT_ID),
                        }
                        input {
                            id: FILES_INPUT_ID,
                            class: "visually-hidden",
                            r#type: "file",
                            multiple: true,
                            onchange: move |_| pick(FILES_INPUT_ID),
                        }
                    }
                    if !folder().is_empty() {
                        div { class: "case-editor-library",
                            input {
                                class: "input",
                                r#type: "search",
                                placeholder: "Filter {folder().len()} files…",
                                value: "{filter}",
                                oninput: move |e| filter.set(e.value()),
                            }
                            div { class: "file-chips",
                                for (name , bytes) in visible_files {
                                    button {
                                        key: "{name}",
                                        class: if filename() == name { "file-chip active" } else { "file-chip" },
                                        onclick: {
                                            let name = name.clone();
                                            move |_| open_bytes(&name, &bytes)
                                        },
                                        "{name}"
                                    }
                                }
                            }
                        }
                    }

                    div { class: "target-row",
                        label { class: "field field-char",
                            span { "Character" }
                            input {
                                class: "input char-input",
                                placeholder: "雨",
                                value: "{character}",
                                oninput: move |e| {
                                    character.set(e.value().chars().next().map(String::from).unwrap_or_default());
                                },
                            }
                        }
                        label { class: "field field-name",
                            span { "File name" }
                            input {
                                class: "input",
                                placeholder: "e.g. 雨_wo1",
                                value: "{filename}",
                                oninput: move |e| filename.set(e.value()),
                            }
                        }
                    }
                    p { class: "target-info",
                        match (target(), reference_count()) {
                            (None, _) => rsx! { "Pick a character to analyze against." },
                            (Some(c), Some(n)) => rsx! { "{c} — {n} strokes in the reference · you drew {stroke_count}" },
                            (Some(c), None) => rsx! { span { class: "status-bad", "{c} isn't in the reference data" } },
                        }
                    }

                    EditorCanvas {
                        background_svg: background_svg(),
                        strokes_svg: strokes_svg(),
                        on_stroke: move |stroke: Stroke| editor.edit(|s| s.push(stroke)),
                    }

                    div { class: "toolbar",
                        button {
                            class: "btn",
                            disabled: editor.undo_stack.read().is_empty(),
                            onclick: move |_| editor.undo(),
                            "↶ Undo"
                        }
                        button {
                            class: "btn",
                            disabled: editor.redo_stack.read().is_empty(),
                            onclick: move |_| editor.redo(),
                            "↷ Redo"
                        }
                        button {
                            class: "btn",
                            disabled: stroke_count == 0,
                            onclick: move |_| editor.edit(Vec::clear),
                            "Clear"
                        }
                        button {
                            class: if show_hint() { "btn toggle on" } else { "btn toggle" },
                            onclick: move |_| show_hint.set(!show_hint()),
                            "Hint"
                        }
                    }

                    div { class: "toolbar",
                        button { class: "btn-primary btn-wide", onclick: analyze, "Analyze" }
                        button { class: "btn", onclick: save_download, "⤓ Download .bin" }
                    }

                    match status() {
                        Some(Status::Ok(msg)) => rsx! { p { class: "status-line status-ok", "{msg}" } },
                        Some(Status::Bad(msg)) => rsx! { p { class: "status-line status-bad", "{msg}" } },
                        None => rsx! {},
                    }
                }

                section { class: "card case-editor-results",
                    div { class: "section-head",
                        h2 { "Results" }
                        if is_stale {
                            span { class: "pill pill-warn", "Drawing changed — analyze again" }
                        }
                    }
                    match report() {
                        None => rsx! { p { class: "muted", "Press Analyze to see the matched stroke order and the analyzer's verdict." } },
                        Some(r) => rsx! { ReportView { report: r, candidate, custom_mapping, custom_draft, custom_result } },
                    }
                }
            }
        }
    }
}

/// The drawing surface. The committed strokes come in pre-rendered (`strokes_svg`); this only adds the stroke in progress on top. Sizes itself to its container (so it fills a phone screen) and normalizes pointer positions against its measured size, giving the same `0..1` space `kanji-draw` saves in.
#[component]
fn EditorCanvas(
    background_svg: String,
    strokes_svg: String,
    on_stroke: EventHandler<Stroke>,
) -> Element {
    let mut current = use_signal(Vec::<(f32, f32)>::new);
    // The one pointer currently drawing, so a second finger or a resting palm can't scribble into the same stroke.
    let mut active_pointer = use_signal(|| None::<i32>);
    let mut size = use_signal(|| (300.0_f64, 300.0_f64));

    let normalize = move |evt: &PointerEvent| {
        let p = evt.element_coordinates();
        let (w, h) = size();
        ((p.x / w) as f32, (p.y / h) as f32)
    };

    let start = move |evt: PointerEvent| {
        if active_pointer().is_some() {
            return;
        }
        evt.prevent_default();
        active_pointer.set(Some(evt.pointer_id()));
        current.set(vec![normalize(&evt)]);
    };
    let mv = move |evt: PointerEvent| {
        if active_pointer() != Some(evt.pointer_id()) {
            return;
        }
        evt.prevent_default();
        current.write().push(normalize(&evt));
    };
    let end = move |evt: PointerEvent| {
        if active_pointer() != Some(evt.pointer_id()) {
            return;
        }
        active_pointer.set(None);
        let stroke: Stroke = current.write().drain(..).collect();
        if stroke.len() > 1 {
            on_stroke.call(stroke);
        }
    };

    rsx! {
        div { class: "case-editor-canvas",
            div { class: "layer", dangerous_inner_html: "{background_svg}" }
            div { class: "layer", dangerous_inner_html: "{strokes_svg}" }
            svg {
                class: "layer case-editor-canvas-input",
                view_box: "0 0 {VIEWBOX} {VIEWBOX}",
                onresize: move |e| {
                    if let Ok(s) = e.get_border_box_size()
                        && s.width > 0.0
                        && s.height > 0.0
                    {
                        size.set((s.width, s.height));
                    }
                },
                onpointerdown: start,
                onpointermove: mv,
                onpointerup: end,
                onpointercancel: end,
                onpointerleave: end,
                path {
                    d: "{path_d(&current())}",
                    fill: "none",
                    stroke: INK_COLOR,
                    stroke_width: "3",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                }
            }
        }
    }
}

#[component]
fn ReportView(
    report: Report,
    candidate: Signal<usize>,
    /// The hand-written mapping currently in effect.
    custom_mapping: Signal<String>,
    /// What's typed in the custom row; only takes effect once applied.
    custom_draft: Signal<String>,
    /// The applied hand-written mapping, analyzed, or why it couldn't be.
    custom_result: Signal<Option<Result<Candidate, String>>>,
) -> Element {
    let data = use_context::<AppDataHandle>();
    // The hand-edited mapping sits after the matcher's candidates, at index `candidates.len()`.
    let custom_index = report.candidates.len();
    let pending = custom_draft() != custom_mapping();
    let report_for_apply = report.clone();
    let apply = use_callback(move |()| {
        let draft = custom_draft();
        let r = &report_for_apply;
        let result = parse_mapping(&draft, r.drawn.len(), r.reference_count)
            .map(|order| Candidate::new(&data, r.character, &r.drawn, order, None));
        custom_mapping.set(draft);
        custom_result.set(Some(result));
        candidate.set(custom_index);
    });
    let chosen = candidate().min(custom_index);
    let custom = custom_result();
    let selected: Option<Result<&Candidate, &String>> = match report.candidates.get(chosen) {
        Some(c) => Some(Ok(c)),
        None => custom.as_ref().map(Result::as_ref),
    };
    let verdict = match selected {
        _ if report.reference_count == 0 => Some(Verdict::NotInReference),
        Some(Ok(c)) => Some(c.verdict.clone()),
        _ => None,
    };

    rsx! {
        match &verdict {
            Some(Verdict::Correct) => rsx! { div { class: "verdict verdict-ok", "✓ Looks correct" } },
            Some(Verdict::NotInReference) => rsx! { div { class: "verdict verdict-bad", "{report.character} isn't in the reference data" } },
            Some(Verdict::Issue { label, .. }) => rsx! { div { class: "verdict verdict-bad", "⚠ {label}" } },
            None => rsx! {},
        }

        if report.reference_count > 0 {
            h3 { "match_strokes" }
            p { class: "muted",
                "Your strokes, numbered in the order the selected mapping assigns them to the reference."
            }
            match selected {
                Some(Ok(c)) => {
                    let assertion = rust_mapping(&c.order);
                    rsx! {
                        div { class: "case-editor-figure", dangerous_inner_html: "{c.svg}" }
                        div { class: "assertion",
                            code { "{assertion}" }
                            button {
                                class: "btn",
                                onclick: move |_| {
                                    document::eval(&format!("navigator.clipboard.writeText({assertion:?})"));
                                },
                                "Copy"
                            }
                        }
                    }
                }
                Some(Err(msg)) => rsx! { p { class: "status-bad", "{msg}" } },
                None => rsx! {},
            }
            div { class: "candidates",
                for (i , c) in report.candidates.iter().enumerate() {
                    button {
                        key: "{i}",
                        class: if i == chosen { "candidate-row active" } else { "candidate-row" },
                        onclick: move |_| candidate.set(i),
                        code { "{format_mapping(&c.order)}" }
                        if let Some(score) = c.score {
                            span { class: "muted", "{score:.4}" }
                        }
                    }
                }
                div { class: if chosen == custom_index { "candidate-row custom active" } else { "candidate-row custom" },
                    input {
                        class: "custom-mapping",
                        value: "{custom_draft}",
                        placeholder: "e.g. [1,2,2,3] · repeat = joined · - missing",
                        spellcheck: false,
                        oninput: move |e| custom_draft.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                apply.call(());
                            }
                        },
                    }
                    if pending {
                        button { class: "btn-primary btn-small", onclick: move |_| apply.call(()), "Apply" }
                    } else {
                        button {
                            class: "custom-select muted",
                            onclick: move |_| apply.call(()),
                            "custom"
                        }
                    }
                }
            }
        }

        if let Some(Verdict::Issue { correct, wrong, .. }) = &verdict {
            h3 { "Analyzer" }
            p { class: "muted", "For the selected mapping." }
            div { class: "result-svg-row",
                figure {
                    div { dangerous_inner_html: "{wrong}" }
                    figcaption { "What you drew" }
                }
                figure {
                    div { dangerous_inner_html: "{correct}" }
                    figcaption { "Correct" }
                }
            }
        }

        if !report.recognized.is_empty() {
            h3 { "Recognizer" }
            div { class: "candidate-list",
                for (c , score) in report.recognized.iter() {
                    div { class: if *c == report.character { "candidate hit" } else { "candidate" },
                        span { class: "char", "{c}" }
                        span { class: "score", "{score:.2}" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mapping_round_trips_format_mapping() {
        let order = vec![2, 0, 0, MISSING, 1];
        assert_eq!(parse_mapping(&format_mapping(&order), 3, 5), Ok(order));
    }

    #[test]
    fn parse_mapping_accepts_shorthand() {
        assert_eq!(
            parse_mapping("3 1 1 - 2", 3, 5),
            Ok(vec![2, 0, 0, MISSING, 1])
        );
    }

    #[test]
    fn parse_mapping_rejects_bad_input() {
        assert!(parse_mapping("[1,4]", 3, 2).is_err());
        assert!(parse_mapping("[0,1]", 3, 2).is_err());
        assert!(parse_mapping("[1,2]", 3, 3).is_err());
    }

    #[test]
    fn rust_mapping_names_missing() {
        assert_eq!(rust_mapping(&[0, 0, MISSING]), "[0, 0, MISSING]");
    }
}
