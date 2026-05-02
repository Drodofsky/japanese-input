mod utils;
use analyze::match_node::match_node;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '二');
    let user = load_test_file("二");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes, vec![0, 1]);
}

#[test]
fn wo() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '二');
    let user = load_test_file("二_wo");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes, vec![1, 0]);
}
