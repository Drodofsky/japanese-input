use dioxus::prelude::*;
use japanese_input::stroke_point::ToStrokeVector as _;

use crate::components::MultiCharCanvas;
use crate::data::AppDataHandle;
use crate::outcome::Outcome;
use crate::recognize_answer::compare_with_target;
use crate::vocab_data::{KAKI_ENTRIES, VocabEntry, yomi_entries};

const GRID_COLOR: &str = "var(--border)";
const STROKE_COLOR: &str = "var(--ink)";
const CORNER_RADIUS: f32 = 6.0;

// Same marks the addon's own `ReviewWidget.set_analyses` draws when every kanji in the answer comes back with no stroke-order errors: the hanamaru (flower circle) if the recognized answer also matched exactly, or the triangle if it didn't — copied verbatim from `japanese_input_anki/widgets.py`.
const HANAMARU_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="340" height="340" viewBox="0 0 340 340">
  <g transform="translate(0.000000,340.000000) scale(0.100000,-0.100000)" fill-rule="evenodd">
    <path d="M1137 3008 c-126 -16 -223 -65 -310 -158 -81 -85 -113 -142 -139
-242 -20 -82 -21 -96 -5 -293 2 -27 -1 -30 -43 -37 -217 -37 -354 -316 -261
-532 30 -68 103 -169 151 -207 l39 -32 -53 -57 c-108 -118 -135 -246 -77 -365
36 -72 105 -140 179 -177 72 -35 224 -34 303 3 92 43 139 92 139 145 0 33 -5
46 -22 58 -34 24 -59 20 -140 -21 -101 -52 -172 -53 -239 -5 -85 62 -99 137
-39 216 37 49 116 106 146 106 41 0 79 43 79 88 0 48 -24 78 -69 88 -71 15
-182 120 -224 213 -56 121 -5 273 101 302 37 10 43 8 122 -40 88 -54 110 -59
154 -36 58 29 55 67 -16 206 l-58 113 0 120 c0 112 2 125 27 171 106 199 370
267 521 135 55 -48 81 -100 102 -200 18 -84 35 -108 86 -116 43 -7 84 23 95
69 25 117 199 237 344 237 180 -1 355 -253 256 -371 -74 -88 17 -189 122 -135
145 73 410 -38 491 -207 65 -135 10 -228 -137 -231 -57 -1 -67 -5 -88 -29 -48
-56 -24 -138 43 -152 76 -16 105 -75 111 -226 15 -374 -222 -645 -471 -539
l-59 24 -36 -19 c-48 -26 -60 -60 -43 -124 16 -63 0 -100 -70 -166 -82 -76
-210 -116 -264 -81 -22 14 -25 24 -25 74 0 50 -4 62 -25 82 -52 49 -145 14
-145 -54 0 -72 -246 -113 -389 -65 -119 40 -164 122 -126 227 29 80 30 106 3
135 -86 95 -189 -15 -188 -202 1 -166 91 -277 269 -332 125 -38 324 -32 436
15 33 14 36 13 71 -15 85 -67 249 -67 391 2 73 36 184 151 221 230 l32 66 84
5 c204 11 384 170 465 414 58 173 64 366 15 512 l-24 73 58 54 c67 62 92 120
92 215 0 252 -239 479 -534 508 l-70 7 -12 74 c-24 141 -128 286 -255 356
-157 85 -371 57 -541 -72 l-57 -43 -18 26 c-115 167 -273 237 -476 212z" fill="#e6331b"/>
    <path d="M1635 2361 c-312 -50 -596 -281 -651 -529 -59 -263 70 -562 292 -679
249 -132 568 -36 675 202 60 135 15 284 -128 421 -102 98 -241 113 -387 41
-151 -75 -204 -174 -152 -282 66 -133 221 -184 354 -116 97 50 85 157 -16 139
-131 -23 -130 -23 -167 12 -41 40 -36 57 31 96 31 18 64 28 107 31 76 5 103
-9 166 -91 84 -108 59 -216 -68 -293 -232 -140 -501 24 -543 330 -18 137 12
226 116 336 244 258 614 283 841 56 122 -121 183 -243 192 -382 7 -99 -5 -151
-57 -256 -151 -302 -474 -510 -729 -468 -57 9 -69 15 -121 66 -66 62 -106 71
-154 34 -47 -37 -37 -88 31 -160 92 -97 138 -114 308 -113 131 0 146 2 227 31
333 119 621 445 668 757 49 326 -210 701 -550 795 -83 23 -214 33 -285 22z" fill="#e6331b"/>
  </g>
</svg>"##;

