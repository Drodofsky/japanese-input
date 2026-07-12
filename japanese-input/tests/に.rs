mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '二');
    let user = load_test_file("二");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![0, 1]);
}

#[test]
fn wo() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '二');
    let user = load_test_file("二_wo");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![1, 0]);
}
