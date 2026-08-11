/// One admissible reference-stroke window, `[lo, hi]` inclusive. Two bytes.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrokeWindow {
    pub lo: u8,
    pub hi: u8,
}

impl StrokeWindow {
    /// Whether `n` reference strokes fall within this window.
    #[inline]
    #[must_use]
    pub const fn contains(self, n: u8) -> bool {
        self.lo <= n && n <= self.hi
    }
}

const fn w(lo: u8, hi: u8) -> StrokeWindow {
    StrokeWindow { lo, hi }
}

pub const WINDOWS: [StrokeWindow; 30] = [
    w(1, 3),
    w(2, 4),
    w(3, 7),
    w(4, 8),
    w(5, 9),
    w(6, 9),
    w(7, 9),
    w(8, 10),
    w(9, 11),
    w(10, 12),
    w(10, 12),
    w(11, 13),
    w(12, 14),
    w(13, 16),
    w(14, 20),
    w(15, 21),
    w(16, 22),
    w(17, 23),
    w(18, 23),
    w(19, 24),
    w(19, 25),
    w(21, 27),
    w(21, 28),
    w(21, 30),
    w(22, 30),
    w(24, 30),
    w(24, 30),
    w(26, 30),
    w(26, 30),
    w(26, 30),
];
#[inline]
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "idx is clamped to a valid index below"
)]
pub const fn window_for(user_count: usize) -> StrokeWindow {
    let Some(i) = user_count.checked_sub(1) else {
        return StrokeWindow { lo: 1, hi: 0 };
    };
    let idx = if i < WINDOWS.len() {
        i
    } else {
        WINDOWS.len().saturating_sub(1)
    };
    WINDOWS[idx]
}

#[inline]
#[must_use]
pub fn admits(template_count: usize, user_count: usize) -> bool {
    match u8::try_from(template_count) {
        Ok(n) => window_for(user_count).contains(n),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_matches_nothing() {
        assert!(!admits(1, 0));
    }

    #[test]
    fn clamps_above_table() {
        assert_eq!(window_for(35), window_for(30));
    }

    #[test]
    fn every_window_admits_its_bounds_and_nothing_outside_them() {
        for (index, window) in WINDOWS.iter().enumerate() {
            let user_count = index.saturating_add(1);
            assert_eq!(window_for(user_count), *window, "at {user_count}");
            assert!(
                admits(usize::from(window.lo), user_count),
                "at {user_count}"
            );
            assert!(
                admits(usize::from(window.hi), user_count),
                "at {user_count}"
            );
            assert!(
                !admits(usize::from(window.lo).saturating_sub(1), user_count),
                "at {user_count}"
            );
            assert!(
                !admits(usize::from(window.hi).saturating_add(1), user_count),
                "at {user_count}"
            );
        }
    }

    #[test]
    fn a_short_drawing_rules_out_a_long_kanji() {
        assert!(admits(3, 1) && !admits(4, 1));
        assert!(!admits(1, 8));
    }

    #[test]
    fn every_window_is_a_real_range_that_grows_with_the_drawing() {
        let mut previous: Option<StrokeWindow> = None;
        for window in &WINDOWS {
            assert!(window.lo <= window.hi, "{window:?} is empty");
            if let Some(before) = previous {
                assert!(before.lo <= window.lo, "{before:?} then {window:?}");
                assert!(before.hi <= window.hi, "{before:?} then {window:?}");
            }
            previous = Some(*window);
        }
    }

    #[test]
    fn a_count_too_large_for_a_byte_is_refused() {
        assert!(!admits(usize::from(u8::MAX).saturating_add(1), 1));
    }
}
