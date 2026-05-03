mod utils;

use crate::utils::*;
#[test]
fn m1() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '語');
    let user = load_test_file("語_m1");

    let result = match_node(&reference, &user);

    assert_eq!(
        result[0].user_strokes,
        vec![usize::MAX, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    );
}
#[ignore = "reason"]
#[test]
fn profile_match() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '語');
    let user = load_test_file("語_m1");

    for _ in 0..100 {
        let _ = match_node(&reference, &user);
    }
}
