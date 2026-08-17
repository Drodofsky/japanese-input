mod utils;

use crate::utils::*;
use japanese_input::match_strokes::MISSING;

#[test]
fn wc1() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '百');
    let user = load_test_file("百_wc1");
    let result = match_strokes(reference, &user);
    assert_eq!(
        result[0].user_stroke_order.as_slice(),
        vec![MISSING, MISSING, MISSING, 1, 2, 3]
    );
}
