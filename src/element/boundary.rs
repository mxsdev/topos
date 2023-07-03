use num_traits::{Float, Num};

use crate::util::{LogicalUnit, Pos2};

pub trait Boundary<T: Float = f32, U = LogicalUnit> {
    // might want to think about "on boundary" at some point
    fn is_inside(&self, pos: &euclid::Point2D<T, U>) -> bool;
}

pub trait SDF<T: Float = f32, U = LogicalUnit> {
    fn sdf(&self, pos: &euclid::Point2D<T, U>) -> T;
}

impl<U, F: Float, T: SDF<F, U>> Boundary<F, U> for T {
    fn is_inside(&self, pos: &euclid::Point2D<F, U>) -> bool {
        self.sdf(pos).is_sign_positive()
    }
}

pub trait RectLikeBoundary<T: Float = f32, U = LogicalUnit>: Boundary<T, U> {
    fn as_rect(&self) -> euclid::Box2D<T, U>;
    fn set_rect(&mut self, rect: euclid::Box2D<T, U>);
}

pub struct EmptyBoundary;

impl<T: Float, U> SDF<T, U> for EmptyBoundary {
    fn sdf(&self, _pos: &euclid::Point2D<T, U>) -> T {
        T::neg_infinity()
    }
}
