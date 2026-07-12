use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use japanese_input::{recognize_kanji::KanjiRecognizer, stroke_point::ToStrokeVector};

mod utils;

use crate::utils::{load_kanji_map, load_test_file};

fn bench_match(c: &mut Criterion) {
    let map = load_kanji_map();
    let user = load_test_file("雨");
    let recognizer = KanjiRecognizer::new(&map);

    c.bench_function("kanji_recognizer 雨", |b| {
        b.iter(|| black_box(recognizer.recognize(user.to_stroke_vector())));
    });
}

criterion_group!(benches, bench_match);
criterion_main!(benches);
