pub trait ConvertLossy {
    type Output;
    fn convert_lossy(self) -> Self::Output;
}

impl ConvertLossy for usize {
    type Output = f64;

    #[inline]
    #[expect(clippy::cast_precision_loss, reason = "convert_lossy")]
    #[expect(clippy::as_conversions, reason = "usize to f64")]
    fn convert_lossy(self) -> f64 {
        self as f64
    }
}
impl ConvertLossy for f64 {
    type Output = f32;

    #[inline]
    #[expect(clippy::cast_possible_truncation, reason = "convert_lossy")]
    #[expect(clippy::as_conversions, reason = "f64 to f32")]
    fn convert_lossy(self) -> f32 {
        self as f32
    }
}
