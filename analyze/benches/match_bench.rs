use criterion::{Criterion, criterion_group, criterion_main};

mod utils;

use crate::utils::*;
fn bench_match(c: &mut Criterion) {
    let map = load_kanji_map();
    let reference = analyzed(&map, '語');
    let user = load_test_file("語_m1");

    c.bench_function("match_node 語", |b| {
        b.iter(|| match_node(&reference, &user))
    });
}

criterion_group!(benches, bench_match);
criterion_main!(benches);
