use std::marker::PhantomData;

use crate::util::markers::*;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Sides<T = f32, U = LogicalUnit> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
    #[doc(hidden)]
    pub _unit: PhantomData<U>,
}

pub type PhysicalSides<F = f32> = Sides<F, PhysicalUnit>;
