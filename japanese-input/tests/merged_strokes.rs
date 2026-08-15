mod utils;

use crate::utils::*;
use japanese_input::match_strokes::{FILLER, match_strokes};
use japanese_input::merge_variants::generate;
use japanese_input::stroke_point::ToStrokeVector as _;
use japanese_input::weights::Weights;
use smallvec::smallvec;

/// 三: a flat, single group of three strokes.
#[test]
fn an_adjacent_pair_merges_in_a_flat_three_stroke_group() {
    let map = load_kanji_map();
    let tree = load_kanji_node(&map, '三');
    let ink = load_test_file("三").to_stroke_vector();
    let found = generate(&tree, &ink, &smallvec![0, 1, 2])
        .into_iter()
        .find(|candidate| candidate.suffix == "_m0-1")
        .expect("an adjacent-pair merge of the first two strokes");
    assert_eq!(found.truth.as_slice(), &[0, FILLER, 1]);
    let results = match_strokes(tree, found.ink, Weights::default(), 64);
    assert!(
        results
            .iter()
            .any(|result| result.user_stroke_order == found.truth),
        "{:?} was never offered",
        found.truth
    );
}

/// 三: the same flat group, merged as a single whole.
#[test]
fn a_flat_three_stroke_group_can_merge_as_a_whole() {
    let map = load_kanji_map();
    let tree = load_kanji_node(&map, '三');
    let ink = load_test_file("三").to_stroke_vector();
    let found = generate(&tree, &ink, &smallvec![0, 1, 2])
        .into_iter()
        .find(|candidate| candidate.suffix == "_m0-2")
        .expect("a whole-group merge of all three strokes");
    assert_eq!(found.truth.as_slice(), &[0, FILLER, FILLER]);
    let results = match_strokes(tree, found.ink, Weights::default(), 64);
    assert!(
        results
            .iter()
            .any(|result| result.user_stroke_order == found.truth),
        "{:?} was never offered",
        found.truth
    );
}

/// 二: the smallest possible group, two strokes merged together.
#[test]
fn the_only_two_strokes_in_a_group_can_merge_together() {
    let map = load_kanji_map();
    let tree = load_kanji_node(&map, '二');
    let ink = load_test_file("二").to_stroke_vector();
    let found = generate(&tree, &ink, &smallvec![0, 1])
        .into_iter()
        .find(|candidate| candidate.suffix == "_m0-1")
        .expect("the only two strokes merged together");
    assert_eq!(found.truth.as_slice(), &[0, FILLER]);
    let results = match_strokes(tree, found.ink, Weights::default(), 64);
    assert!(
        results
            .iter()
            .any(|result| result.user_stroke_order == found.truth),
        "{:?} was never offered",
        found.truth
    );
}

/// 右: five strokes split across two sibling groups.
#[test]
fn an_adjacent_pair_merges_in_the_first_of_two_sibling_groups() {
    let map = load_kanji_map();
    let tree = load_kanji_node(&map, '右');
    let ink = load_test_file("右").to_stroke_vector();
    let found = generate(&tree, &ink, &smallvec![0, 1, 2, 3, 4])
        .into_iter()
        .find(|candidate| candidate.suffix == "_m0-1")
        .expect("an adjacent-pair merge in the first group");
    let results = match_strokes(tree, found.ink, Weights::default(), 64);
    assert!(
        results
            .iter()
            .any(|result| result.user_stroke_order == found.truth),
        "{:?} was never offered",
        found.truth
    );
}

/// 右: the second, three-stroke sibling group, merged as a whole.
#[test]
fn a_whole_sibling_group_merges_inside_a_larger_kanji() {
    let map = load_kanji_map();
    let tree = load_kanji_node(&map, '右');
    let ink = load_test_file("右").to_stroke_vector();
    let found = generate(&tree, &ink, &smallvec![0, 1, 2, 3, 4])
        .into_iter()
        .find(|candidate| candidate.suffix == "_m2-4")
        .expect("a whole-group merge of the second group");
    let results = match_strokes(tree, found.ink, Weights::default(), 64);
    assert!(
        results
            .iter()
            .any(|result| result.user_stroke_order == found.truth),
        "{:?} was never offered",
        found.truth
    );
}

/// 音: nine strokes split across three sibling groups.
#[test]
fn an_adjacent_pair_merges_in_the_last_of_three_sibling_groups() {
    let map = load_kanji_map();
    let tree = load_kanji_node(&map, '音');
    let ink = load_test_file("音").to_stroke_vector();
    let found = generate(&tree, &ink, &smallvec![0, 1, 2, 3, 4, 5, 6, 7, 8])
        .into_iter()
        .find(|candidate| candidate.suffix == "_m5-6")
        .expect("an adjacent-pair merge in the last group");
    let results = match_strokes(tree, found.ink, Weights::default(), 64);
    assert!(
        results
            .iter()
            .any(|result| result.user_stroke_order == found.truth),
        "{:?} was never offered",
        found.truth
    );
}

/// 音: one drawing merges strokes in two different sibling groups at once.
#[test]
fn two_different_sibling_groups_merge_in_the_same_drawing() {
    let map = load_kanji_map();
    let tree = load_kanji_node(&map, '音');
    let ink = load_test_file("音").to_stroke_vector();
    let found = generate(&tree, &ink, &smallvec![0, 1, 2, 3, 4, 5, 6, 7, 8])
        .into_iter()
        .find(|candidate| candidate.suffix == "_m2-3+7-8")
        .expect("a combination merging two different groups at once");
    let results = match_strokes(tree, found.ink, Weights::default(), 64);
    assert!(
        results
            .iter()
            .any(|result| result.user_stroke_order == found.truth),
        "{:?} was never offered",
        found.truth
    );
}
