use std::{
    marker::PhantomData,
    ops::{Div, Mul},
};

use num_traits::Float;
use ordered_float::NotNan;

use crate::util::DeviceUnit;

use super::super::{LogicalUnit, PhysicalUnit};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScaleFactor<Src, Dst, T = NotNan<f32>>(
    pub(super) T,
    #[doc(hidden)] PhantomData<(Src, Dst)>,
);

impl<T: Clone, Src, Dst> Clone for ScaleFactor<Src, Dst, T> {
    fn clone(&self) -> ScaleFactor<Src, Dst, T> {
        ScaleFactor::new(self.0.clone())
    }
}

impl<Src, Dst, T: Copy> Copy for ScaleFactor<Src, Dst, T> {}

pub type TransformationScaleFactor = ScaleFactor<DeviceUnit, PhysicalUnit>;
pub type DeviceScaleFactor = ScaleFactor<LogicalUnit, DeviceUnit>;
pub type CompleteScaleFactor = ScaleFactor<LogicalUnit, PhysicalUnit>;

impl<F: crate::num::One, Src, Dst> Default for ScaleFactor<Src, Dst, F> {
    fn default() -> Self {
        Self::identity()
    }
}

impl<T, Src, Dst> ScaleFactor<Src, Dst, T> {
    #[inline]
    pub fn new(x: impl Into<T>) -> Self {
        Self(x.into(), PhantomData)
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
    pub fn inverse(self) -> ScaleFactor<Dst, Src, T::Output>
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
    pub fn map<R>(self, f: impl Fn(T) -> R) -> ScaleFactor<Src, Dst, R> {
        ScaleFactor::new(f(self.0))
    }
}

impl<F: Float, Src, Dst> ScaleFactor<Src, Dst, NotNan<F>> {
    pub fn from_float(x: F) -> Self {
        Self::new(NotNan::new(x).unwrap())
    }

    pub fn as_float(self) -> ScaleFactor<Src, Dst, F> {
        self.into()
    }
}

impl<T, K, O, U1, U2, U3> Mul<ScaleFactor<U2, U3, T>> for ScaleFactor<U1, U2, K>
where
    K: Mul<T, Output = O>,
{
    type Output = ScaleFactor<U1, U3, O>;

    fn mul(self, rhs: ScaleFactor<U2, U3, T>) -> Self::Output {
        Self::Output::new(self.0 * rhs.0)
    }
}

impl<F: Float, Src, Dst> Into<ScaleFactor<Src, Dst, F>> for ScaleFactor<Src, Dst, NotNan<F>> {
    fn into(self) -> ScaleFactor<Src, Dst, F> {
        ScaleFactor::new(self.0.into_inner())
    }
}
