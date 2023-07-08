use std::ops::*;

use crate::num::{One, Zero};
use num_traits::real::Real;
use num_traits::{Float, FloatConst};

type Inner<T> = euclid::Angle<T>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Angle<T> {
    pub radians: T,
}

impl<T> Angle<T> {
    #[inline]
    pub fn radians(radians: T) -> Self {
        Angle { radians }
    }

    #[inline]
    pub fn get(self) -> T {
        self.radians
    }
}

impl<T> Angle<T>
where
    T: euclid::Trig,
{
    #[inline]
    pub fn degrees(deg: T) -> Self {
        Angle {
            radians: T::degrees_to_radians(deg),
        }
    }

    #[inline]
    pub fn to_degrees(self) -> T {
        T::radians_to_degrees(self.radians)
    }
}

impl<T> Angle<T>
where
    T: Rem<Output = T> + Sub<Output = T> + Add<Output = T> + Zero + FloatConst + PartialOrd + Copy,
{
    /// Returns this angle in the [0..2*PI[ range.
    pub fn positive(&self) -> Self {
        let two_pi = T::PI() + T::PI();
        let mut a = self.radians % two_pi;
        if a < T::zero() {
            a = a + two_pi;
        }
        Angle::radians(a)
    }

    /// Returns this angle in the ]-PI..PI] range.
    pub fn signed(&self) -> Self {
        Angle::pi() - (Angle::pi() - *self).positive()
    }
}

impl<T> Angle<T>
where
    T: Rem<Output = T>
        + Mul<Output = T>
        + Sub<Output = T>
        + Add<Output = T>
        + One
        + FloatConst
        + Copy,
{
    /// Returns the shortest signed angle between two angles.
    ///
    /// Takes wrapping and signs into account.
    pub fn angle_to(&self, to: Self) -> Self {
        let two = T::one() + T::one();
        let max = T::PI() * two;
        let d = (to.radians - self.radians) % max;

        Angle::radians(two * d % max - d)
    }

    /// Linear interpolation between two angles, using the shortest path.
    pub fn lerp(&self, other: Self, t: T) -> Self {
        *self + self.angle_to(other) * t
    }
}

impl<T> Angle<T>
where
    T: Float,
{
    /// Returns true if the angle is a finite number.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.radians.is_finite()
    }
}

impl<T> Angle<T>
where
    T: Real,
{
    /// Returns (sin(self), cos(self)).
    pub fn sin_cos(self) -> (T, T) {
        self.radians.sin_cos()
    }
}

impl<T> Angle<T>
where
    T: Zero,
{
    pub fn zero() -> Self {
        Angle::radians(T::zero())
    }
}

impl<T> Angle<T>
where
    T: FloatConst + Add<Output = T>,
{
    pub fn pi() -> Self {
        Angle::radians(T::PI())
    }

    pub fn two_pi() -> Self {
        Angle::radians(T::PI() + T::PI())
    }

    pub fn frac_pi_2() -> Self {
        Angle::radians(T::FRAC_PI_2())
    }

    pub fn frac_pi_3() -> Self {
        Angle::radians(T::FRAC_PI_3())
    }

    pub fn frac_pi_4() -> Self {
        Angle::radians(T::FRAC_PI_4())
    }
}

impl<T: Add<T, Output = T>> Add for Angle<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::radians(self.radians + other.radians)
    }
}

impl<T: Copy + Add<T, Output = T>> Add<&Self> for Angle<T> {
    type Output = Self;
    fn add(self, other: &Self) -> Self {
        Self::radians(self.radians + other.radians)
    }
}

// FIXME
// impl<T: Add + Zero> Sum for Angle<T> {
//     fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
//         iter.fold(Self::zero(), Add::add)
//     }
// }

// impl<'a, T: 'a + Add + Copy + Zero> Sum<&'a Self> for Angle<T> {
//     fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
//         iter.fold(Self::zero(), Add::add)
//     }
// }

impl<T: AddAssign<T>> AddAssign for Angle<T> {
    fn add_assign(&mut self, other: Angle<T>) {
        self.radians += other.radians;
    }
}

impl<T: Sub<T, Output = T>> Sub<Angle<T>> for Angle<T> {
    type Output = Angle<T>;
    fn sub(self, other: Angle<T>) -> <Self as Sub>::Output {
        Angle::radians(self.radians - other.radians)
    }
}

impl<T: SubAssign<T>> SubAssign for Angle<T> {
    fn sub_assign(&mut self, other: Angle<T>) {
        self.radians -= other.radians;
    }
}

impl<T: Div<T, Output = T>> Div<Angle<T>> for Angle<T> {
    type Output = T;
    #[inline]
    fn div(self, other: Angle<T>) -> T {
        self.radians / other.radians
    }
}

impl<T: Div<T, Output = T>> Div<T> for Angle<T> {
    type Output = Angle<T>;
    #[inline]
    fn div(self, factor: T) -> Angle<T> {
        Angle::radians(self.radians / factor)
    }
}

impl<T: DivAssign<T>> DivAssign<T> for Angle<T> {
    fn div_assign(&mut self, factor: T) {
        self.radians /= factor;
    }
}

impl<T: Mul<T, Output = T>> Mul<T> for Angle<T> {
    type Output = Angle<T>;
    #[inline]
    fn mul(self, factor: T) -> Angle<T> {
        Angle::radians(self.radians * factor)
    }
}

impl<T: MulAssign<T>> MulAssign<T> for Angle<T> {
    fn mul_assign(&mut self, factor: T) {
        self.radians *= factor;
    }
}

impl<T: Neg<Output = T>> Neg for Angle<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Angle::radians(-self.radians)
    }
}

pub trait Trig {
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn tan(self) -> Self;
    fn fast_atan2(y: Self, x: Self) -> Self;
    fn degrees_to_radians(deg: Self) -> Self;
    fn radians_to_degrees(rad: Self) -> Self;
}

macro_rules! trig {
    ($ty:ident) => {
        impl Trig for $ty {
            #[inline]
            fn sin(self) -> $ty {
                num_traits::Float::sin(self)
            }
            #[inline]
            fn cos(self) -> $ty {
                num_traits::Float::cos(self)
            }
            #[inline]
            fn tan(self) -> $ty {
                num_traits::Float::tan(self)
            }

            /// A slightly faster approximation of `atan2`.
            ///
            /// Note that it does not deal with the case where both x and y are 0.
            #[inline]
            fn fast_atan2(y: $ty, x: $ty) -> $ty {
                // This macro is used with f32 and f64 and clippy warns about the extra
                // precision with f32.
                #![cfg_attr(feature = "cargo-clippy", allow(excessive_precision))]

                // See https://math.stackexchange.com/questions/1098487/atan2-faster-approximation#1105038
                use core::$ty::consts;
                let x_abs = num_traits::Float::abs(x);
                let y_abs = num_traits::Float::abs(y);
                let a = x_abs.min(y_abs) / x_abs.max(y_abs);
                let s = a * a;
                let mut result =
                    ((-0.046_496_474_9 * s + 0.159_314_22) * s - 0.327_622_764) * s * a + a;
                if y_abs > x_abs {
                    result = consts::FRAC_PI_2 - result;
                }
                if x < 0.0 {
                    result = consts::PI - result
                }
                if y < 0.0 {
                    result = -result
                }

                result
            }

            #[inline]
            fn degrees_to_radians(deg: Self) -> Self {
                deg.to_radians()
            }

            #[inline]
            fn radians_to_degrees(rad: Self) -> Self {
                rad.to_degrees()
            }
        }
    };
}

trig!(f32);
trig!(f64);
