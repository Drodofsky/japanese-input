mod utils;

use crate::utils::*;
#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '生');
    let user = load_test_file("生");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![0, 1, 2, 3, 4]);
}
