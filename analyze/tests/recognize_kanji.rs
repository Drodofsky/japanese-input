mod utils;

use analyze::recognize_kanji::KanjiRecognizer;

use crate::utils::*;

#[test]
fn いち() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("一");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '一');
}
#[test]
fn に() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("二");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '二');
}
#[test]
fn に_wo() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("二_wo");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '二');
}

#[test]
fn 三() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("三");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '三');
}
#[test]
fn 三_wo1() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("三_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '三');
}
#[test]
fn 三_wo2() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("三_wo2");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '三');
}
#[test]
fn 三_wo3() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("三_wo3");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '三');
}
#[test]
fn じゅう() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("十");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '十');
}
#[test]
fn じゅう_wo1() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("十_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '十');
}

#[test]
fn 川() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("川");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '川');
}
#[test]
fn 川_wo1() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("川_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '川');
}
#[test]
fn 円() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("円");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '円');
}
#[test]
fn 土() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("土");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '土');
}

#[test]
fn 右() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("右");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '右');
}
#[test]
#[ignore = "fix later"]
fn 右_wo1() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("右_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '右');
}

#[test]
fn 生() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("生");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '生');
}

#[test]
fn 王() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("王");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '王');
}

#[test]
fn 音() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("音");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '音');
}
#[test]
fn 音_wo1() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("音_wo1");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '音');
}

#[test]
#[ignore = "fix later"]
fn 音_wp() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("音_wp");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '音');
}

#[test]
fn 雨() {
    let map = load_kanji_map();
    let recognizer = KanjiRecognizer::new(&map);
    let user = load_test_file("雨");
    let result = recognizer.recognize(&user);
    assert_eq!(result[0].character, '雨');
}
