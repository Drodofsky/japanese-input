use criterion::{Criterion, criterion_group, criterion_main};
use japanese_input::{recognize_hiragana::HiraganaRecognizer, stroke_point::ToStrokeVector};

mod utils;

use crate::utils::{load_hiragana_map, load_test_file};

fn bench_match(c: &mut Criterion) {
    let map = load_hiragana_map();
    let user = load_test_file("あ");
    let recognizer = HiraganaRecognizer::new(&map);

    c.bench_function("hiragana_recognizer あ", |b| {
        b.iter(|| recognizer.recognize(user.to_stroke_vector()));
    });
}

criterion_group!(benches, bench_match);
criterion_main!(benches);
