mod utils;

use crate::utils::*;

#[test]
fn wc1() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '百');
    let user = load_test_file("百_wc1");
    let result = match_strokes(&reference, &user);
    assert_eq!(result.as_slice(), vec![0, 0, 0, 1, 2, 3]);
}

#[test]
fn wc2() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '百');
    let user = load_test_file("百_wc2");
    let result = match_strokes(&reference, &user);
    assert_eq!(result.as_slice(), vec![0, 1, 1, 2, 3, 4]);
}
