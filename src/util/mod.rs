mod markers;
pub use markers::*;

mod traits;
pub use traits::*;

pub mod math;

pub mod layout;

pub mod taffy;

pub fn min<T: PartialOrd>(x: T, y: T) -> T {
    if x <= y {
        x
    } else {
        y
    }
}

pub fn max<T: PartialOrd>(x: T, y: T) -> T {
    if x >= y {
        x
    } else {
        y
    }
}

// // old impl

// use std::ops::{Deref, DerefMut, Range};

// use self::euclid::{Box2D, Point2D, SideOffsets2D, Size2D, Vector2D};
// use num_traits::{Num, Signed, ToPrimitive};

// use crate::element::boundary::{Boundary, RectLikeBoundary, SDF};

// #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct LogicalUnit;

// #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct PhysicalUnit;

// pub type Rect<F = f32> = euclid::Box2D<F, LogicalUnit>;
// pub type PhysicalRect<F = f32> = euclid::Box2D<F, PhysicalUnit>;

// pub type RoundedRect<F = f32> = RoundedBox2D<F, LogicalUnit>;
// pub type PhysicalRoundedRect<F = f32> = RoundedBox2D<F, PhysicalUnit>;

// pub type Pos2<F = f32> = euclid::Point2D<F, LogicalUnit>;
// pub type PhysicalPos2<F = f32> = euclid::Point2D<F, PhysicalUnit>;

// pub type Vec2<F = f32> = euclid::Vector2D<F, LogicalUnit>;
// pub type PhysicalVec2<F = f32> = euclid::Vector2D<F, PhysicalUnit>;

// pub type Size2<F = f32> = euclid::Size2D<F, LogicalUnit>;
// pub type PhysicalSize2<F = f32> = euclid::Size2D<F, PhysicalUnit>;

// pub trait ToEuclid {
//     type EuclidResult;
//     fn to_euclid(self) -> Self::EuclidResult;
// }

// impl<P> ToEuclid for winit::dpi::LogicalPosition<P> {
//     type EuclidResult = Pos2<P>;

//     fn to_euclid(self) -> Self::EuclidResult {
//         Self::EuclidResult::new(self.x, self.y)
//     }
// }

// impl<P> ToEuclid for winit::dpi::PhysicalPosition<P> {
//     type EuclidResult = PhysicalPos2<P>;

//     fn to_euclid(self) -> Self::EuclidResult {
//         Self::EuclidResult::new(self.x, self.y)
//     }
// }

// impl<P> ToEuclid for winit::dpi::PhysicalSize<P> {
//     type EuclidResult = PhysicalSize2<P>;

//     fn to_euclid(self) -> Self::EuclidResult {
//         Self::EuclidResult::new(self.width, self.height)
//     }
// }

// impl<P> ToEuclid for winit::dpi::LogicalSize<P> {
//     type EuclidResult = Size2<P>;

//     fn to_euclid(self) -> Self::EuclidResult {
//         Self::EuclidResult::new(self.width, self.height)
//     }
// }

// pub trait RoundToInt {
//     type IntegralResult;
//     fn round_to_int(self) -> Self::IntegralResult;
// }

// impl RoundToInt for f32 {
//     type IntegralResult = u32;

//     fn round_to_int(self) -> Self::IntegralResult {
//         self.round().to_u32().unwrap_or_default()
//     }
// }

// impl RoundToInt for f64 {
//     type IntegralResult = u64;

//     fn round_to_int(self) -> Self::IntegralResult {
//         self.round().to_u64().unwrap_or_default()
//     }
// }

// pub trait LogicalToPhysical {
//     type PhysicalResult;
//     fn to_physical(&self, scale_factor: impl CanScale) -> Self::PhysicalResult;
// }

// pub trait LogicalToPhysicalInto {
//     type PhysicalResult;
//     fn to_physical(self, scale_factor: impl CanScale) -> Self::PhysicalResult;
// }

// pub trait PhysicalToLogical {
//     type LogicalResult;
//     fn to_logical(&self, scale_factor: impl CanScale) -> Self::LogicalResult;
// }

// pub trait CanScale: num_traits::Float {
//     fn from_scale_fac(scale_factor: impl CanScale) -> Self;
//     fn as_f32(self) -> f32;
//     fn as_f64(self) -> f64;
// }

// impl CanScale for f64 {
//     fn from_scale_fac(scale_factor: impl CanScale) -> Self {
//         scale_factor.as_f64()
//     }

//     fn as_f32(self) -> f32 {
//         self as f32
//     }

//     fn as_f64(self) -> f64 {
//         self
//     }
// }

// impl CanScale for f32 {
//     fn from_scale_fac(scale_factor: impl CanScale) -> Self {
//         scale_factor.as_f32()
//     }

