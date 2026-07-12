mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '生');
    let user = load_test_file("生");

    let result = match_strokes(reference, &user);

    assert_eq!(result[0].user_stroke_order.as_slice(), vec![0, 1, 2, 3, 4]);
}
