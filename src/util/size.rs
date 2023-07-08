use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use euclid::Size2D;
use num_traits::{Float, Signed};

use crate::impl_euclid_wrapper;

use super::traits::{CastUnit, MultiplyNumericFields};

use super::{markers::*, ScaleFactor};

type Inner<F, U> = euclid::Size2D<F, U>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Size<F = f32, U = LogicalUnit> {
    inner: Inner<F, U>,
}

pub type PhysicalSize<F = f32> = Size<F, PhysicalUnit>;

impl_euclid_wrapper!(Size, Size2D);

impl<F, U> Size<F, U> {
    pub const fn new(width: F, height: F) -> Self {
        Self {
            inner: Inner::new(width, height),
        }
    }

    #[inline(always)]
    pub const fn inner_ref(&self) -> &euclid::Size2D<F, U> {
        &self.inner
    }
}

#[inline]
pub const fn size<T, U>(width: T, height: T) -> Size<T, U> {
    Size::<T, U>::new(width, height)
}

impl<F: Copy, U> Size<F, U> {
    #[inline(always)]
    pub fn width(&self) -> F {
        self.inner.width
    }

    #[inline(always)]
    pub const fn height(&self) -> F {
        self.inner.height
    }

    /// Return this size as an array of two elements (width, then height).
    #[inline(always)]
    pub fn to_array(self) -> [F; 2] {
        self.inner.to_array()
    }

    /// Return this size as a tuple of two elements (width, then height).
    #[inline(always)]
    pub fn to_tuple(self) -> (F, F) {
        self.inner.to_tuple()
    }

    /// Return this size as a vector with width and height.
    #[inline(always)]
    pub fn to_vector(self) -> super::Vector<F, U> {
        self.inner.to_vector().into()
    }

    #[inline]
    pub fn map<T>(self, f: impl Fn(F) -> T) -> Size<T, U> {
        Size::new(f(self.inner.width), f(self.inner.height))
    }

    /// Returns result of multiplication of both components
    #[inline(always)]
    pub fn area(self) -> F::Output
    where
        F: Mul,
    {
        self.inner.area()
    }

    /// Linearly interpolate each component between this size and another size.
    #[inline(always)]
    pub fn lerp(self, other: Self, t: F) -> Self
    where
        F: euclid::num::One + Sub<Output = F> + Mul<Output = F> + Add<Output = F>,
    {
        self.inner.lerp(other.into(), t).into()
    }
}

impl<T: Signed, U> Size<T, U> {
    /// Computes the absolute value of each component.
    ///
    /// For `f32` and `f64`, `NaN` will be returned for component if the component is `NaN`.
    ///
    /// For signed integers, `::MIN` will be returned for component if the component is `::MIN`.
    #[inline(always)]
    pub fn abs(self) -> Self {
        self.inner.abs().into()
    }

    /// Returns `true` if both components is positive and `false` any component is zero or negative.
    #[inline(always)]
    pub fn is_positive(self) -> bool {
        self.inner.is_positive()
    }
}

impl<T: PartialOrd, U> Size<T, U> {
    /// Returns the size each component of which are minimum of this size and another.
    #[inline(always)]
    pub fn min(self, other: Self) -> Self {
        self.inner.min(other.into()).into()
    }

    /// Returns the size each component of which are maximum of this size and another.
    #[inline(always)]
    pub fn max(self, other: Self) -> Self {
        self.inner.max(other.into()).into()
    }

    /// Returns the size each component of which clamped by corresponding
    /// components of `start` and `end`.
    ///
    /// Shortcut for `self.max(start).min(end)`.
    #[inline(always)]
    pub fn clamp(self, start: Self, end: Self) -> Self
    where
        T: Copy,
    {
        self.inner.clamp(start.into(), end.into()).into()
    }

    // Returns true if this size is larger or equal to the other size in all dimensions.
    #[inline(always)]
    pub fn contains(self, other: Self) -> bool {
        self.inner.contains(other.into())
    }

    /// Returns vector with results of "greater then" operation on each component.
    #[inline(always)]
    pub fn greater_than(self, other: Self) -> euclid::BoolVector2D {
        self.inner.greater_than(other.into())
    }

    /// Returns vector with results of "lower then" operation on each component.
    #[inline(always)]
    pub fn lower_than(self, other: Self) -> euclid::BoolVector2D {
        self.inner.lower_than(other.into())
    }

    /// Returns `true` if any component of size is zero, negative, or NaN.
    #[inline(always)]
    pub fn is_empty(self) -> bool
    where
        T: euclid::num::Zero,
    {
        self.inner.is_empty()
    }
}

impl<T: PartialEq, U> Size<T, U> {
    /// Returns vector with results of "equal" operation on each component.
    #[inline(always)]
    pub fn equal(self, other: Self) -> euclid::BoolVector2D {
        self.inner.equal(other.into())
    }

    /// Returns vector with results of "not equal" operation on each component.
    #[inline(always)]
    pub fn not_equal(self, other: Self) -> euclid::BoolVector2D {
        self.inner.not_equal(other.into())
    }
}

impl<F, U> Size<Option<F>, U> {
    /// Tries to unwrap every inner option;
    pub fn try_unwrap(self) -> Option<Size<F, U>> {
        Option::zip(self.inner.width, self.inner.height)
            .map(|(width, height)| Size::new(width, height))
    }
}

