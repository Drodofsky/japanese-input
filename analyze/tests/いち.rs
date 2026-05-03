mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '一');
    let user = load_test_file("一");

    let result = match_node(&reference, &user);

    assert_eq!(result.len(), 1, "should find exactly one match: {result:?}");
    assert_eq!(result[0].user_strokes.as_slice(), vec![0]);
}
