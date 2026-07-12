mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '土');
    let user = load_test_file("土");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![0, 1, 2]);
}

#[test]
fn m1() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '土');
    let user = load_test_file("土_m1");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![0, u8::MAX, 1]);
}
