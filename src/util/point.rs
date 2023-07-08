use num_traits::Float;

use crate::impl_euclid_wrapper;

use super::traits::{CastUnit, MultiplyNumericFields};

use super::markers::*;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Pos<F = f32, U = LogicalUnit> {
    pub(super) inner: euclid::Point2D<F, U>,
}

pub type PhysicalPos<F = f32> = Pos<F, PhysicalUnit>;

impl_euclid_wrapper!(Pos, Point2D);
