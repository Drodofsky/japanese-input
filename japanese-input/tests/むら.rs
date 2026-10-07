mod utils;

use crate::utils::*;

#[test]
fn wp() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '村');
    let user = load_test_file("村_wp");

    let result = match_strokes(reference, &user);

    assert_eq!(
        result[0].user_stroke_order.as_slice(),
        vec![0, 1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn wp_wo() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '村');
    let user = load_test_file("村_wp_wo");

    let result = match_strokes(reference, &user);

    assert_eq!(
        result[0].user_stroke_order.as_slice(),
        vec![3, 4, 5, 6, 0, 1, 2]
    );
}
