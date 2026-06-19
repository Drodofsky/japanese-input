mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '右');
    let user = load_test_file("右");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![0, 1, 2, 3, 4]);
}

#[test]
fn wo1() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '右');
    let user = load_test_file("右_wo1");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![1, 0, 2, 3, 4]);
}
