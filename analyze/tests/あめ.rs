mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '雨');
    let user = load_test_file("雨");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}
