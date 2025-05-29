use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::*;

use crate::num::{One, Zero};
use num_traits::Float;
use num_traits::{real::Real, Signed};

use super::{size, Angle, Pos, ScaleFactor, Size, Trig};
use crate::util::{markers::*, max, min};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Vector<T = f32, U = LogicalUnit> {
    /// The `x` (traditionally, horizontal) coordinate.
    pub x: T,
    /// The `y` (traditionally, vertical) coordinate.
    pub y: T,
    #[doc(hidden)]
    pub _unit: PhantomData<U>,
}

impl<T: Copy, U> Copy for Vector<T, U> {}

impl<T: Clone, U> Clone for Vector<T, U> {
    fn clone(&self) -> Self {
        Vector {
            x: self.x.clone(),
            y: self.y.clone(),
            _unit: PhantomData,
        }
    }
}

impl<T, U> Vector<T, U> {
    /// Constructor, setting all components to zero.
    #[inline]
    pub fn zero() -> Self
    where
        T: Zero,
    {
        Vector::new(Zero::zero(), Zero::zero())
    }

    /// Constructor, setting all components to one.
    #[inline]
    pub fn one() -> Self
    where
        T: One,
    {
        Vector::new(One::one(), One::one())
    }

    /// Constructor taking scalar values directly.
    #[inline]
    pub const fn new(x: T, y: T) -> Self {
        Vector {
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
        Vector {
            x: v.clone(),
            y: v,
            _unit: PhantomData,
        }
    }

    /// Constructor taking angle and length
    pub fn from_angle_and_length(angle: Angle<T>, length: T) -> Self
    where
        T: Trig + Mul<Output = T> + Copy,
    {
        vector(length * angle.radians.cos(), length * angle.radians.sin())
    }

    /// Computes the vector with absolute values of each component.
    pub fn abs(self) -> Self
    where
        T: Signed,
    {
        vector(self.x.abs(), self.y.abs())
    }

    /// Dot product.
    #[inline]
    pub fn dot(self, other: Self) -> T
    where
        T: Add<Output = T> + Mul<Output = T>,
    {
        self.x * other.x + self.y * other.y
    }

    /// Returns the norm of the cross product [self.x, self.y, 0] x [other.x, other.y, 0].
    #[inline]
    pub fn cross(self, other: Self) -> T
    where
        T: Sub<Output = T> + Mul<Output = T>,
    {
        self.x * other.y - self.y * other.x
    }

    /// Returns the component-wise multiplication of the two vectors.
    #[inline]
    pub fn component_mul(self, other: Self) -> Self
    where
        T: Mul<Output = T>,
    {
        vector(self.x * other.x, self.y * other.y)
    }

    /// Returns the component-wise division of the two vectors.
    #[inline]
    pub fn component_div(self, other: Self) -> Self
    where
        T: Div<Output = T>,
    {
        vector(self.x / other.x, self.y / other.y)
    }
}

impl<T: Copy, U> Vector<T, U> {
    /// Cast this vector into a point.
    ///
    /// Equivalent to adding this vector to the origin.
    #[inline]
    pub fn to_pos(self) -> Pos<T, U> {
        Pos {
            x: self.x,
            y: self.y,
            _unit: PhantomData,
        }
    }

    /// Swap x and y.
    #[inline]
    pub fn yx(self) -> Self {
        vector(self.y, self.x)
    }

    /// Cast this vector into a size.
    #[inline]
    pub fn to_size(self) -> Size<T, U> {
        size(self.x, self.y)
    }

