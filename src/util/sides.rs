use num_traits::Float;

use crate::impl_euclid_wrapper;

use super::traits::{CastUnit, MultiplyNumericFields};

use super::markers::*;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Sides<F = f32, U = LogicalUnit> {
    inner: euclid::SideOffsets2D<F, U>,
}

pub type PhysicalSides<F = f32> = Sides<F, PhysicalUnit>;

impl_euclid_wrapper!(Sides, SideOffsets2D);
