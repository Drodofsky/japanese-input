mod utils;
use analyze::match_node::match_node;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '右');
    let user = load_test_file("右");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes, vec![0, 1, 2, 3, 4]);
}
#[test]
fn wo1() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '右');
    let user = load_test_file("右_wo1");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes, vec![1, 0, 2, 3, 4]);
}
