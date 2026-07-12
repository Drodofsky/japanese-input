use kurbo::{Affine, Rect};

const PLACE_EPS: f64 = 0.05;

pub trait MapSpaceTo {
    #[must_use]
    fn map_space_to(self, dst: Rect) -> Option<Affine>;
}

impl MapSpaceTo for Rect {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "extents guarded > PLACE_EPS"
    )]
    #[inline]
    fn map_space_to(self, dst: Rect) -> Option<Affine> {
        let sw = self.width();
        let sh = self.height();
        if sw <= PLACE_EPS && sh <= PLACE_EPS {
            return None;
        }
        let sx = if sw > PLACE_EPS {
            dst.width() / sw
        } else {
            1.0_f64
        };
        let sy = if sh > PLACE_EPS {
            dst.height() / sh
        } else {
            1.0_f64
        };
        Some(
            Affine::translate(dst.center().to_vec2())
                * Affine::scale_non_uniform(sx, sy)
                * Affine::translate(-self.center().to_vec2()),
        )
    }
}