impl<T: euclid::num::Zero, U> euclid::num::Zero for Size<T, U> {
    #[inline(always)]
    fn zero() -> Self {
        Inner::zero().into()
    }
}

impl<T: Neg, U> Neg for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        self.inner.neg().into()
    }
}

impl<T: Add, U> Add for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        self.inner.add(other.into()).into()
    }
}

impl<T: Copy + Add<T, Output = T>, U> Add<&Self> for Size<T, U> {
    type Output = Self;
    fn add(self, other: &Self) -> Self {
        self.inner.add(&other.inner).into()
    }
}

impl<T: Add<Output = T> + euclid::num::Zero, U> Sum for Size<T, U> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        use euclid::num::Zero;
        iter.fold(Self::zero(), Add::add)
    }
}

impl<'a, T: 'a + Add<Output = T> + Copy + euclid::num::Zero, U: 'a> Sum<&'a Self> for Size<T, U> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        use euclid::num::Zero;
        iter.fold(Self::zero(), Add::add)
    }
}

impl<T: AddAssign, U> AddAssign for Size<T, U> {
    #[inline(always)]
    fn add_assign(&mut self, other: Self) {
        self.inner.add_assign(other.into())
    }
}

impl<T: Sub, U> Sub for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline(always)]
    fn sub(self, other: Self) -> Self::Output {
        self.inner.sub(other.into()).into()
    }
}

impl<T: SubAssign, U> SubAssign for Size<T, U> {
    #[inline(always)]
    fn sub_assign(&mut self, other: Self) {
        self.inner.sub_assign(other.into())
    }
}

impl<T: Copy + Mul, U> Mul<T> for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline(always)]
    fn mul(self, scale: T) -> Self::Output {
        self.inner.mul(scale).into()
    }
}

impl<T: Copy + MulAssign, U> MulAssign<T> for Size<T, U> {
    #[inline(always)]
    fn mul_assign(&mut self, other: T) {
        self.inner.mul_assign(other)
    }
}

impl<T: Copy + Mul, U1, U2> Mul<ScaleFactor<T, U1, U2>> for Size<T, U1> {
    type Output = Size<T::Output, U2>;

    #[inline(always)]
    fn mul(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        self.inner.mul(scale.inner).into()
    }
}

impl<T: Copy + MulAssign, U> MulAssign<ScaleFactor<T, U, U>> for Size<T, U> {
    #[inline(always)]
    fn mul_assign(&mut self, other: ScaleFactor<T, U, U>) {
        self.inner.mul_assign(other.inner)
    }
}

impl<T: Copy + Div, U> Div<T> for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline(always)]
    fn div(self, scale: T) -> Self::Output {
        self.inner.div(scale).into()
    }
}

impl<T: Copy + DivAssign, U> DivAssign<T> for Size<T, U> {
    #[inline(always)]
    fn div_assign(&mut self, other: T) {
        self.inner.div_assign(other)
    }
}

impl<T: Copy + Div, U1, U2> Div<ScaleFactor<T, U1, U2>> for Size<T, U2> {
    type Output = Size2D<T::Output, U1>;

    #[inline(always)]
    fn div(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        self.inner.div(scale.inner).into()
    }
}

impl<T: Copy + DivAssign, U> DivAssign<ScaleFactor<T, U, U>> for Size<T, U> {
    #[inline(always)]
    fn div_assign(&mut self, other: ScaleFactor<T, U, U>) {
        self.inner.div_assign(other.inner)
    }
}

impl<T, U> From<super::Vector<T, U>> for Size<T, U> {
    #[inline(always)]
    fn from(v: super::Vector<T, U>) -> Self {
        Into::<Inner<T, U>>::into(v.inner).into()
    }
}

impl<T, U> Into<[T; 2]> for Size<T, U> {
    #[inline(always)]
    fn into(self) -> [T; 2] {
        self.inner.into()
    }
}

impl<T, U> From<[T; 2]> for Size<T, U> {
    #[inline(always)]
    fn from(x: [T; 2]) -> Self {
        Inner::from(x).into()
    }
}

impl<T, U> Into<(T, T)> for Size<T, U> {
    #[inline(always)]
    fn into(self) -> (T, T) {
        self.inner.into()
    }
}

impl<T, U> From<(T, T)> for Size<T, U> {
    #[inline(always)]
    fn from(tuple: (T, T)) -> Self {
        Into::into(tuple)
    }
}

impl Into<taffy::geometry::Size<taffy::style::AvailableSpace>> for Size {
    fn into(self) -> taffy::geometry::Size<taffy::style::AvailableSpace> {
        taffy::geometry::Size {
            // TODO: support max-content and min-content
            height: self.inner.height.into(),
            width: self.inner.width.into(),
        }
    }
}

impl Into<taffy::geometry::Size<f32>> for Size {
    fn into(self) -> taffy::geometry::Size<f32> {
        taffy::geometry::Size {
            height: self.inner.height.into(),
            width: self.inner.width.into(),
        }
    }
}

impl Into<taffy::geometry::Size<taffy::style::Dimension>> for Size {
    fn into(self) -> taffy::geometry::Size<taffy::style::Dimension> {
        taffy::geometry::Size {
            height: taffy::style::Dimension::Points(self.inner.height),
            width: taffy::style::Dimension::Points(self.inner.width),
        }
    }
}

impl From<taffy::geometry::Size<f32>> for Size {
    fn from(value: taffy::geometry::Size<f32>) -> Self {
        Self::new(value.width, value.height)
    }
}
