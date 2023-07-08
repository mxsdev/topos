use std::{
    marker::PhantomData,
    ops::{Div, Mul},
};

use super::{LogicalUnit, PhysicalUnit};

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct ScaleFactor<T, Src, Dst>(pub(super) T, #[doc(hidden)] PhantomData<(Src, Dst)>);

impl<T: Clone, Src, Dst> Clone for ScaleFactor<T, Src, Dst> {
    fn clone(&self) -> ScaleFactor<T, Src, Dst> {
        ScaleFactor::new(self.0.clone())
    }
}

impl<T: Copy, Src, Dst> Copy for ScaleFactor<T, Src, Dst> {}

pub type WindowScaleFactor = ScaleFactor<f32, LogicalUnit, PhysicalUnit>;

impl<T, Src, Dst> ScaleFactor<T, Src, Dst> {
    #[inline]
    pub const fn new(x: T) -> Self {
        Self(x, PhantomData)
    }

    /// Creates an identity scale (1.0).
    #[inline]
    pub fn identity() -> Self
    where
        T: crate::num::One,
    {
        ScaleFactor::new(T::one())
    }

    #[inline]
    pub fn inverse(self) -> ScaleFactor<T::Output, Dst, Src>
    where
        T: crate::num::One + Div,
    {
        ScaleFactor::new(T::one() / self.0)
    }

    #[inline]
    pub fn get(self) -> T {
        self.0
    }

    #[inline]
    pub fn map<R>(self, f: impl Fn(T) -> R) -> ScaleFactor<R, Src, Dst> {
        ScaleFactor::new(f(self.0))
    }
}