const SANKAKU_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="340" height="340" viewBox="0 0 340 340">
  <path d="M170 60 L290 280 L50 280 Z" fill="none" stroke="#e6331b" stroke-width="14" stroke-linejoin="round" stroke-linecap="round"/>
</svg>"##;

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Yomi,
    Kaki,
}

impl Direction {
    fn entries(self) -> Vec<VocabEntry> {
        match self {
            Direction::Yomi => yomi_entries(),
            Direction::Kaki => KAKI_ENTRIES.to_vec(),
        }
    }

    fn levels(self) -> &'static [u8] {
        match self {
            Direction::Yomi => &[10, 9, 8],
            Direction::Kaki => &[10],
        }
    }

    /// What the user has to write and get recognized: the word itself for "kaki" (書き), its reading for "yomi" (読み) — matches whichever field the addon's own `{{type:...}}` note template would point at.
    fn target(self, entry: &VocabEntry) -> &str {
        match self {
            Direction::Yomi => entry.reading,
            Direction::Kaki => entry.word,
        }
    }

    /// What's shown highlighted in the sentence: the reading in kaki mode (since the kanji itself is what you're writing), the word in yomi mode (since the reading is what you're writing).
    fn shown_in_sentence<'a>(self, entry: &'a VocabEntry) -> &'a str {
        match self {
            Direction::Yomi => entry.word,
            Direction::Kaki => entry.reading,
        }
    }
}

/// Same kana ranges as the addon's own `_is_kana` in `japanese_input_anki/__init__.py` — only non-kana (kanji) characters get stroke-order analysis.
fn is_kana(ch: char) -> bool {
    ('ぁ'..='ん').contains(&ch)
        || ('ァ'..='ヶ').contains(&ch)
        || ('ｦ'..='ﾝ').contains(&ch)
        || matches!(ch, 'ー' | 'ｰ' | '・')
}

fn reset_card_state(
    mut revealed: Signal<bool>,
    mut commits: Signal<Vec<Vec<Vec<(f32, f32)>>>>,
    mut current: Signal<Vec<Vec<(f32, f32)>>>,
    mut recognized: Signal<Option<String>>,
    mut analyses: Signal<Vec<(char, Outcome)>>,
) {
    revealed.set(false);
    commits.write().clear();
    current.write().clear();
    recognized.set(None);
    analyses.write().clear();
}

fn sentence_view(entry: &VocabEntry, shown: &str) -> Element {
    let Some((before, after)) = entry.sentence.split_once(entry.word) else {
        return rsx! {
            p { class: "sentence", "{entry.sentence}" }
        };
    };
    rsx! {
        p { class: "sentence",
            "{before}"
            span { class: "target-word", "{shown}" }
            "{after}"
        }
    }
}

/// Renders the recognized string against the correct target character by character, like Anki's own typed-answer diff highlighting.
fn compare_view(target: &str, recognized: &str) -> Element {
    let target_chars: Vec<char> = target.chars().collect();
    let recognized_chars: Vec<char> = recognized.chars().collect();
    let len = target_chars.len().max(recognized_chars.len());
    let pairs: Vec<(char, bool)> = (0..len)
        .map(|i| {
            let t = target_chars.get(i).copied();
            let r = recognized_chars.get(i).copied();
            let ok = t.is_some() && t == r;
            (r.unwrap_or('-'), ok)
        })
        .collect();
    rsx! {
        p { class: "compare-row",
            for (ch , ok) in pairs {
                span { class: if ok { "compare-char-ok" } else { "compare-char-bad" }, "{ch}" }
            }
        }
    }
}

