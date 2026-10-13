mod utils;

use crate::utils::*;

/// Connected *and* out of order: the short upper horizontal was written first, and the vertical
/// then ran straight into the bottom horizontal as one motion. So two drawn strokes cover three
/// reference strokes, and drawn stroke 1 covers reference strokes 0 and 2 — a join whose two
/// halves are not adjacent, with reference stroke 1 assigned in between.
#[test]
fn wc_wo() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '上');
    let user = load_test_file("上_wc_wo");
    let result = match_strokes(&reference, &user);
    assert_eq!(result.as_slice(), vec![1, 0, 1]);
}
