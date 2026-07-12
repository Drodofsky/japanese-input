mod utils;

use japanese_input::analyze::AnalyzeResult;

use crate::utils::*;

#[test]
fn いち() {
    let res = analyze('一', "一");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn に() {
    let res = analyze('二', "二");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn に_wo() {
    let res = analyze('二', "二_wo");
    assert_eq!(
        res,
        AnalyzeResult::StrokeOrder {
            correct: include_str!("../../data/test/二_wo_c.svg").to_string(),
            wrong: include_str!("../../data/test/二_wo_w.svg").to_string()
        }
    )
}

#[test]
fn さん() {
    let res = analyze('三', "三");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn さん_wo1() {
    let res = analyze('三', "三_wo1");
    assert_eq!(
        res,
        AnalyzeResult::StrokeOrder {
            correct: include_str!("../../data/test/三_wo1_c.svg").to_string(),
            wrong: include_str!("../../data/test/三_wo1_w.svg").to_string()
        }
    )
}

#[test]
fn さん_wo2() {
    let res = analyze('三', "三_wo2");
    assert_eq!(
        res,
        AnalyzeResult::StrokeOrder {
            correct: include_str!("../../data/test/三_wo2_c.svg").to_string(),
            wrong: include_str!("../../data/test/三_wo2_w.svg").to_string()
        }
    )
}

#[test]
fn さん_wo3() {
    let res = analyze('三', "三_wo3");
    assert_eq!(
        res,
        AnalyzeResult::StrokeOrder {
            correct: include_str!("../../data/test/三_wo3_c.svg").to_string(),
            wrong: include_str!("../../data/test/三_wo3_w.svg").to_string()
        }
    )
}

#[test]
fn さん_p1() {
    let res = analyze('三', "三_p1");
    assert_eq!(
        res,
        AnalyzeResult::ExtraOrMissingStrokes {
            correct: include_str!("../../data/test/三_p1_c.svg").to_string(),
            wrong: include_str!("../../data/test/三_p1_w.svg").to_string()
        }
    )
}

#[test]
fn さん_p1_wo() {
    let res = analyze('三', "三_p1_wo");
    assert_eq!(
        res,
        AnalyzeResult::ExtraOrMissingStrokes {
            correct: include_str!("../../data/test/三_p1_wo_c.svg").to_string(),
            wrong: include_str!("../../data/test/三_p1_wo_w.svg").to_string()
        }
    )
}

#[test]
fn さん_p2() {
    let res = analyze('三', "三_p2");
    assert_eq!(
        res,
        AnalyzeResult::ExtraOrMissingStrokes {
            correct: include_str!("../../data/test/三_p2_c.svg").to_string(),
            wrong: include_str!("../../data/test/三_p2_w.svg").to_string()
        }
    )
}

#[test]
fn さん_m1() {
    let res = analyze('三', "三_m1");
    assert_eq!(
        res,
        AnalyzeResult::ExtraOrMissingStrokes {
            correct: include_str!("../../data/test/三_m1_c.svg").to_string(),
            wrong: include_str!("../../data/test/三_m1_w.svg").to_string()
        }
    )
}

#[test]
fn さん_m2() {
    let res = analyze('三', "三_m2");
    assert_eq!(
        res,
        AnalyzeResult::ExtraOrMissingStrokes {
            correct: include_str!("../../data/test/三_m2_c.svg").to_string(),
            wrong: include_str!("../../data/test/三_m2_w.svg").to_string()
        }
    )
}

#[test]
fn じゅう() {
    let res = analyze('十', "十");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn じゅう_wo1() {
    let res = analyze('十', "十_wo1");
    assert_eq!(
        res,
        AnalyzeResult::StrokeOrder {
            correct: include_str!("../../data/test/十_wo1_c.svg").to_string(),
            wrong: include_str!("../../data/test/十_wo1_w.svg").to_string()
        }
    )
}
#[test]
fn せい() {
    let res = analyze('生', "生");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn つち() {
    let res = analyze('土', "土");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn つち_m1() {
    let res = analyze('土', "土_m1");
    assert_eq!(
        res,
        AnalyzeResult::ExtraOrMissingStrokes {
            correct: include_str!("../../data/test/土_m1_c.svg").to_string(),
            wrong: include_str!("../../data/test/土_m1_w.svg").to_string()
        }
    )
}

#[test]
fn みぎ() {
    let res = analyze('右', "右");
    assert_eq!(res, AnalyzeResult::NoError)
}
#[test]
fn みぎ_wo1() {
    let res = analyze('右', "右_wo1");
    assert_eq!(
        res,
        AnalyzeResult::StrokeOrder {
            correct: include_str!("../../data/test/右_wo1_c.svg").to_string(),
            wrong: include_str!("../../data/test/右_wo1_w.svg").to_string()
        }
    )
}

#[test]
fn かわ() {
    let res = analyze('川', "川");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn かわ_wo1() {
    let res = analyze('川', "川_wo1");
    assert_eq!(
        res,
        AnalyzeResult::StrokeOrder {
            correct: include_str!("../../data/test/川_wo1_c.svg").to_string(),
            wrong: include_str!("../../data/test/川_wo1_w.svg").to_string()
        }
    )
}

#[test]
fn おと() {
    let res = analyze('音', "音");
    assert_eq!(res, AnalyzeResult::NoError)
}
#[test]
fn おと_wo1() {
    let res = analyze('音', "音_wo1");
    assert_eq!(
        res,
        AnalyzeResult::StrokeOrder {
            correct: include_str!("../../data/test/音_wo1_c.svg").to_string(),
            wrong: include_str!("../../data/test/音_wo1_w.svg").to_string()
        }
    )
}

#[test]
fn おと_wp() {
    let res = analyze('音', "音_wp");
    assert_eq!(
        res,
        AnalyzeResult::StrokePositions {
            correct: include_str!("../../data/test/音_wp_c.svg").to_string(),
            wrong: include_str!("../../data/test/音_wp_w.svg").to_string()
        }
    )
}

#[test]
fn おう() {
    let res = analyze('王', "王");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn えん() {
    let res = analyze('円', "円");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn あめ() {
    let res = analyze('雨', "雨");
    assert_eq!(res, AnalyzeResult::NoError)
}

#[test]
fn ご_m1() {
    let res = analyze('語', "語_m1");
    assert_eq!(
        res,
        AnalyzeResult::ExtraOrMissingStrokes {
            correct: include_str!("../../data/test/語_m1_c.svg").to_string(),
            wrong: include_str!("../../data/test/語_m1_w.svg").to_string()
        }
    )
}
