pub trait ToIndex {
    fn to_index(self) -> usize;
}

impl ToIndex for f64 {
    #[expect(clippy::as_conversions, reason = "f64 to usize")]
    #[expect(clippy::cast_sign_loss, reason = "f64 to usize")]
    #[expect(clippy::cast_possible_truncation, reason = "f64 to usize")]
    #[inline]
    fn to_index(self) -> usize {
        self as usize
    }
}
