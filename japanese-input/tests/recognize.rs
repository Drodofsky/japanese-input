mod utils;

use japanese_input::recognizer::Recognizer;

use crate::utils::*;

macro_rules! recognizer_test {
    ($name:ident,  $ch:literal) => {
        #[test]
        fn $name() {
            let model = load_recognizer_model();
            let recognizer = Recognizer::load(&model).unwrap();
            let user = load_test_file(&$ch.to_string());
            let result = recognizer.recognize(&user);
            assert_eq!(result[0].character, $ch);
        }
    };
}

recognizer_test!(あ, 'あ');
recognizer_test!(い, 'い');
recognizer_test!(う, 'う');
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

#[test]
fn bad_い() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("い2");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, 'い');
}
#[test]
fn bad_く() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("く2");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, 'く');
}
#[test]
fn bad_ぐ() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("ぐ2");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, 'ぐ');
}

#[test]
fn いち() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("一");
    let result = recognizer.recognize(&user);
    assert!(result[0].character == '一' || result[0].character == 'ー');
}
#[test]
fn に() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("二");
    let result = recognizer.recognize(&user);
    assert!(result[0].character == '二' || result[0].character == 'ニ');
}
#[ignore = "wo is currently not supported"]
#[test]
fn に_wo() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("二_wo");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '二');
}

#[test]
fn 三() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("三");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '三');
}
#[ignore = "wo is currently not supported"]
#[test]
fn 三_wo1() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("三_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '三');
}
#[ignore = "wo is currently not supported"]
#[test]
fn 三_wo2() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("三_wo2");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '三');
}
#[ignore = "wo is currently not supported"]
#[test]
fn 三_wo3() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("三_wo3");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '三');
}
#[test]
fn じゅう() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("十");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '十');
}
#[test]
#[ignore = "wo is currently not supported"]
fn じゅう_wo1() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("十_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '十');
}

#[test]
fn 川() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("川");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '川');
}
#[ignore = "wo is currently not supported"]
#[test]
fn 川_wo1() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("川_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '川');
}
#[test]
fn 円() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("円");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '円');
}
#[test]
fn 土() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("土");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '土');
}

#[test]
//#[ignore = "maybe the ai learned the wrong order"]
fn 右() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("右");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '右');
}
#[test]
fn 右_wo1() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("右_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '右');
}

#[test]
fn 生() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("生");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '生');
}

#[test]
fn 王() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("王");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '王');
}

#[test]
fn 音() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("音");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '音');
}
#[test]
#[ignore = "wo is currently not supported"]
fn 音_wo1() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("音_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '音');
}

#[test]
#[ignore = "too difficult"]
fn 音_wp() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("音_wp");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '音');
}

#[test]
fn 雨() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("雨");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '雨');
}
#[test]
fn 語_m1() {
    let model = load_recognizer_model();
    let recognizer = Recognizer::load(&model).unwrap();
    let user = load_test_file("語_m1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '語');
}
