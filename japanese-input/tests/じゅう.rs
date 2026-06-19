mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '十');
    let user = load_test_file("十");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![0, 1]);
}

#[test]
fn wo1() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '十');
    let user = load_test_file("十_wo1");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![1, 0]);
}