    /// Cast the unit.
    #[inline]
    pub fn cast_unit<V>(self) -> Vector<T, V> {
        vector(self.x, self.y)
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
    pub fn map<R>(self, f: impl Fn(T) -> R) -> Vector<R, U> {
        Vector::new(f(self.x), f(self.y))
    }

    /// Returns the signed angle between this vector and the x axis.
    /// Positive values counted counterclockwise, where 0 is `+x` axis, `PI/2`
    /// is `+y` axis.
    ///
    /// The returned angle is between -PI and PI.
    pub fn angle_from_x_axis(self) -> Angle<T>
    where
        T: Trig,
    {
        Angle::radians(Trig::fast_atan2(self.y, self.x))
    }

    // /// Creates translation by this vector in vector units.
    // #[inline]
    // pub fn to_transform(self) -> Transform<T, U, U>
    // where
    //     T: Zero + One,
    // {
    //     Transform::translation(self.x, self.y)
    // }
}

impl<T, U> Vector<T, U>
where
    T: Copy + Mul<T, Output = T> + Add<T, Output = T>,
{
    /// Returns the vector's length squared.
    #[inline]
    pub fn square_length(self) -> T {
        self.x * self.x + self.y * self.y
    }

    /// Returns this vector projected onto another one.
    ///
    /// Projecting onto a nil vector will cause a division by zero.
    #[inline]
    pub fn project_onto_vector(self, onto: Self) -> Self
    where
        T: Sub<T, Output = T> + Div<T, Output = T>,
    {
        onto * (self.dot(onto) / onto.square_length())
    }

    /// Returns the signed angle between this vector and another vector.
    ///
    /// The returned angle is between -PI and PI.
    pub fn angle_to(self, other: Self) -> Angle<T>
    where
        T: Sub<Output = T> + Trig,
    {
        Angle::radians(Trig::fast_atan2(self.cross(other), self.dot(other)))
    }
}

impl<T: Float, U> Vector<T, U> {
    /// Return the normalized vector even if the length is larger than the max value of Float.
    #[inline]
    #[must_use]
    pub fn robust_normalize(self) -> Self {
        let length = self.length();
        if length.is_infinite() {
            let scaled = self / T::max_value();
            scaled / scaled.length()
        } else {
            self / length
        }
    }

    /// Returns true if all members are finite.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Checks if `self` has length `1.0` up to a precision of `1e-6`.
    #[inline(always)]
    pub fn is_normalized(self) -> bool {
        (self.square_length() - T::one()).abs() < T::from(2e-6).unwrap()
    }
}

impl<T: Real, U> Vector<T, U> {
    /// Returns the vector length.
    #[inline]
    pub fn length(self) -> T {
        self.square_length().sqrt()
    }

    /// Returns the vector with length of one unit.
    #[inline]
    #[must_use]
    pub fn normalize(self) -> Self {
        self / self.length()
    }

    /// Returns the vector with length of one unit.
    ///
    /// Unlike [`Vector::normalize`](#method.normalize), this returns None in the case that the
    /// length of the vector is zero.
    #[inline]
    #[must_use]
    pub fn try_normalize(self) -> Option<Self> {
        let len = self.length();
        if len == T::zero() {
            None
        } else {
            Some(self / len)
        }
    }

    /// Return this vector scaled to fit the provided length.
    #[inline]
    pub fn with_length(self, length: T) -> Self {
        self.normalize() * length
    }

    /// Return this vector capped to a maximum length.
    #[inline]
    pub fn with_max_length(self, max_length: T) -> Self {
        let square_length = self.square_length();
        if square_length > max_length * max_length {
            return self * (max_length / square_length.sqrt());
        }

        self
    }

    /// Return this vector with a minimum length applied.
    #[inline]
    pub fn with_min_length(self, min_length: T) -> Self {
        let square_length = self.square_length();
        if square_length < min_length * min_length {
            return self * (min_length / square_length.sqrt());
        }

        self
    }

    /// Return this vector with minimum and maximum lengths applied.
    #[inline]
    pub fn clamp_length(self, min: T, max: T) -> Self {
        debug_assert!(min <= max);
        self.with_min_length(min).with_max_length(max)
    }
}

impl<T, U> Vector<T, U>
where
    T: Copy + One + Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    /// Linearly interpolate each component between this vector and another vector.
    #[inline]
    pub fn lerp(self, other: Self, t: T) -> Self {
        let one_t = T::one() - t;
        self * one_t + other * t
    }

    /// Returns a reflection vector using an incident ray and a surface normal.
    #[inline]
    pub fn reflect(self, normal: Self) -> Self {
        let two = T::one() + T::one();
        self - normal * two * self.dot(normal)
    }
}

impl<T: PartialOrd, U> Vector<T, U> {
    /// Returns the vector each component of which are minimum of this vector and another.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        vector(min(self.x, other.x), min(self.y, other.y))
    }

    /// Returns the vector each component of which are maximum of this vector and another.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        vector(max(self.x, other.x), max(self.y, other.y))
    }

    /// Returns the maximum of x and y components.
    #[inline]
    pub fn max_elem(self) -> T {
        max(self.x, self.y)
    }

    /// Returns the minimum of x and y components.
    #[inline]
    pub fn min_elem(self) -> T {
        min(self.x, self.y)
    }

    /// Returns the vector each component of which is clamped by corresponding
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

impl<T: Neg, U> Neg for Vector<T, U> {
    type Output = Vector<T::Output, U>;

    #[inline]
    fn neg(self) -> Self::Output {
        vector(-self.x, -self.y)
    }
}