//     fn as_f32(self) -> f32 {
//         self
//     }

//     fn as_f64(self) -> f64 {
//         self as f64
//     }
// }

// impl<F: CanScale> LogicalToPhysical for F {
//     type PhysicalResult = F;

//     fn to_physical(&self, scale_factor: impl CanScale) -> Self::PhysicalResult {
//         *self * F::from_scale_fac(scale_factor)
//     }
// }

// impl<F: RoundToInt, U> RoundToInt for euclid::Point2D<F, U> {
//     type IntegralResult = euclid::Point2D<F::IntegralResult, U>;

//     fn round_to_int(self) -> Self::IntegralResult {
//         Self::IntegralResult::new(self.x.round_to_int(), self.y.round_to_int())
//     }
// }

// impl<F: CanScale> LogicalToPhysical for Pos2<F> {
//     type PhysicalResult = PhysicalPos2<F>;

//     fn to_physical(&self, scale_factor: impl CanScale) -> Self::PhysicalResult {
//         Self::PhysicalResult::new(
//             self.x.to_physical(scale_factor),
//             self.y.to_physical(scale_factor),
//         )
//     }
// }

// impl<F: RoundToInt, U> RoundToInt for euclid::Vector2D<F, U> {
//     type IntegralResult = euclid::Vector2D<F::IntegralResult, U>;

//     fn round_to_int(self) -> Self::IntegralResult {
//         Self::IntegralResult::new(self.x.round_to_int(), self.y.round_to_int())
//     }
// }

// impl<F: CanScale> LogicalToPhysical for Vec2<F> {
//     type PhysicalResult = PhysicalVec2<F>;

//     fn to_physical(&self, scale_factor: impl CanScale) -> Self::PhysicalResult {
//         Self::PhysicalResult::new(
//             self.x.to_physical(scale_factor),
//             self.y.to_physical(scale_factor),
//         )
//     }
// }

// impl<F: RoundToInt, U> RoundToInt for euclid::Size2D<F, U> {
//     type IntegralResult = euclid::Size2D<F::IntegralResult, U>;

//     fn round_to_int(self) -> Self::IntegralResult {
//         Self::IntegralResult::new(self.width.round_to_int(), self.height.round_to_int())
//     }
// }

// impl<F: CanScale> LogicalToPhysical for Size2<F> {
//     type PhysicalResult = PhysicalSize2<F>;

//     fn to_physical(&self, scale_factor: impl CanScale) -> Self::PhysicalResult {
//         Self::PhysicalResult::new(
//             self.width.to_physical(scale_factor),
//             self.height.to_physical(scale_factor),
//         )
//     }
// }

// impl<F: RoundToInt, U> RoundToInt for euclid::Box2D<F, U> {
//     type IntegralResult = euclid::Box2D<F::IntegralResult, U>;

//     fn round_to_int(self) -> Self::IntegralResult {
//         Self::IntegralResult::new(self.min.round_to_int(), self.max.round_to_int())
//     }
// }

// impl<F: CanScale> LogicalToPhysical for Rect<F> {
//     type PhysicalResult = PhysicalRect<F>;

//     fn to_physical(&self, scale_factor: impl CanScale) -> Self::PhysicalResult {
//         Self::PhysicalResult::new(
//             self.min.to_physical(scale_factor),
//             self.max.to_physical(scale_factor),
//         )
//     }
// }

// impl<F: CanScale> LogicalToPhysical for RoundedRect<F> {
//     type PhysicalResult = PhysicalRoundedRect<F>;

//     fn to_physical(&self, scale_factor: impl CanScale) -> Self::PhysicalResult {
//         Self::PhysicalResult::new(
//             self.rect.to_physical(scale_factor),
//             self.radius.map(|r| r.to_physical(scale_factor)),
//         )
//     }
// }

// impl<F: CanScale> PhysicalToLogical for PhysicalSize2<F> {
//     type LogicalResult = Size2<F>;

//     fn to_logical(&self, scale_factor: impl CanScale) -> Self::LogicalResult {
//         let scale_factor = F::from_scale_fac(scale_factor);
//         Self::LogicalResult::new(self.width / scale_factor, self.height / scale_factor)
//     }
// }

// #[derive(Clone, Debug, Default)]
// pub struct RoundedBox2D<T, U> {
//     pub rect: euclid::Box2D<T, U>,
//     pub radius: Option<T>,
// }

// impl<T: Copy, U: Clone> Copy for RoundedBox2D<T, U> {}

// impl<T, U> RoundedBox2D<T, U> {
//     pub fn new(rect: euclid::Box2D<T, U>, radius: impl Into<Option<T>>) -> Self {
//         Self {
//             rect,
//             radius: radius.into(),
//         }
//     }

