use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::*;

use crate::num::{One, Zero};
use num_traits::{Float, Signed};

use super::{ScaleFactor, Vector};
use crate::util::{markers::*, max, min, taffy::*};

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct Size<T = f32, U = LogicalUnit> {
    /// The extent of the element in the `U` units along the `x` axis (usually horizontal).
    pub width: T,
    /// The extent of the element in the `U` units along the `y` axis (usually vertical).
    pub height: T,
    #[doc(hidden)]
    pub _unit: PhantomData<U>,
}

impl<T: Copy, U> Copy for Size<T, U> {}

impl<T: Clone, U> Clone for Size<T, U> {
    fn clone(&self) -> Self {
        Size {
            width: self.width.clone(),
            height: self.height.clone(),
            _unit: PhantomData,
        }
    }
}

pub type PhysicalSize<F = f32> = Size<F, PhysicalUnit>;

impl<T, U> Size<T, U> {
    /// The same as [`Zero::zero()`] but available without importing trait.
    ///
    /// [`Zero::zero()`]: ./num/trait.Zero.html#tymethod.zero
    #[inline]
    pub fn zero() -> Self
    where
        T: Zero,
    {
        Size::new(Zero::zero(), Zero::zero())
    }

    /// Constructor taking scalar values.
    #[inline]
    pub const fn new(width: T, height: T) -> Self {
        Size {
            width,
            height,
            _unit: PhantomData,
        }
    }

    /// Constructor setting all components to the same value.
    #[inline]
    pub fn splat(v: T) -> Self
    where
        T: Clone,
    {
        Size {
            width: v.clone(),
            height: v,
            _unit: PhantomData,
        }
    }
}

impl<T: Copy, U> Size<T, U> {
    /// Return this size as an array of two elements (width, then height).
    #[inline]
    pub fn to_array(self) -> [T; 2] {
        [self.width, self.height]
    }

    /// Return this size as a tuple of two elements (width, then height).
    #[inline]
    pub fn to_tuple(self) -> (T, T) {
        (self.width, self.height)
    }

    /// Return this size as a vector with width and height.
    #[inline]
    pub fn to_vector(self) -> Vector<T, U> {
        Vector::new(self.width, self.height)
    }

    /// Cast the unit
    #[inline]
    pub fn cast_unit<V>(self) -> Size<T, V> {
        Size::new(self.width, self.height)
    }

    #[inline]
    #[must_use]
    pub fn map<R>(self, f: impl Fn(T) -> R) -> Size<R, U> {
        Size::new(f(self.width), f(self.height))
    }

    /// Returns result of multiplication of both components
    pub fn area(self) -> T::Output
    where
        T: Mul,
    {
        self.width * self.height
    }

    /// Linearly interpolate each component between this size and another size.
    #[inline]
    pub fn lerp(self, other: Self, t: T) -> Self
    where
        T: One + Sub<Output = T> + Mul<Output = T> + Add<Output = T>,
    {
        let one_t = T::one() - t;
        self * one_t + other * t
    }
}

impl<T, U> Size<Option<T>, U> {
    pub fn try_unwrap(self) -> Option<Size<T, U>> {
        Option::zip(self.width, self.height).map(|(w, h)| Size::new(w, h))
    }
}

impl<T: Float, U> Size<T, U> {
    /// Returns true if all members are finite.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.width.is_finite() && self.height.is_finite()
    }
}

impl<T: Signed, U> Size<T, U> {
    /// Computes the absolute value of each component.
    ///
    /// For `f32` and `f64`, `NaN` will be returned for component if the component is `NaN`.
    ///
    /// For signed integers, `::MIN` will be returned for component if the component is `::MIN`.
    pub fn abs(self) -> Self {
        size(self.width.abs(), self.height.abs())
    }

    /// Returns `true` if both components is positive and `false` any component is zero or negative.
    pub fn is_positive(self) -> bool {
        self.width.is_positive() && self.height.is_positive()
    }
}

impl<T: PartialOrd, U> Size<T, U> {
    /// Returns the size each component of which are minimum of this size and another.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        size(min(self.width, other.width), min(self.height, other.height))
    }

    /// Returns the size each component of which are maximum of this size and another.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        size(max(self.width, other.width), max(self.height, other.height))
    }

    /// Returns the size each component of which clamped by corresponding
    /// components of `start` and `end`.
    ///
    /// Shortcut for `self.max(start).min(end)`.
    #[inline]
    pub fn clamp(self, start: Self, end: Self) -> Self
    where
        T: Copy,
    {
        self.max(start).min(end)
    }

    // Returns true if this size is larger or equal to the other size in all dimensions.
    #[inline]
    pub fn contains(self, other: Self) -> bool {
        self.width >= other.width && self.height >= other.height
    }

    /// Returns `true` if any component of size is zero, negative, or NaN.
    pub fn is_empty(self) -> bool
    where
        T: Zero,
    {
        let zero = T::zero();
        // The condition is experessed this way so that we return true in
        // the presence of NaN.
        !(self.width > zero && self.height > zero)
    }
}

impl<T: Zero, U> Zero for Size<T, U> {
    #[inline]
    fn zero() -> Self {
        Size::new(Zero::zero(), Zero::zero())
    }
}

impl<T: Neg, U> Neg for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline]
    fn neg(self) -> Self::Output {
        Size::new(-self.width, -self.height)
    }
}

impl<T: Add, U> Add for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        Size::new(self.width + other.width, self.height + other.height)
    }
}

