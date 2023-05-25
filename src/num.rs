use std::ops::Shl;

use num_traits::{Float, One};

pub trait Two {
    const TWO: Self;
}

impl Two for f32 {
    const TWO: Self = 2.;
}

impl Two for f64 {
    const TWO: Self = 2.;
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

pub trait NextPowerOfTwo {
    type ClosestInt;
    fn next_power_of_2(self) -> Self::ClosestInt;
}

impl NextPowerOfTwo for f32 {
    type ClosestInt = i32;

    fn next_power_of_2(self) -> Self::ClosestInt {
        1 << (self.log2().ceil() as Self::ClosestInt)
    }
}

impl NextPowerOfTwo for f64 {
    type ClosestInt = i64;

    fn next_power_of_2(self) -> Self::ClosestInt {
        1 << (self.log2().ceil() as Self::ClosestInt)
    }
}