//     pub fn with_radius(&self, radius: impl Into<Option<T>>) -> Self
//     where
//         T: Copy,
//     {
//         Self {
//             radius: radius.into(),
//             rect: self.rect,
//         }
//     }
// }

// impl<T, U> RoundedBox2D<T, U>
// where
//     T: Copy + std::ops::Add<T, Output = T> + std::ops::Sub<T, Output = T>,
// {
//     pub fn inflate(&self, width: T, height: T) -> Self {
//         Self {
//             radius: self.radius,
//             rect: self.rect.inflate(width, height),
//         }
//     }

//     pub fn inner_box(&self, offsets: SideOffsets2D<T, U>) -> Self {
//         Self {
//             radius: self.radius,
//             rect: self.rect.inner_box(offsets),
//         }
//     }

//     pub fn outer_box(&self, offsets: SideOffsets2D<T, U>) -> Self {
//         Self {
//             radius: self.radius,
//             rect: self.rect.outer_box(offsets),
//         }
//     }
// }

// impl<T: Num, U> RoundedBox2D<T, U> {
//     pub fn from_rect(rect: euclid::Box2D<T, U>) -> Self {
//         Self { rect, radius: None }
//     }
// }

// impl<T: Num, U> From<euclid::Box2D<T, U>> for RoundedBox2D<T, U> {
//     fn from(rect: euclid::Box2D<T, U>) -> Self {
//         Self::from_rect(rect)
//     }
// }

// impl<T, U> Deref for RoundedBox2D<T, U> {
//     type Target = euclid::Box2D<T, U>;

//     fn deref(&self) -> &Self::Target {
//         &self.rect
//     }
// }

// impl<T, U> DerefMut for RoundedBox2D<T, U> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.rect
//     }
// }

// impl<T: num_traits::Float + Signed, U> SDF<T, U> for RoundedBox2D<T, U> {
//     fn sdf(&self, pos: &euclid::Point2D<T, U>) -> T {
//         match self.radius {
//             Some(radius) => {
//                 let c = self.center();
//                 let b = (self.max - c) - euclid::Vector2D::<T, U>::splat(radius);
//                 let pos = *pos - c;

//                 let q = pos.abs() - b;

//                 -(q.max(euclid::Vector2D::splat(T::zero())).length()
//                     + T::min(T::zero(), T::max(q.x, q.y))
//                     - radius)
//             }

//             None => self.rect.sdf(pos),
//         }
//     }
// }

// impl<T: num_traits::Float + Signed, U> RectLikeBoundary<T, U> for RoundedBox2D<T, U> {
//     fn as_rect(&self) -> euclid::Box2D<T, U> {
//         self.rect
//     }

//     fn set_rect(&mut self, rect: euclid::Box2D<T, U>) {
//         self.rect = rect
//     }
// }

// impl<T: num_traits::Float + Signed, U> SDF<T, U> for euclid::Box2D<T, U> {
//     fn sdf(&self, pos: &euclid::Point2D<T, U>) -> T {
//         let c = self.center();
//         let b = self.max - c;
//         let pos = *pos - c;

//         let q = pos.abs() - b;

//         -(q.max(euclid::Vector2D::splat(T::zero())).length() + T::min(T::zero(), T::max(q.x, q.y)))
//     }
// }

// impl<T: num_traits::Float + Signed, U> RectLikeBoundary<T, U> for euclid::Box2D<T, U> {
//     fn as_rect(&self) -> euclid::Box2D<T, U> {
//         *self
//     }

//     fn set_rect(&mut self, rect: euclid::Box2D<T, U>) {
//         *self = rect
//     }
// }

// pub trait WgpuDescriptor<const N: usize>: Sized {
//     const ATTRIBS: [wgpu::VertexAttribute; N];

//     fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
//         use std::mem;

//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &Self::ATTRIBS,
//         }
//     }
// }

// pub trait AsWinit {
//     type Winit;

//     unsafe fn as_winit(&self) -> &Self::Winit;
// }

// impl AsWinit for winit::monitor::MonitorHandle {
//     type Winit = winit::monitor::MonitorHandle;

//     unsafe fn as_winit(&self) -> &Self::Winit {
//         return std::mem::transmute(self);
//     }
// }

// pub trait Translate2D<F, U>: Sized {
//     fn translate(&self, x: F, y: F) -> Self;

//     fn translate_vec(&self, vec: Vector2D<F, U>) -> Self {
//         self.translate(vec.x, vec.y)
//     }
// }

// pub trait Translate2DMut<F, U> {
//     fn translate_mut(&mut self, x: F, y: F);

//     fn translate_mut_vec(&mut self, vec: Vector2D<F, U>) {
//         self.translate_mut(vec.x, vec.y)
//     }
// }

