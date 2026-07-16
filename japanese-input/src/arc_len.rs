use crate::stroke_point::StrokePoint;

pub trait ArcLen {
    fn arc_len(&self) -> f64;
}
impl ArcLen for [StrokePoint] {
    #[inline]
    fn arc_len(&self) -> f64 {
        self.iter()
            .zip(self.iter().skip(1))
            .map(|(a, b)| a.position.distance(b.position))
            .sum()
    }
}
