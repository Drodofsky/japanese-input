//! Small, hand-curated sample vocabulary for the "Vocab Review" demo page.
//!
//! This is illustrative content only, not a real Kanken deck — the `japanese-input` addon repo has no such dataset (only accuracy-testing CSVs broken out by Kanken level), so this exists purely to show the deck-direction x level review UX with a handful of real examples.
//!
//! Every sentence here is taken essentially verbatim from the Japan Kanji Aptitude Testing Foundation's own official sample tests for levels 10/9/8 (kanken.or.jp), rather than invented — that's what keeps the grammar, register, and vocabulary genuinely typical of each level instead of reading like a textbook example. Kanji usage also respects the cumulative grade for each level (10 = grade-1 kyōiku kanji only, per this project's own `japanese-input/stats/matcher_kanken_10.csv`; 9 adds grade 2; 8 adds grade 3), which is exactly what the official sentences do too.
//!
//! Real Kanken reading passages usually underline *several* words in the same sentence as separate questions, not just one — so for "yomi" (reading) each [`SentenceSource`] lists every kanji word worth testing in it, and [`yomi_entries`] expands that into one [`VocabEntry`] card per word. For a kanji+hiragana word (okurigana, or a hiragana particle/prefix — 花だん, 教えて, ふる里), the listed word/reading is trimmed to just the kanji character and *its* reading in that context, matching how Kanken only ever underlines the kanji itself. Pure kanji compounds with no separating hiragana (熟語, e.g. 研究) are tested as the whole compound instead, same as real exams.
//!
//! "Kaki" (writing) stays one card per sentence, deliberately *not* exploded the same way: in that mode the front swaps only the current target's reading into the sentence, so any other kanji from the same sentence would still sit there fully written — splitting one sentence into several writing cards would mean later cards give away characters from an earlier one in the same sequence.

#[derive(Clone, Copy)]
pub struct VocabEntry {
    pub kanken_level: u8,
    pub word: &'static str,
    pub reading: &'static str,
    pub meaning: &'static str,
    pub sentence: &'static str,
    pub translation: &'static str,
}

pub struct WordEntry {
    pub word: &'static str,
    pub reading: &'static str,
    pub meaning: &'static str,
}

pub struct SentenceSource {
    pub kanken_level: u8,
    pub sentence: &'static str,
    pub translation: &'static str,
    pub words: &'static [WordEntry],
}

fn expand(sources: &'static [SentenceSource]) -> Vec<VocabEntry> {
    sources
        .iter()
        .flat_map(|s| {
            s.words.iter().map(|w| VocabEntry {
                kanken_level: s.kanken_level,
                word: w.word,
                reading: w.reading,
                meaning: w.meaning,
                sentence: s.sentence,
                translation: s.translation,
            })
        })
        .collect()
}

/// Writing practice (書き, "kaki"): Kanken level 10 only, one card per sentence (see module docs for why), each target a pure kanji chunk so it can be graded with `Analyzer::analyze_kanji`.
pub const KAKI_ENTRIES: &[VocabEntry] = &[
    VocabEntry {
        kanken_level: 10,
        word: "一",
        reading: "ひと",
        meaning: "one",
        sentence: "あめ玉を一つ口に入れる。",
        translation: "Put one piece of candy in your mouth.",
    },
    VocabEntry {
        kanken_level: 10,
        word: "空",
        reading: "そら",
        meaning: "sky",
        sentence: "夕がた、にしの空が赤くそまっていた。",
        translation: "In the evening, the western sky had turned red.",
    },
    VocabEntry {
        kanken_level: 10,
        word: "天気",
        reading: "てんき",
        meaning: "weather",
        sentence: "天気がよいので、犬をつれてちかくの林をさんぽした。",
        translation: "Since the weather was nice, I took the dog for a walk in the nearby forest.",
    },
    VocabEntry {
        kanken_level: 10,
        word: "木",
        reading: "もく",
        meaning: "Thursday (木よう日)",
        sentence: "木よう日にピアノをならう。",
        translation: "I take piano lessons on Thursdays.",
    },
    VocabEntry {
        kanken_level: 10,
        word: "円",
        reading: "まる",
        meaning: "round",
        sentence: "へやに円いテーブルをおく。",
        translation: "Put a round table in the room.",
    },
];

/// Reading practice: Kanken levels 10 (grade 1), 9 (adds grade 2), and 8 (adds grade 3).
const YOMI_SENTENCES: &[SentenceSource] = &[
    SentenceSource {
        kanken_level: 10,
        sentence: "花だんのざっ草をぬく。",
        translation: "Pull the weeds from the flower bed.",
        words: &[
            WordEntry { word: "花", reading: "か", meaning: "flower (花だん, flower bed)" },
            WordEntry { word: "草", reading: "そう", meaning: "weed, grass (雑草)" },
        ],
    },
    SentenceSource {
        kanken_level: 10,
        sentence: "おかあさんに青いけ糸で手ぶくろをあんでもらう。",
        translation: "Have Mom knit gloves with blue yarn.",
        words: &[
            WordEntry { word: "青", reading: "あお", meaning: "blue" },
            WordEntry { word: "手", reading: "て", meaning: "hand (手ぶくろ, gloves)" },
        ],
    },
    SentenceSource {
        kanken_level: 9,
        sentence: "今日、母のふる里からおばあさんが来る。",
        translation: "Today, Grandma is coming from Mom's hometown.",
        words: &[
            WordEntry { word: "今日", reading: "きょう", meaning: "today" },
            WordEntry { word: "母", reading: "はは", meaning: "mother" },
            WordEntry { word: "里", reading: "さと", meaning: "hometown (ふるさと)" },
            WordEntry { word: "来", reading: "く", meaning: "to come (来る)" },
        ],
    },
    SentenceSource {
        kanken_level: 9,
        sentence: "算数のテストでまちがえたところをお姉さんに教えてもらった。",
        translation: "I had my older sister teach me the part I got wrong on the math test.",
        words: &[
            WordEntry { word: "算数", reading: "さんすう", meaning: "arithmetic, math" },
            WordEntry { word: "姉", reading: "ねえ", meaning: "older sister (お姉さん)" },
            WordEntry { word: "教", reading: "おし", meaning: "to teach (教える)" },
        ],
    },
    SentenceSource {
        kanken_level: 8,
        sentence: "森にいる生き物について研究する。",
        translation: "Research the creatures living in the forest.",
        words: &[
            WordEntry { word: "森", reading: "もり", meaning: "forest" },
            WordEntry { word: "生", reading: "い", meaning: "life, living (生き物)" },
            WordEntry { word: "物", reading: "もの", meaning: "thing (生き物)" },
            WordEntry { word: "研究", reading: "けんきゅう", meaning: "research" },
        ],
    },
    SentenceSource {
        kanken_level: 8,
        sentence: "橋の上から深い谷をのぞく。",
        translation: "Peek into the deep valley from the bridge.",
        words: &[
            WordEntry { word: "橋", reading: "はし", meaning: "bridge" },
            WordEntry { word: "上", reading: "うえ", meaning: "top, above" },
            WordEntry { word: "深", reading: "ふか", meaning: "deep (深い)" },
            WordEntry { word: "谷", reading: "たに", meaning: "valley" },
        ],
    },
];

pub fn yomi_entries() -> Vec<VocabEntry> {
    expand(YOMI_SENTENCES)
}
