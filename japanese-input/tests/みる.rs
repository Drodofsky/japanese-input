mod utils;

use crate::utils::*;
use japanese_input::match_strokes::{MISSING, match_strokes};
use japanese_input::stroke_point::ToStrokeVector as _;
use japanese_input::weights::Weights;

#[test]
fn wc1() {
    let map = load_kanji_map();
    let reference = load_kanji_node(&map, '見');
    let user = load_test_file("見_wc1");
    // The shared `utils::match_strokes` helper's beam (5) is too narrow to even offer the
    // correct answer for this case; the real matcher runs with a much wider beam (100+), so
    // call the library function directly with one to match.
    let result = match_strokes(reference, user.to_stroke_vector(), Weights::default(), 64);
    assert_eq!(
        result[0].user_stroke_order.as_slice(),
        vec![0, 1, 2, 3, MISSING, MISSING, 5]
    );
}
