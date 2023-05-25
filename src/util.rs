use num_traits::{Float, Num, NumCast};
use tao::dpi::PhysicalSize;

// use crate::num::{Infty, Two};

// #[derive(Clone, Copy)]
// pub struct Size2<P: Num = f32> {
//     pub x: P,
//     pub y: P,
// }

// impl<P: Num + Copy> Size2<P> {
//     pub fn new(x: P, y: P) -> Self {
//         Self { x, y }
//     }

//     pub fn to_physical<R: From<P> + Num>(&self, scale_factor: impl Into<P>) -> PhysicalSize2<R> {
//         let scale_factor = scale_factor.into();

//         PhysicalSize2 {
//             x: (scale_factor * self.x).into(),
//             y: (scale_factor * self.y).into(),
//         }
//     }

//     pub fn scale(&self, fac: P) -> Self {
//         Self {
//             x: self.x * fac,
//             y: self.y * fac,
//         }
//     }

//     pub fn div(&self, fac: P) -> Self {
//         Self {
//             x: self.x / fac,
//             y: self.y / fac,
//         }
//     }
// }

// #[derive(Clone, Copy)]
// pub struct Pos2<P: Num = f32> {
//     pub x: P,
//     pub y: P,
// }

// impl<P: Num + Copy> Pos2<P> {
//     pub const fn new(x: P, y: P) -> Self {
//         Self { x, y }
//     }

//     pub const fn splat(p: P) -> Self {
//         Self::new(p, p)
//     }

//     pub fn to_physical<R: From<P> + Num>(&self, scale_factor: impl Into<P>) -> PhysicalPos2<R> {
//         let scale_factor = scale_factor.into();

//         PhysicalPos2 {
//             x: (scale_factor * self.x).into(),
//             y: (scale_factor * self.y).into(),
//         }
//     }

//     pub fn scale(&self, fac: P) -> Self {
//         Self {
//             x: self.x * fac,
//             y: self.y * fac,
//         }
//     }

//     pub fn div(&self, fac: P) -> Self {
//         Self {
//             x: self.x / fac,
//             y: self.y / fac,
//         }
//     }
// }

// #[derive(Clone, Copy)]
// pub struct Rect<P: Num = f32> {
//     pub top_left: Pos2<P>,
//     pub bottom_right: Pos2<P>,
// }

// impl<P: Num + Copy> Rect<P> {
//     fn new(top_left: Pos2<P>, bottom_right: Pos2<P>) -> Self {
//         Self {
//             top_left,
//             bottom_right,
//         }
//     }

//     pub fn size(&self) -> Size2<P> {
//         self.bottom_right - self.top_left
//     }

//     pub fn top_left(&self) -> Pos2<P> {
//         self.top_left
//     }

//     pub fn bottom_right(&self) -> Pos2<P> {
//         self.bottom_right
//     }

//     pub fn top_right(&self) -> Pos2<P> {
//         Pos2::new(self.bottom_right.x, self.top_left.y)
//     }

//     pub fn bottom_left(&self) -> Pos2<P> {
//         Pos2::new(self.top_left.x, self.bottom_right.y)
//     }

//     pub fn left(&self) -> P {
//         self.top_left.x
//     }

//     pub fn right(&self) -> P {
//         self.bottom_right.x
//     }

//     pub fn top(&self) -> P {
//         self.top_left.y
//     }

//     pub fn bottom(&self) -> P {
//         self.bottom_right.y
//     }

//     pub fn to_physical<R: From<P> + Num>(&self, scale_factor: impl Into<P>) -> PhysicalRect<R> {
//         let scale_factor = scale_factor.into();

//         PhysicalRect {
//             top_left: self.top_left.to_physical(scale_factor),
//             bottom_right: self.bottom_right.to_physical(scale_factor),
//         }
//     }
// }

// impl<P: Num + Copy + PartialOrd> Rect<P> {
//     fn intersection(&self, other: &Rect<P>) -> Rect<P> {
//         todo!()
//     }

//     fn is_within(&self, other: &Rect<P>) -> bool {
//         return self.left() >= other.left()
//             && self.right() <= other.right()
//             && self.top() >= other.top()
//             && self.bottom() <= other.bottom();
//     }

//     fn is_within_strict(&self, other: &Rect<P>) -> bool {
//         return self.left() > other.left()
//             && self.right() < other.right()
//             && self.top() > other.top()
//             && self.bottom() < other.bottom();
//     }
// }

