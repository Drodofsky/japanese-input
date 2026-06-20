mod utils;

use japanese_input::{recognize_character::Recognizer, stroke_point::ToStrokeVector as _};

use crate::utils::*;

macro_rules! recognizer_test {
    ($name:ident,  $ch:literal) => {
        #[test]
        fn $name() {
            let map = load_reference_map();
            let recognizer = Recognizer::new(&map);
            let user = load_test_file(&$ch.to_string());
            let result = recognizer.recognize(user.to_stroke_vector());
            assert_eq!(result[0].character, $ch);
        }
    };
}

recognizer_test!(あ, 'あ');
recognizer_test!(い, 'い');
// fix later
// recognizer_test!(う, 'う');
recognizer_test!(え, 'え');
recognizer_test!(お, 'お');
recognizer_test!(か, 'か');
recognizer_test!(き, 'き');
recognizer_test!(く, 'く');
recognizer_test!(け, 'け');
recognizer_test!(こ, 'こ');
recognizer_test!(が, 'が');
recognizer_test!(ぎ, 'ぎ');
recognizer_test!(ぐ, 'ぐ');
recognizer_test!(げ, 'げ');
recognizer_test!(ご, 'ご');
recognizer_test!(ゃ, 'ゃ');
recognizer_test!(ゅ, 'ゅ');
recognizer_test!(ょ, 'ょ');
recognizer_test!(っ, 'っ');
recognizer_test!(ッ, 'ッ');

// not nicely drawn:
#[test]
fn bad_い() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("い2");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, 'い');
}
#[test]
fn bad_く() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("く2");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, 'く');
}
#[test]
fn bad_ぐ() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("ぐ2");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, 'ぐ');
}

#[test]
#[ignore = "fix later"]
fn いち() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("一");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '一');
}
#[test]
fn に() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("二");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '二');
}
#[test]
fn に_wo() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("二_wo");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '二');
}

#[test]
fn 三() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("三");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '三');
}
#[test]
fn 三_wo1() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("三_wo1");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '三');
}
#[test]
fn 三_wo2() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("三_wo2");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '三');
}
#[test]
fn 三_wo3() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("三_wo3");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '三');
}
#[test]
fn じゅう() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("十");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '十');
}
#[test]
fn じゅう_wo1() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("十_wo1");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '十');
}

#[test]
fn 川() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("川");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '川');
}
#[test]
fn 川_wo1() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("川_wo1");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '川');
}
#[test]
fn 円() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("円");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '円');
}
#[test]
fn 土() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("土");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '土');
}

#[test]
fn 右() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("右");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '右');
}
#[test]
fn 右_wo1() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("右_wo1");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '右');
}

#[test]
fn 生() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("生");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '生');
}

#[test]
fn 王() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("王");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '王');
}

#[test]
fn 音() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("音");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '音');
}
#[test]
fn 音_wo1() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("音_wo1");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '音');
}

#[test]
#[ignore = "fix later"]
fn 音_wp() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("音_wp");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '音');
}

#[test]
fn 雨() {
    let map = load_reference_map();
    let recognizer = Recognizer::new(&map);
    let user = load_test_file("雨");
    let result = recognizer.recognize(user.to_stroke_vector());
    assert_eq!(result[0].character, '雨');
}
