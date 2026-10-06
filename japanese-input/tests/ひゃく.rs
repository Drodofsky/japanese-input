mod utils;

use crate::utils::*;
use japanese_input::match_strokes::{MISSING, match_strokes as ms};
use japanese_input::stroke_point::ToStrokeVector as _;
use japanese_input::weights::Weights;

#[test]
fn wc1() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '百');
    let user = load_test_file("百_wc1");
    let result = ms(reference, user.to_stroke_vector(), Weights::default(), 64);
    assert_eq!(
        result[0].user_stroke_order.as_slice(),
        vec![0, 0, 0, 1, 2, 3]
    );
}
#[test]
fn wc2() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '百');
    let user = load_test_file("百_wc2");
    let result = match_strokes(reference, &user);
    assert_eq!(
        result[0].user_stroke_order.as_slice(),
        vec![0, 1, 1, 2, 3, 4]
    );
}
