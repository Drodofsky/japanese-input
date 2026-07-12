mod utils;

use japanese_input::{recognize_hiragana::HiraganaRecognizer, stroke_point::ToStrokeVector as _};

use crate::utils::*;

macro_rules! hiragana_test {
    ($name:ident,  $ch:literal) => {
        #[test]
        fn $name() {
            let map = load_hiragana_map();
            let recognizer = HiraganaRecognizer::new(&map);
            let user = load_test_file(&$ch.to_string());
            let result = recognizer.recognize(user.to_stroke_vector());
            assert_eq!(result[0].character, $ch);
        }
    };
}

hiragana_test!(あ, 'あ');
hiragana_test!(い, 'い');
hiragana_test!(う, 'う');
hiragana_test!(え, 'え');
hiragana_test!(お, 'お');
hiragana_test!(か, 'か');
hiragana_test!(き, 'き');
hiragana_test!(く, 'く');
hiragana_test!(け, 'け');
hiragana_test!(こ, 'こ');
hiragana_test!(が, 'が');
hiragana_test!(ぎ, 'ぎ');
hiragana_test!(ぐ, 'ぐ');
hiragana_test!(げ, 'げ');
hiragana_test!(ご, 'ご');
hiragana_test!(ゃ, 'ゃ');
hiragana_test!(ゅ, 'ゅ');
hiragana_test!(ょ, 'ょ');
hiragana_test!(っ, 'っ');

// not nicely drawn:
#[test]
fn bad_い() {
    let map = load_hiragana_map();
    let recognizer = HiraganaRecognizer::new(&map);
    let user = load_test_file("い2");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, 'い');
}
#[test]
fn bad_く() {
    let map = load_hiragana_map();
    let recognizer = HiraganaRecognizer::new(&map);
    let user = load_test_file("く2");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, 'く');
}
#[test]
fn bad_ぐ() {
    let map = load_hiragana_map();
    let recognizer = HiraganaRecognizer::new(&map);
    let user = load_test_file("ぐ2");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, 'ぐ');
}
