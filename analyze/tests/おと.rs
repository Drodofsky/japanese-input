mod utils;
use analyze::match_node::match_node;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '音');
    let user = load_test_file("音");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
}
#[test]
fn wo1() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '音');
    let user = load_test_file("音_wo1");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes, vec![4, 5, 6, 7, 8, 0, 1, 2, 3]);
}
#[test]
fn wp() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '音');
    let user = load_test_file("音_wp");
    let result = match_node(&reference, &user);
    assert_eq!(result[0].user_strokes, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
}
