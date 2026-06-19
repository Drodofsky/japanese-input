use criterion::{Criterion, criterion_group, criterion_main};
use japanese_input::{
    match_strokes::{Weights, match_strokes},
    stroke_point::ToStrokeVector,
};

mod utils;

use crate::utils::{load_kanji_map, load_test_file};

fn bench_match(c: &mut Criterion) {
    let map = load_kanji_map();
    let reference = map.get(&'語').cloned().unwrap().to_analyzed();
    let user = load_test_file("語_m1");

    c.bench_function("match_node 語", |b| {
        b.iter(|| {
            match_strokes(
                reference.clone(),
                user.to_stroke_vector(),
                Weights::default(),
                100,
            )
        });
    });
}

criterion_group!(benches, bench_match);
criterion_main!(benches);
