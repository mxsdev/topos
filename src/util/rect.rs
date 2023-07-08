use num_traits::Float;

use crate::impl_euclid_wrapper;

use super::traits::{CastUnit, MultiplyNumericFields};

use super::{markers::*, Pos, Size};

type Inner<F, U> = euclid::Box2D<F, U>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Rect<F = f32, U = LogicalUnit> {
    inner: Inner<F, U>,
}

impl_euclid_wrapper!(Rect, Box2D);

pub type PhysicalRect<F = f32> = Rect<F, PhysicalUnit>;

impl<F, U> Rect<F, U> {
    #[inline(always)]
    pub const fn new(min: Pos<F, U>, max: Pos<F, U>) -> Self {
        Self::from_euclid(Inner::new(min.inner, max.inner))
    }
}

impl<F: Copy, U> Rect<F, U> {
    // #[inline]
    // pub const fn from_min_size(min: Pos<F, U>, size: Size<F, U>) -> Self {
    //     Self::new(min, min + size.to_vector())
    // }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RoundedRect<F = f32, U = LogicalUnit> {
    pub inner: Rect<F, U>,
    pub radius: Option<F>,
}

pub type PhysicalRoundedRect<F = f32> = RoundedRect<F, PhysicalUnit>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_sdf() {
        // let rect: Rect<_, _> =
        //     euclid::Box2D::<f32, LogicalUnit>::new(euclid::point2(0., 0.), euclid::point2(4., 4.))
        //         .into();

        // use crate::util::WindowScaleFactor;

        // let scale_factor = WindowScaleFactor::new(2.);

        // let physical_rect = rect * scale_factor;

        // assert_eq!(physical_rect,)
    }
}
