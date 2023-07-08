use num_traits::Float;

use crate::impl_euclid_wrapper;

use super::traits::{CastUnit, MultiplyNumericFields};

use super::markers::*;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Vector<F = f32, U = LogicalUnit> {
    pub(super) inner: euclid::Vector2D<F, U>,
}

pub type PhysicalVector<F = f32> = Vector<F, PhysicalUnit>;

impl_euclid_wrapper!(Vector, Vector2D);
