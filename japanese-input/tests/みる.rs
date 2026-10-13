mod utils;

use crate::utils::*;

#[test]
fn wc1() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '見');
    let user = load_test_file("見_wc1");
    let result = match_strokes(&reference, &user);
    assert_eq!(result.as_slice(), vec![0, 1, 2, 3, 4, 4, 5]);
}