#[component]
pub fn ReviewPage() -> Element {
    let data = use_context::<AppDataHandle>();
    let mut direction = use_signal(|| Direction::Yomi);
    let mut level = use_signal(|| 10_u8);
    let mut index = use_signal(|| 0_usize);
    let mut revealed = use_signal(|| false);
    let mut commits = use_signal(Vec::<Vec<Vec<(f32, f32)>>>::new);
    let mut current = use_signal(Vec::<Vec<(f32, f32)>>::new);
    let mut recognized = use_signal(|| None::<String>);
    let mut analyses = use_signal(Vec::<(char, Outcome)>::new);

    let on_direction_change = move |evt: FormEvent| {
        let new_dir = if evt.value() == "kaki" {
            Direction::Kaki
        } else {
            Direction::Yomi
        };
        direction.set(new_dir);
        level.set(new_dir.levels()[0]);
        index.set(0);
        reset_card_state(revealed, commits, current, recognized, analyses);
    };

    let on_level_change = move |evt: FormEvent| {
        if let Ok(lv) = evt.value().parse::<u8>() {
            level.set(lv);
            index.set(0);
            reset_card_state(revealed, commits, current, recognized, analyses);
        }
    };

    // A single persistent button drives both halves, like Anki's own "Show Answer": while on the front it checks (reveals the back), while on the back it advances to the next card — no separate Check/Next Card buttons.
    let data_for_check = data.clone();
    let show_answer = move |_| {
        if revealed() {
            let n = direction()
                .entries()
                .iter()
                .filter(|e| e.kanken_level == level())
                .count();
            if n > 0 {
                index.set((index() + 1) % n);
            }
            reset_card_state(revealed, commits, current, recognized, analyses);
            return;
        }

        let entries = direction().entries();
        let cards: Vec<&VocabEntry> = entries
            .iter()
            .filter(|e| e.kanken_level == level())
            .collect();
        let Some(entry) = cards.get(index()).copied() else {
            return;
        };
        let target = direction().target(entry);

        // Mirrors `InputWidget.auto_commit_pending`: whatever's still on the canvas gets committed first, so checking doesn't silently drop it.
        let pending: Vec<Vec<(f32, f32)>> = current.write().drain(..).collect();
        if !pending.is_empty() {
            commits.write().push(pending);
        }
        let user_commits = commits();

        recognized.set(
            data_for_check
                .recognizer
                .as_ref()
                .map(|r| compare_with_target(r, &user_commits, target)),
        );

        let mut results = Vec::new();
        for (i, ch) in target.chars().enumerate() {
            if is_kana(ch) {
                continue;
            }
            let Some(strokes) = user_commits.get(i) else {
                continue;
            };
            let result = data_for_check.analyzer.analyze_kanji(
                ch,
                strokes.to_stroke_vector(),
                GRID_COLOR,
                CORNER_RADIUS,
                STROKE_COLOR,
            );
            results.push((ch, Outcome::from_analysis(result)));
        }
        analyses.set(results);
        revealed.set(true);
    };

    let render_entries = direction().entries();
    let cards: Vec<&VocabEntry> = render_entries
        .iter()
        .filter(|e| e.kanken_level == level())
        .collect();
    let card = cards.get(index()).copied();

    rsx! {
        div { class: "card",
            h1 { "Vocab Review" }
            p { "A feel for how the add-on could fit into a review workflow." }
            div { class: "review-controls",
                div { class: "field",
                    label { r#for: "direction-select", "Mode" }
                    select {
                        id: "direction-select",
                        onchange: on_direction_change,
                        option { value: "yomi", "読み (reading)" }
                        option { value: "kaki", "書き (writing)" }
                    }
                }
                div { class: "field",
                    label { r#for: "level-select", "Kanken level" }
                    select { id: "level-select", onchange: on_level_change,
                        for lv in direction().levels() {
                            option { value: "{lv}", "Level {lv}" }
                        }
                    }
                }
            }

            match card {
                None => rsx! {
                    div { class: "notice", "No sample cards at this level yet." }
                },
                Some(entry) => {
                    let target = direction().target(entry);
                    rsx! {
                        div { class: "flashcard",
                            {sentence_view(entry, direction().shown_in_sentence(entry))}

                            if !revealed() {
                                MultiCharCanvas { commits, current, target: target.chars().collect::<Vec<char>>() }
                            } else {
                                p { class: "reading", "Meaning: {entry.meaning}" }
                                p { class: "sentence-translation", "{entry.translation}" }
                                match recognized() {
                                    None => rsx! {
                                        p { class: "notice", "Recognition isn't available in this build." }
                                        p { class: "reading", "Correct: {target}" }
                                    },
                                    Some(rec) if rec == target => rsx! {
                                        {compare_view(target, &rec)}
                                    },
                                    Some(rec) => rsx! {
                                        {compare_view(target, &rec)}
                                        p { class: "reading", "Correct: {target}" }
                                    },
                                }

                                if !analyses().is_empty() && analyses().iter().all(|(_, o)| *o == Outcome::Correct) {
                                    div {
                                        class: "result-mark",
                                        dangerous_inner_html: if recognized().as_deref() == Some(target) { HANAMARU_SVG } else { SANKAKU_SVG },
                                    }
                                } else {
                                    for (_ , o) in analyses().into_iter().filter(|(_, o)| *o != Outcome::Correct) {
                                        {crate::outcome::view(&o)}
                                    }
                                }
                            }

                            div { class: "stroke-canvas-actions",
                                button {
                                    class: "btn",
                                    onclick: show_answer,
                                    if revealed() { "Next Card" } else { "Show Answer" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
