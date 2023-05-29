use num_traits::{Float, Num};

use crate::util::{LogicalUnit, Pos2};

pub trait Boundary<T: Float = f32, U = LogicalUnit> {
    fn sdf(&self, pos: euclid::Point2D<T, U>) -> T;
}

pub struct EmptyBoundary;

impl<T: Float, U> Boundary<T, U> for EmptyBoundary {
    fn sdf(&self, _pos: euclid::Point2D<T, U>) -> T {
        T::neg_infinity()
    }
}
