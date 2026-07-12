pub trait ConvertStrokeIndex {
    type Output;
    fn convert_stroke_index(self) -> Self::Output;
}

impl ConvertStrokeIndex for usize {
    type Output = u8;

    #[inline]
    fn convert_stroke_index(self) -> Self::Output {
        self.try_into().unwrap_or(u8::MAX)
    }
}
