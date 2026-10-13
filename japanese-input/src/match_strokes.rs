//! The stroke-mapping vocabulary: the notation a mapping is written in, and the one geometric
//! helper that shares it.
//!
//! The beam-search matcher that used to live here is gone. Stroke-order analysis now runs the
//! evolved matcher in [`crate::ge_matcher`], which reaches the same answers through a different
//! mechanism — it runs a program discovered by evolution that builds one assignment and then
//! revises it, rather than enumerating slicings of the kanji tree and keeping the best few.
//!
//! What stayed is what the notation itself needs, because the whole codebase speaks it: the two
//! sentinels, the [`StrokeVec`] a mapping is carried in, and [`joined_reference_shape`], which
//! describes what several reference strokes look like when drawn as one motion.
//!
//! There is one notation, and no conversion anywhere: a reference stroke joined into an earlier
//! drawn stroke carries that stroke's own index, so a join is simply a repeated index. The old
//! `FILLER` sentinel for a continuation is gone, along with the beam-search matcher that needed
//! it internally and translated on the way out.

use kurbo::Vec2;
use smallvec::SmallVec;

use crate::{
    shape::{Shape, ToShape as _},
    stroke_point::StrokePoint,
};

/// Marks a reference stroke the user never drew.
pub const MISSING: u8 = u8::MAX;


/// One mapping: for each reference stroke, which drawn stroke explains it.
pub type StrokeVec = SmallVec<[u8; 32]>;

/// Shape of `length` consecutive reference strokes starting at `offset`, drawn as one
/// motion.
#[must_use]
#[inline]
pub fn joined_reference_shape(
    reference_strokes: &[Vec<StrokePoint>],
    offset: usize,
    length: usize,
) -> Option<Shape> {
    let mut points: Vec<StrokePoint> = Vec::new();
    for step in 0..length {
        let stroke = reference_strokes.get(offset.saturating_add(step))?;
        for (index, point) in stroke.iter().enumerate() {
            let mut copy = *point;
            if index == 0 {
                copy.displacement = match points.last() {
                    Some(previous) => Vec2::new(
                        point.position.x - previous.position.x,
                        point.position.y - previous.position.y,
                    ),
                    None => Vec2::ZERO,
                };
            }
            points.push(copy);
        }
    }
    let shape = points.to_shape();
    shape.is_usable().then_some(shape)
}
