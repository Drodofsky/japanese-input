//! Mirrors the addon's own `Recognizer.compare_with_target` (the pyo3 bindings in `japanese-input-py/src/lib.rs`, which the addon calls on every "show answer" keypress to fill in Anki's native typed-answer field): recognize each committed character, then nudge known-confusable pairs toward the expected answer before falling back to the raw top guess. That confusable-pair table lives only in the pyo3 crate (not the core `japanese-input` library this site depends on), so it's copied here verbatim to keep behavior consistent with the real addon.

use japanese_input::recognizer::Recognizer;

const CONFUSABLE_GROUPS: [&str; 13] = [
    "へヘ", "べベ", "ぺペ", "イ亻", "エ工", "カ力", "タ夕", "ト卜", "ニ二", "ネ礻", "ム厶", "ロ口",
    "ー一",
];

fn correct_char(expected_char: char, drawn_chars: [char; 2]) -> char {
    for drawn_char in drawn_chars {
        if drawn_char == expected_char {
            return drawn_char;
        }
        for group in CONFUSABLE_GROUPS {
            if group.contains(expected_char) && group.contains(drawn_char) {
                return expected_char;
            }
        }
    }
    drawn_chars[0]
}

/// `committed` is one entry per drawn character, each a list of strokes, each a list of `(x, y)` points — same shape the addon collects from its numbered input slots.
pub fn compare_with_target(
    recognizer: &Recognizer,
    committed: &[Vec<Vec<(f32, f32)>>],
    target: &str,
) -> String {
    let recognized: Vec<[char; 2]> = committed
        .iter()
        .map(|strokes| {
            let results = recognizer.recognize(strokes);
            [
                results.first().map_or('-', |r| r.character),
                results.get(1).map_or('-', |r| r.character),
            ]
        })
        .collect();

    let mut out: String = recognized
        .iter()
        .zip(target.chars())
        .map(|(d, e)| correct_char(e, *d))
        .collect();
    for d in recognized.iter().skip(out.chars().count()) {
        out.push(d[0]);
    }
    out
}
