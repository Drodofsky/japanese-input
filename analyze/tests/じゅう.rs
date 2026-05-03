mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '十');
    let user = load_test_file("十");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![0, 1]);
}
#[test]
fn wo1() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '十');
    let user = load_test_file("十_wo1");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![1, 0]);
}
