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
