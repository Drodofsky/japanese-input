#[must_use]
#[inline]
pub fn is_japanese(text: &str) -> bool {
    text.chars().any(is_japanese_char)
}

#[must_use]
#[inline]
pub fn is_japanese_char(character: char) -> bool {
    matches!(character,
        '\u{3041}'..='\u{3096}'
        | '\u{309D}'..='\u{309F}'
        | '\u{30A1}'..='\u{30FA}'
        | '\u{30FC}'..='\u{30FF}'
        | '\u{FF66}'..='\u{FF9D}'
        | '\u{3005}'
        | '\u{3007}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{20000}'..='\u{2A6DF}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hiragana_katakana_and_kanji_are_japanese() {
        assert!(is_japanese("ねこ"));
        assert!(is_japanese("ネコ"));
        assert!(is_japanese("猫"));
        assert!(is_japanese("ﾈｺ"));
    }

    #[test]
    fn latin_digits_and_punctuation_are_not() {
        assert!(!is_japanese("cat"));
        assert!(!is_japanese("Cat (animal)"));
        assert!(!is_japanese("123"));
        assert!(!is_japanese(""));
        assert!(!is_japanese("   "));
        assert!(!is_japanese("?!.,-"));
    }

    #[test]
    fn a_mixed_answer_counts_as_japanese() {
        assert!(is_japanese("cat 猫"));
        assert!(is_japanese("ねこ (cat)"));
    }

    #[test]
    fn japanese_punctuation_alone_is_not_enough() {
        assert!(!is_japanese("。、「」"));
        assert!(!is_japanese("　"));
    }

    #[test]
    fn marks_that_only_appear_in_japanese_text_count() {
        assert!(is_japanese("ー"));
        assert!(is_japanese("々"));
        assert!(is_japanese("〇"));
    }

    #[test]
    fn rare_kanji_outside_the_common_block_count() {
        assert!(is_japanese("\u{3400}"));
        assert!(is_japanese("\u{20000}"));
        assert!(is_japanese("\u{F900}"));
    }
}
