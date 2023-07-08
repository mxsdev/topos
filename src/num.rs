use std::ops::Shl;

use num_traits::{Float, Num};

pub trait MaxNum {
    fn max_num(self, other: Self) -> Self;
}

// impl<T: Ord> MaxNum for T {
//     fn max_num(self, other: Self) -> Self {
//         self.max(other)
//     }
// }

impl MaxNum for f32 {
    fn max_num(self, other: Self) -> Self {
        self.max(other)
    }
}

impl MaxNum for f64 {
    fn max_num(self, other: Self) -> Self {
        self.max(other)
    }
}

pub trait Two {
    const TWO: Self;
}

impl Two for f32 {
    const TWO: Self = 2.;
}

impl Two for f64 {
    const TWO: Self = 2.;
}

impl Two for u16 {
    const TWO: Self = 2;
}

impl Two for u32 {
    const TWO: Self = 2;
}

impl Two for u64 {
    const TWO: Self = 2;
}

pub trait Infty {
    const INFINITY: Self;
    const NEG_INFINITY: Self;
}

impl Infty for f32 {
    const INFINITY: Self = Self::INFINITY;
    const NEG_INFINITY: Self = Self::NEG_INFINITY;
}

impl Infty for f64 {
    const INFINITY: Self = Self::INFINITY;
    const NEG_INFINITY: Self = Self::NEG_INFINITY;
}

pub trait NextPowerOfTwo: Sized {
    type ClosestInt: Num + num_traits::One + Shl<i32, Output = Self::ClosestInt>;

    fn next_power_of_2(self) -> Self::ClosestInt {
        Self::ClosestInt::one() << self.next_power_of_2_exp()
    }

    fn next_power_of_2_exp(self) -> i32;
}

impl NextPowerOfTwo for f32 {
    type ClosestInt = i32;

    fn next_power_of_2_exp(self) -> i32 {
        self.log2().ceil() as i32
    }
}

// impl NextPowerOfTwo for f64 {
//     type ClosestInt = i64;

//     fn next_power_of_2_exp(self) -> i32 {
//         self.log2().ceil() as i32
//     }
// }

impl NextPowerOfTwo for u32 {
    type ClosestInt = u32;

    fn next_power_of_2_exp(self) -> i32 {
        (self as f32).log2().ceil() as i32
    }
}

// impl NextPowerOfTwo for i32 {
//     type ClosestInt = i32;

//     fn next_power_of_2(self) -> Self::ClosestInt {
//         1 << ((self as f32).log2().ceil() as Self::ClosestInt)
//     }
// }

pub trait Zero {
    fn zero() -> Self;
}

impl<T: num_traits::Zero> Zero for T {
    fn zero() -> T {
        num_traits::Zero::zero()
    }
}

pub trait One {
    fn one() -> Self;
}

impl<T: num_traits::One> One for T {
    fn one() -> T {
        num_traits::One::one()
    }
}