// impl<P: Num + Two + Copy> Rect<P> {
//     fn origin(self) -> Pos2<P> {
//         (self.top_left + self.bottom_right).div(P::TWO)
//     }
// }

// impl<P: Infty + Num + Copy> Rect<P> {
//     const NOTHING: Self = Self {
//         bottom_right: Pos2::splat(P::NEG_INFINITY),
//         top_left: Pos2::splat(P::INFINITY),
//     };

//     const EVERYTHING: Self = Self {
//         bottom_right: Pos2::splat(P::INFINITY),
//         top_left: Pos2::splat(P::NEG_INFINITY),
//     };
// }

// pub struct PhysicalPos2<P: Num = f32> {
//     pub x: P,
//     pub y: P,
// }

// pub struct PhysicalSize2<P: Num = f32> {
//     pub x: P,
//     pub y: P,
// }

// pub struct PhysicalRect<P: Num = f32> {
//     pub top_left: PhysicalPos2<P>,
//     pub bottom_right: PhysicalPos2<P>,
// }

// impl<P: Num> std::ops::Add<Size2<P>> for Pos2<P> {
//     type Output = Pos2<P>;

//     fn add(self, rhs: Size2<P>) -> Self::Output {
//         Self::Output {
//             x: self.x + rhs.x,
//             y: self.y + rhs.y,
//         }
//     }
// }

// impl<P: Num> std::ops::Sub<Size2<P>> for Pos2<P> {
//     type Output = Pos2<P>;

//     fn sub(self, rhs: Size2<P>) -> Self::Output {
//         Self::Output {
//             x: self.x - rhs.x,
//             y: self.y - rhs.y,
//         }
//     }
// }

// impl<P: Num> std::ops::Add<Pos2<P>> for Pos2<P> {
//     type Output = Pos2<P>;

//     fn add(self, rhs: Pos2<P>) -> Self::Output {
//         Self::Output {
//             x: self.x + rhs.x,
//             y: self.y + rhs.y,
//         }
//     }
// }

// impl<P: Num> std::ops::Sub<Pos2<P>> for Pos2<P> {
//     type Output = Size2<P>;

//     fn sub(self, rhs: Pos2<P>) -> Self::Output {
//         Self::Output {
//             x: self.x - rhs.x,
//             y: self.y - rhs.y,
//         }
//     }
// }

// #[repr(u32)]
// #[derive(Clone, Copy, Eq, PartialEq)]
// pub(crate) enum DrawUnit {
//     Logical = 0,
//     Physical = 1,
// }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalUnit;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalUnit;

pub type Rect<F: Float = f32> = euclid::Box2D<F, LogicalUnit>;
pub type PhysicalRect<F: Float = f32> = euclid::Box2D<F, PhysicalUnit>;

pub type Pos2<F: Float = f32> = euclid::Point2D<F, LogicalUnit>;
pub type PhysicalPos2<F: Float = f32> = euclid::Point2D<F, PhysicalUnit>;

pub type Vec2<F: Float = f32> = euclid::Vector2D<F, LogicalUnit>;

pub type Size2<F: Float = f32> = euclid::Size2D<F, LogicalUnit>;

trait LogicalToPhysical {
    type PhysicalResult;
    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult;
}

impl LogicalToPhysical for Pos2<f32> {
    type PhysicalResult = PhysicalPos2<f32>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        let scale_factor = scale_factor as f32;
        Self::PhysicalResult::new(self.x * scale_factor, self.y * scale_factor)
    }
}

impl LogicalToPhysical for Pos2<f64> {
    type PhysicalResult = PhysicalPos2<f64>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        Self::PhysicalResult::new(self.x * scale_factor, self.y * scale_factor)
    }
}

// impl<F> LogicalToPhysical for Rect<F> {
//     type PhysicalResult = PhysicalRect<F>;

//     fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
//         Self::PhysicalResult::new(
//             self.min.to_physical(scale_factor),
//             self.max.to_physical(scale_factor),
//         )
//     }
// }

impl LogicalToPhysical for Rect<f32> {
    type PhysicalResult = PhysicalRect<f32>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        Self::PhysicalResult::new(
            self.min.to_physical(scale_factor),
            self.max.to_physical(scale_factor),
        )
    }
}

impl LogicalToPhysical for Rect<f64> {
    type PhysicalResult = PhysicalRect<f64>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        Self::PhysicalResult::new(
            self.min.to_physical(scale_factor),
            self.max.to_physical(scale_factor),
        )
    }
}
