mod utils;

use crate::utils::*;

#[test]
fn correct() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![0, 1, 2]);
}

#[test]
fn wo1() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三_wo1");
    let result = match_node(&reference, &user);
    assert_eq!(result[0].user_strokes.as_slice(), vec![2, 0, 1]);
}

#[test]
fn wo2() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三_wo2");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![2, 1, 0]);
}
#[test]
fn wo3() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三_wo3");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![0, 2, 1]);
}
#[test]
fn p1() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三_p1");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![0, 1, 2]);
}
#[test]
fn p1_wo() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三_p1_wo");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![0, 3, 2]);
}
#[test]
fn p2() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三_p2");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![0, 2, 3]);
}
#[test]
fn m1() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三_m1");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![0, u8::MAX, 1]);
}
#[test]
fn m2() {
    let map = load_kanji_map();
    let reference = analyzed(&map, '三');
    let user = load_test_file("三_m2");

    let result = match_node(&reference, &user);

    assert_eq!(result[0].user_strokes.as_slice(), vec![u8::MAX, u8::MAX, 0]);
}
