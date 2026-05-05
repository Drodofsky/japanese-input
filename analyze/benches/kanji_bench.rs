use analyze::recognize_kanji::KanjiRecognizer;
use criterion::{Criterion, criterion_group, criterion_main};

mod utils;

use crate::utils::*;
fn bench_match(c: &mut Criterion) {
    let map = load_kanji_map();
    let user = load_test_file("雨");
    let recognizer = KanjiRecognizer::new(&map);

    c.bench_function("kanji_recognizer 雨", |b| {
        b.iter(|| recognizer.recognize(&user))
    });
}

criterion_group!(benches, bench_match);
criterion_main!(benches);