impl<T: Copy + Add<T, Output = T>, U> Add<&Self> for Size<T, U> {
    type Output = Self;
    fn add(self, other: &Self) -> Self {
        Size::new(self.width + other.width, self.height + other.height)
    }
}

impl<T: Add<Output = T> + Zero, U> Sum for Size<T, U> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), Add::add)
    }
}

impl<'a, T: 'a + Add<Output = T> + Copy + Zero, U: 'a> Sum<&'a Self> for Size<T, U> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), Add::add)
    }
}

impl<T: AddAssign, U> AddAssign for Size<T, U> {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.width += other.width;
        self.height += other.height;
    }
}

impl<T: Sub, U> Sub for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        Size::new(self.width - other.width, self.height - other.height)
    }
}

impl<T: SubAssign, U> SubAssign for Size<T, U> {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.width -= other.width;
        self.height -= other.height;
    }
}

impl<T: Copy + Mul, U> Mul<T> for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline]
    fn mul(self, scale: T) -> Self::Output {
        Size::new(self.width * scale, self.height * scale)
    }
}

impl<T: Copy + MulAssign, U> MulAssign<T> for Size<T, U> {
    #[inline]
    fn mul_assign(&mut self, other: T) {
        self.width *= other;
        self.height *= other;
    }
}

impl<T: Copy + Mul, U1, U2> Mul<ScaleFactor<U1, U2, T>> for Size<T, U1> {
    type Output = Size<T::Output, U2>;

    #[inline]
    fn mul(self, scale: ScaleFactor<U1, U2, T>) -> Self::Output {
        Size::new(self.width * scale.0, self.height * scale.0)
    }
}

impl<T: Copy + MulAssign, U> MulAssign<ScaleFactor<U, U, T>> for Size<T, U> {
    #[inline]
    fn mul_assign(&mut self, other: ScaleFactor<U, U, T>) {
        *self *= other.0;
    }
}

impl<T: Copy + Div, U> Div<T> for Size<T, U> {
    type Output = Size<T::Output, U>;

    #[inline]
    fn div(self, scale: T) -> Self::Output {
        Size::new(self.width / scale, self.height / scale)
    }
}

impl<T: Copy + DivAssign, U> DivAssign<T> for Size<T, U> {
    #[inline]
    fn div_assign(&mut self, other: T) {
        self.width /= other;
        self.height /= other;
    }
}

impl<T: Copy + Div, U1, U2> Div<ScaleFactor<U1, U2, T>> for Size<T, U2> {
    type Output = Size<T::Output, U1>;

    #[inline]
    fn div(self, scale: ScaleFactor<U1, U2, T>) -> Self::Output {
        Size::new(self.width / scale.0, self.height / scale.0)
    }
}

impl<T: Copy + DivAssign, U> DivAssign<ScaleFactor<U, U, T>> for Size<T, U> {
    #[inline]
    fn div_assign(&mut self, other: ScaleFactor<U, U, T>) {
        *self /= other.0;
    }
}

/// Shorthand for `Size::new(w, h)`.
#[inline]
pub const fn size<T, U>(w: T, h: T) -> Size<T, U> {
    Size::new(w, h)
}

impl<T, U> From<Vector<T, U>> for Size<T, U> {
    #[inline]
    fn from(v: Vector<T, U>) -> Self {
        size(v.x, v.y)
    }
}

impl<T, U> Into<[T; 2]> for Size<T, U> {
    #[inline]
    fn into(self) -> [T; 2] {
        [self.width, self.height]
    }
}

impl<T, U> From<[T; 2]> for Size<T, U> {
    #[inline]
    fn from([w, h]: [T; 2]) -> Self {
        size(w, h)
    }
}

impl<T, U> Into<(T, T)> for Size<T, U> {
    #[inline]
    fn into(self) -> (T, T) {
        (self.width, self.height)
    }
}

impl<T, U> From<(T, T)> for Size<T, U> {
    #[inline]
    fn from(tuple: (T, T)) -> Self {
        size(tuple.0, tuple.1)
    }
}

impl Into<TaffySize<TaffyAvailableSpace>> for Size<f32, LogicalUnit> {
    fn into(self) -> TaffySize<TaffyAvailableSpace> {
        TaffySize {
            // TODO: support max-content and min-content
            height: self.height.into(),
            width: self.width.into(),
        }
    }
}

impl<T> Into<TaffySize<T>> for Size<T, LogicalUnit> {
    fn into(self) -> TaffySize<T> {
        TaffySize {
            height: self.height.into(),
            width: self.width.into(),
        }
    }
}

impl Into<TaffySize<TaffyDimension>> for Size<f32, LogicalUnit> {
    fn into(self) -> TaffySize<TaffyDimension> {
        TaffySize {
            height: TaffyDimension::length(self.height),
            width: TaffyDimension::length(self.width),
        }
    }
}

impl<T> From<TaffySize<T>> for Size<T, LogicalUnit> {
    fn from(value: TaffySize<T>) -> Self {
        Self::new(value.width, value.height)
    }
}

impl<T> From<winit::dpi::LogicalSize<T>> for Size<T, LogicalUnit> {
    fn from(value: winit::dpi::LogicalSize<T>) -> Self {
        Self::new(value.width, value.height)
    }
}

impl<T> From<winit::dpi::PhysicalSize<T>> for Size<T, PhysicalUnit> {
    fn from(value: winit::dpi::PhysicalSize<T>) -> Self {
        Self::new(value.width, value.height)
    }
}
