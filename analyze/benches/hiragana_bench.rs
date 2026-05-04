use analyze::recognize_hiragana::HiraganaRecognizer;
use criterion::{Criterion, criterion_group, criterion_main};

mod utils;

use crate::utils::*;
fn bench_match(c: &mut Criterion) {
    let map = load_kanji_map();
    let user = load_test_file("あ");
    let recognizer = HiraganaRecognizer::new(&map);

    c.bench_function("hiragana_recognizer あ", |b| {
        b.iter(|| recognizer.recognize(&user))
    });
}

criterion_group!(benches, bench_match);
criterion_main!(benches);
