use std::marker::PhantomData;
use std::ops::*;

use crate::num::{One, Zero};
use num_traits::real::Real;
use num_traits::Float;

use super::{markers::*, vector, ScaleFactor, Size, Vector};

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct Pos<T = f32, U = LogicalUnit> {
    pub x: T,
    pub y: T,
    #[doc(hidden)]
    pub _unit: PhantomData<U>,
}

impl<T: Copy, U> Copy for Pos<T, U> {}

impl<T: Clone, U> Clone for Pos<T, U> {
    fn clone(&self) -> Self {
        Pos {
            x: self.x.clone(),
            y: self.y.clone(),
            _unit: PhantomData,
        }
    }
}

impl<T, U> Pos<T, U> {
    /// Constructor, setting all components to zero.
    #[inline]
    pub fn origin() -> Self
    where
        T: Zero,
    {
        pos(Zero::zero(), Zero::zero())
    }

    /// The same as [`origin()`](#method.origin).
    #[inline]
    pub fn zero() -> Self
    where
        T: Zero,
    {
        Self::origin()
    }

    /// Constructor taking scalar values directly.
    #[inline]
    pub const fn new(x: T, y: T) -> Self {
        Pos {
            x,
            y,
            _unit: PhantomData,
        }
    }

    /// Constructor setting all components to the same value.
    #[inline]
    pub fn splat(v: T) -> Self
    where
        T: Clone,
    {
        Pos {
            x: v.clone(),
            y: v,
            _unit: PhantomData,
        }
    }
}

impl<T: Copy, U> Pos<T, U> {
    /// Cast this point into a vector.
    ///
    /// Equivalent to subtracting the origin from this point.
    #[inline]
    pub fn to_vector(self) -> Vector<T, U> {
        Vector {
            x: self.x,
            y: self.y,
            _unit: PhantomData,
        }
    }

    /// Swap x and y.
    #[inline]
    pub fn yx(self) -> Self {
        pos(self.y, self.x)
    }

    /// Cast the unit, preserving the numeric value.
    #[inline]
    pub fn cast_unit<V>(self) -> Pos<T, V> {
        pos(self.x, self.y)
    }

    /// Cast into an array with x and y.
    #[inline]
    pub fn to_array(self) -> [T; 2] {
        [self.x, self.y]
    }

    /// Cast into a tuple with x and y.
    #[inline]
    pub fn to_tuple(self) -> (T, T) {
        (self.x, self.y)
    }

    #[inline]
    #[must_use]
    pub fn map<R>(self, f: impl Fn(T) -> R) -> Pos<R, U> {
        Pos::new(f(self.x), f(self.y))
    }

    /// Linearly interpolate between this point and another point.
    #[inline]
    pub fn lerp(self, other: Self, t: T) -> Self
    where
        T: One + Sub<Output = T> + Mul<Output = T> + Add<Output = T>,
    {
        let one_t = T::one() - t;
        pos(one_t * self.x + t * other.x, one_t * self.y + t * other.y)
    }
}

impl<T: PartialOrd, U> Pos<T, U> {
    #[inline]
    pub fn min(self, other: Self) -> Self {
        pos(super::min(self.x, other.x), super::min(self.y, other.y))
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        pos(super::max(self.x, other.x), super::max(self.y, other.y))
    }

    /// Returns the point each component of which clamped by corresponding
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
}

impl<T: Float, U> Pos<T, U> {
    /// Returns true if all members are finite.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl<T: Copy + Add<T, Output = T>, U> Pos<T, U> {
    #[inline]
    pub fn add_size(self, other: &Size<T, U>) -> Self {
        pos(self.x + other.width, self.y + other.height)
    }
}

impl<T: Real + Sub<T, Output = T>, U> Pos<T, U> {
    #[inline]
    pub fn distance_to(self, other: Self) -> T {
        (self - other).length()
    }
}

impl<T: Neg, U> Neg for Pos<T, U> {
    type Output = Pos<T::Output, U>;

    #[inline]
    fn neg(self) -> Self::Output {
        pos(-self.x, -self.y)
    }
}

impl<T: Add, U> Add<Size<T, U>> for Pos<T, U> {
    type Output = Pos<T::Output, U>;

    #[inline]
    fn add(self, other: Size<T, U>) -> Self::Output {
        pos(self.x + other.width, self.y + other.height)
    }
}

impl<T: AddAssign, U> AddAssign<Size<T, U>> for Pos<T, U> {
    #[inline]
    fn add_assign(&mut self, other: Size<T, U>) {
        self.x += other.width;
        self.y += other.height;
    }
}

impl<T: Add, U> Add<Vector<T, U>> for Pos<T, U> {
    type Output = Pos<T::Output, U>;

    #[inline]
    fn add(self, other: Vector<T, U>) -> Self::Output {
        pos(self.x + other.x, self.y + other.y)
    }
}

impl<T: Copy + Add<T, Output = T>, U> AddAssign<Vector<T, U>> for Pos<T, U> {
    #[inline]
    fn add_assign(&mut self, other: Vector<T, U>) {
        *self = *self + other
    }
}

impl<T: Sub, U> Sub for Pos<T, U> {
    type Output = Vector<T::Output, U>;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        vector(self.x - other.x, self.y - other.y)
    }
}

