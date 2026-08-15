use criterion::{Criterion, criterion_group, criterion_main};
use japanese_input::recognizer::Recognizer;

mod utils;

use crate::utils::{load_recognizer_model, load_test_file};

/// One test file per stroke count found in `data/test` — 7 and 10 strokes have no
/// fixture on disk, so the cycle skips straight from 6 to 8, then closes with the
/// highest stroke count available, `語_m1` at 13.
const STROKE_SAMPLES: [&str; 9] = ["一", "こ", "あ", "円", "右", "ぎ", "雨", "音", "語_m1"];

fn bench_recognize(c: &mut Criterion) {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).expect("load recognizer model");
    let corpus: Vec<Vec<Vec<(f32, f32)>>> = STROKE_SAMPLES
        .iter()
        .map(|&name| load_test_file(name))
        .collect();

    let mut next = 0_usize;
    c.bench_function(
        "recognize (one drawing per round, cycling stroke counts 1-13)",
        |b| {
            b.iter(|| {
                let strokes = &corpus[next % corpus.len()];
                next = next.wrapping_add(1);
                recognizer.recognize(strokes)
            });
        },
    );
}

criterion_group!(benches, bench_recognize);
criterion_main!(benches);
