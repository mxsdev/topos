use std::ops::Div;

use super::{LogicalUnit, PhysicalUnit};

type Inner<T, Src, Dst> = euclid::Scale<T, Src, Dst>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ScaleFactor<T, Src, Dst> {
    pub(super) inner: Inner<T, Src, Dst>,
}

pub type WindowScaleFactor = ScaleFactor<f64, LogicalUnit, PhysicalUnit>;

impl<T, Src, Dst> ScaleFactor<T, Src, Dst> {
    #[inline(always)]
    pub(super) const fn from_euclid(inner: Inner<T, Src, Dst>) -> Self {
        Self { inner }
    }

    #[inline(always)]
    pub(super) fn to_euclid(self) -> Inner<T, Src, Dst> {
        self.inner
    }

    #[inline(always)]
    pub const fn new(x: T) -> Self {
        Self {
            inner: Inner::new(x),
        }
    }

    /// Creates an identity scale (1.0).
    #[inline(always)]
    pub fn identity() -> Self
    where
        T: euclid::num::One,
    {
        Inner::identity().into()
    }

    #[inline(always)]
    pub fn inverse(self) -> ScaleFactor<T::Output, Dst, Src>
    where
        T: euclid::num::One + Div,
    {
        self.inner.inverse().into()
    }

    #[inline(always)]
    pub fn get(self) -> T {
        self.inner.get()
    }

    #[inline]
    pub fn map<R>(self, f: impl Fn(T) -> R) -> ScaleFactor<R, Src, Dst> {
        ScaleFactor {
            inner: Inner::new(f(self.inner.0)),
        }
    }
}

impl<T, Src, Dst> Into<Inner<T, Src, Dst>> for ScaleFactor<T, Src, Dst> {
    #[inline(always)]
    fn into(self) -> Inner<T, Src, Dst> {
        self.to_euclid()
    }
}

impl<T, Src, Dst> From<Inner<T, Src, Dst>> for ScaleFactor<T, Src, Dst> {
    #[inline(always)]
    fn from(inner: Inner<T, Src, Dst>) -> Self {
        Self::from_euclid(inner)
    }
}