// impl<F: Num + Copy, U> Translate2DMut<F, U> for euclid::Point2D<F, U> {
//     fn translate_mut(&mut self, x: F, y: F) {
//         self.x = self.x + x;
//         self.y = self.y + y;
//     }
// }

// impl<F: Num + Copy, U> Translate2DMut<F, U> for euclid::Box2D<F, U> {
//     fn translate_mut(&mut self, x: F, y: F) {
//         self.min.translate_mut(x, y);
//         self.max.translate_mut(x, y);
//     }
// }

// impl<F: Num + Copy, U: Clone> Translate2D<F, U> for RoundedBox2D<F, U> {
//     fn translate(&self, x: F, y: F) -> Self {
//         let mut res = *self;
//         res.rect.translate_mut(x, y);
//         res
//     }
// }

// pub trait FromMinSize<F, U> {
//     fn from_min_size(min: Point2D<F, U>, size: Size2D<F, U>) -> Self;
// }

// impl<F: Num + Copy, U> FromMinSize<F, U> for Box2D<F, U> {
//     fn from_min_size(min: Point2D<F, U>, size: Size2D<F, U>) -> Self {
//         Self {
//             min,
//             max: min + size,
//         }
//     }
// }

// pub trait ScaleRange<F> {
//     fn scale(&self, fac: F) -> Self;
// }

// impl<F: Num + Copy> ScaleRange<F> for Range<F> {
//     fn scale(&self, fac: F) -> Self {
//         self.map_range(|x| x * fac)

//         // Self {
//         //     start: self.start * fac,
//         //     end: self.end * fac,
//         // }
//     }
// }

// pub trait MapRange<A, B> {
//     type Result;

//     fn map_range<F>(&self, f: F) -> Self::Result
//     where
//         F: Fn(A) -> B;
// }

// impl<A: Copy, B> MapRange<A, B> for Range<A> {
//     type Result = Range<B>;

//     fn map_range<F>(&self, f: F) -> Self::Result
//     where
//         F: Fn(A) -> B,
//     {
//         Self::Result {
//             start: f(self.start),
//             end: f(self.end),
//         }
//     }
// }

// pub trait IntoTaffy<T>: Sized {
//     /// Converts this type into the (usually inferred) input type.
//     #[must_use]
//     fn into_taffy(self) -> T;
// }

// pub trait IntoGeom<T>: Sized {
//     #[must_use]
//     fn into_geom(self) -> T;
// }

// pub trait AsRect<T>: Sized {
//     #[must_use]
//     fn as_rect(&self) -> T;
// }

// // taffy

// impl IntoTaffy<taffy::geometry::Size<taffy::style::AvailableSpace>> for Size2 {
//     fn into_taffy(self) -> taffy::geometry::Size<taffy::style::AvailableSpace> {
//         taffy::geometry::Size {
//             // TODO: support max-content and min-content
//             height: self.height.into(),
//             width: self.width.into(),
//         }
//     }
// }

// impl IntoTaffy<taffy::geometry::Size<f32>> for Size2 {
//     fn into_taffy(self) -> taffy::geometry::Size<f32> {
//         taffy::geometry::Size {
//             height: self.height,
//             width: self.width,
//         }
//     }
// }

// impl IntoTaffy<taffy::geometry::Size<taffy::style::Dimension>> for Size2 {
//     fn into_taffy(self) -> taffy::geometry::Size<taffy::style::Dimension> {
//         taffy::geometry::Size {
//             height: taffy::style::Dimension::Points(self.height),
//             width: taffy::style::Dimension::Points(self.width),
//         }
//     }
// }

// impl IntoGeom<Size2> for taffy::geometry::Size<f32> {
//     fn into_geom(self) -> Size2 {
//         Size2::new(self.width, self.height)
//     }
// }

// impl IntoGeom<Pos2> for taffy::geometry::Point<f32> {
//     fn into_geom(self) -> Pos2 {
//         Pos2::new(self.x, self.y)
//     }
// }

// impl AsRect<Rect> for taffy::layout::Layout {
//     fn as_rect(&self) -> Rect {
//         Rect::from_min_size(self.location.into_geom(), self.size.into_geom())
//     }
// }

// // lerp
// pub trait Lerp<F> {
//     fn lerp(self, to: F, fac: F) -> F;
// }

// impl<F: Num + Copy> Lerp<F> for F {
//     fn lerp(self, to: F, fac: F) -> F {
//         to * fac + (F::one() - fac) * self
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_rect_sdf() {
//         let rect = Rect::from_size(Size2::new(10., 10.));

//         assert_eq!(rect.sdf(&Pos2::new(1., 1.)), 1.);

//         let theta = (225_f32).to_radians();
//         assert_eq!(rect.sdf(&Pos2::new(theta.cos(), theta.sin())), 1.)
//     }
// }