impl<T: Add, U> Add for Vector<T, U> {
    type Output = Vector<T::Output, U>;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        Vector::new(self.x + other.x, self.y + other.y)
    }
}

impl<T: Add + Copy, U> Add<&Self> for Vector<T, U> {
    type Output = Vector<T::Output, U>;

    #[inline]
    fn add(self, other: &Self) -> Self::Output {
        Vector::new(self.x + other.x, self.y + other.y)
    }
}

impl<T: Add<Output = T> + Zero, U> Sum for Vector<T, U> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), Add::add)
    }
}

impl<'a, T: 'a + Add<Output = T> + Copy + Zero, U: 'a> Sum<&'a Self> for Vector<T, U> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), Add::add)
    }
}

impl<T: Copy + Add<T, Output = T>, U> AddAssign for Vector<T, U> {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        *self = *self + other
    }
}

impl<T: Sub, U> Sub for Vector<T, U> {
    type Output = Vector<T::Output, U>;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        vector(self.x - other.x, self.y - other.y)
    }
}

impl<T: Copy + Sub<T, Output = T>, U> SubAssign<Vector<T, U>> for Vector<T, U> {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other
    }
}

impl<T: Copy + Mul, U> Mul<T> for Vector<T, U> {
    type Output = Vector<T::Output, U>;

    #[inline]
    fn mul(self, scale: T) -> Self::Output {
        vector(self.x * scale, self.y * scale)
    }
}

impl<T: Copy + Mul<T, Output = T>, U> MulAssign<T> for Vector<T, U> {
    #[inline]
    fn mul_assign(&mut self, scale: T) {
        *self = *self * scale
    }
}

impl<T: Copy + Mul, U1, U2> Mul<ScaleFactor<U1, U2, T>> for Vector<T, U1> {
    type Output = Vector<T::Output, U2>;

    #[inline]
    fn mul(self, scale: ScaleFactor<U1, U2, T>) -> Self::Output {
        vector(self.x * scale.0, self.y * scale.0)
    }
}

impl<T: Copy + MulAssign, U> MulAssign<ScaleFactor<U, U, T>> for Vector<T, U> {
    #[inline]
    fn mul_assign(&mut self, scale: ScaleFactor<U, U, T>) {
        self.x *= scale.0;
        self.y *= scale.0;
    }
}

impl<T: Copy + Div, U> Div<T> for Vector<T, U> {
    type Output = Vector<T::Output, U>;

    #[inline]
    fn div(self, scale: T) -> Self::Output {
        vector(self.x / scale, self.y / scale)
    }
}

impl<T: Copy + Div<T, Output = T>, U> DivAssign<T> for Vector<T, U> {
    #[inline]
    fn div_assign(&mut self, scale: T) {
        *self = *self / scale
    }
}

impl<T: Copy + Div, U1, U2> Div<ScaleFactor<U1, U2, T>> for Vector<T, U2> {
    type Output = Vector<T::Output, U1>;

    #[inline]
    fn div(self, scale: ScaleFactor<U1, U2, T>) -> Self::Output {
        vector(self.x / scale.0, self.y / scale.0)
    }
}

impl<T: Copy + DivAssign, U> DivAssign<ScaleFactor<U, U, T>> for Vector<T, U> {
    #[inline]
    fn div_assign(&mut self, scale: ScaleFactor<U, U, T>) {
        self.x /= scale.0;
        self.y /= scale.0;
    }
}

impl<T, U> Into<[T; 2]> for Vector<T, U> {
    fn into(self) -> [T; 2] {
        [self.x, self.y]
    }
}

impl<T, U> From<[T; 2]> for Vector<T, U> {
    fn from([x, y]: [T; 2]) -> Self {
        vector(x, y)
    }
}

impl<T, U> Into<(T, T)> for Vector<T, U> {
    fn into(self) -> (T, T) {
        (self.x, self.y)
    }
}

impl<T, U> From<(T, T)> for Vector<T, U> {
    fn from(tuple: (T, T)) -> Self {
        vector(tuple.0, tuple.1)
    }
}

impl<T, U> From<Size<T, U>> for Vector<T, U> {
    fn from(size: Size<T, U>) -> Self {
        vector(size.width, size.height)
    }
}

#[inline]
pub const fn vector<T, U>(x: T, y: T) -> Vector<T, U> {
    Vector::new(x, y)
}

pub type PhysicalVector<F = f32> = Vector<F, PhysicalUnit>;

impl<T, U> Index<usize> for Vector<T, U> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("Index out of bounds"),
        }
    }
}

impl<T, U> IndexMut<usize> for Vector<T, U> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("Index out of bounds"),
        }
    }
}