impl<T: Sub, U> Sub<Size<T, U>> for Pos<T, U> {
    type Output = Pos<T::Output, U>;

    #[inline]
    fn sub(self, other: Size<T, U>) -> Self::Output {
        pos(self.x - other.width, self.y - other.height)
    }
}

impl<T: SubAssign, U> SubAssign<Size<T, U>> for Pos<T, U> {
    #[inline]
    fn sub_assign(&mut self, other: Size<T, U>) {
        self.x -= other.width;
        self.y -= other.height;
    }
}

impl<T: Sub, U> Sub<Vector<T, U>> for Pos<T, U> {
    type Output = Pos<T::Output, U>;

    #[inline]
    fn sub(self, other: Vector<T, U>) -> Self::Output {
        pos(self.x - other.x, self.y - other.y)
    }
}

impl<T: Copy + Sub<T, Output = T>, U> SubAssign<Vector<T, U>> for Pos<T, U> {
    #[inline]
    fn sub_assign(&mut self, other: Vector<T, U>) {
        *self = *self - other
    }
}

impl<T: Copy + Mul, U> Mul<T> for Pos<T, U> {
    type Output = Pos<T::Output, U>;

    #[inline]
    fn mul(self, scale: T) -> Self::Output {
        pos(self.x * scale, self.y * scale)
    }
}

impl<T: Copy + Mul<T, Output = T>, U> MulAssign<T> for Pos<T, U> {
    #[inline]
    fn mul_assign(&mut self, scale: T) {
        *self = *self * scale
    }
}

impl<T: Copy + Mul, U1, U2> Mul<ScaleFactor<T, U1, U2>> for Pos<T, U1> {
    type Output = Pos<T::Output, U2>;

    #[inline]
    fn mul(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        pos(self.x * scale.0, self.y * scale.0)
    }
}

impl<T: Copy + MulAssign, U> MulAssign<ScaleFactor<T, U, U>> for Pos<T, U> {
    #[inline]
    fn mul_assign(&mut self, scale: ScaleFactor<T, U, U>) {
        self.x *= scale.0;
        self.y *= scale.0;
    }
}

impl<T: Copy + Div, U> Div<T> for Pos<T, U> {
    type Output = Pos<T::Output, U>;

    #[inline]
    fn div(self, scale: T) -> Self::Output {
        pos(self.x / scale, self.y / scale)
    }
}

impl<T: Copy + Div<T, Output = T>, U> DivAssign<T> for Pos<T, U> {
    #[inline]
    fn div_assign(&mut self, scale: T) {
        *self = *self / scale
    }
}

impl<T: Copy + Div, U1, U2> Div<ScaleFactor<T, U1, U2>> for Pos<T, U2> {
    type Output = Pos<T::Output, U1>;

    #[inline]
    fn div(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        pos(self.x / scale.0, self.y / scale.0)
    }
}

impl<T: Copy + DivAssign, U> DivAssign<ScaleFactor<T, U, U>> for Pos<T, U> {
    #[inline]
    fn div_assign(&mut self, scale: ScaleFactor<T, U, U>) {
        self.x /= scale.0;
        self.y /= scale.0;
    }
}

impl<T: Zero, U> Zero for Pos<T, U> {
    #[inline]
    fn zero() -> Self {
        Self::origin()
    }
}

// impl<T: ApproxEq<T>, U> ApproxEq<Pos<T, U>> for Pos<T, U> {
//     #[inline]
//     fn approx_epsilon() -> Self {
//         pos(T::approx_epsilon(), T::approx_epsilon())
//     }

//     #[inline]
//     fn approx_eq_eps(&self, other: &Self, eps: &Self) -> bool {
//         self.x.approx_eq_eps(&other.x, &eps.x) && self.y.approx_eq_eps(&other.y, &eps.y)
//     }
// }

impl<T, U> Into<[T; 2]> for Pos<T, U> {
    fn into(self) -> [T; 2] {
        [self.x, self.y]
    }
}

impl<T, U> From<[T; 2]> for Pos<T, U> {
    fn from([x, y]: [T; 2]) -> Self {
        pos(x, y)
    }
}

impl<T, U> Into<(T, T)> for Pos<T, U> {
    fn into(self) -> (T, T) {
        (self.x, self.y)
    }
}

impl<T, U> From<(T, T)> for Pos<T, U> {
    fn from(tuple: (T, T)) -> Self {
        pos(tuple.0, tuple.1)
    }
}

impl<T> From<taffy::geometry::Point<T>> for Pos<T, LogicalUnit> {
    fn from(value: taffy::geometry::Point<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T> From<winit::dpi::LogicalPosition<T>> for Pos<T, LogicalUnit> {
    fn from(value: winit::dpi::LogicalPosition<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T> From<winit::dpi::PhysicalPosition<T>> for Pos<T, PhysicalUnit> {
    fn from(value: winit::dpi::PhysicalPosition<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

/// Shorthand for `Point::new(x, y)`.
#[inline]
pub const fn pos<T, U>(x: T, y: T) -> Pos<T, U> {
    Pos {
        x,
        y,
        _unit: PhantomData,
    }
}

pub type PhysicalPos<F = f32> = Pos<F, PhysicalUnit>;
