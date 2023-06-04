use std::ops::{Deref, DerefMut};

use euclid::{Box2D, Point2D, Size2D, Translation2D};
use num_traits::{Float, Num, Signed};
use swash::scale;

use crate::element::boundary::Boundary;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalUnit;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalUnit;

pub type Rect<F = f32> = euclid::Box2D<F, LogicalUnit>;
pub type PhysicalRect<F = f32> = euclid::Box2D<F, PhysicalUnit>;

pub type RoundedRect<F = f32> = RoundedBox2D<F, LogicalUnit>;
pub type PhysicalRoundedRect<F = f32> = RoundedBox2D<F, PhysicalUnit>;

pub type Pos2<F = f32> = euclid::Point2D<F, LogicalUnit>;
pub type PhysicalPos2<F = f32> = euclid::Point2D<F, PhysicalUnit>;

pub type Vec2<F = f32> = euclid::Vector2D<F, LogicalUnit>;
pub type PhysicalVec2<F = f32> = euclid::Vector2D<F, PhysicalUnit>;

pub type Size2<F = f32> = euclid::Size2D<F, LogicalUnit>;
pub type PhysicalSize2<F = f32> = euclid::Size2D<F, PhysicalUnit>;

pub trait ToEuclid {
    type EuclidResult;
    fn to_euclid(self) -> Self::EuclidResult;
}

impl<P> ToEuclid for winit::dpi::LogicalPosition<P> {
    type EuclidResult = Pos2<P>;

    fn to_euclid(self) -> Self::EuclidResult {
        Self::EuclidResult::new(self.x, self.y)
    }
}

impl<P> ToEuclid for winit::dpi::PhysicalPosition<P> {
    type EuclidResult = PhysicalPos2<P>;

    fn to_euclid(self) -> Self::EuclidResult {
        Self::EuclidResult::new(self.x, self.y)
    }
}

impl<P> ToEuclid for winit::dpi::PhysicalSize<P> {
    type EuclidResult = PhysicalSize2<P>;

    fn to_euclid(self) -> Self::EuclidResult {
        Self::EuclidResult::new(self.width, self.height)
    }
}

impl<P> ToEuclid for winit::dpi::LogicalSize<P> {
    type EuclidResult = Size2<P>;

    fn to_euclid(self) -> Self::EuclidResult {
        Self::EuclidResult::new(self.width, self.height)
    }
}

pub trait LogicalToPhysical {
    type PhysicalResult;
    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult;
}

pub trait PhysicalToLogical {
    type LogicalResult;
    fn to_logical(&self, scale_factor: f64) -> Self::LogicalResult;
}

pub trait CanScale: Float {
    fn from_scale_fac(scale_factor: f64) -> Self;
}

impl CanScale for f64 {
    fn from_scale_fac(scale_factor: f64) -> Self {
        scale_factor
    }
}

impl CanScale for f32 {
    fn from_scale_fac(scale_factor: f64) -> Self {
        scale_factor as Self
    }
}

impl<F: CanScale> LogicalToPhysical for F {
    type PhysicalResult = F;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        *self * F::from_scale_fac(scale_factor)
    }
}

impl<F: CanScale> LogicalToPhysical for Pos2<F> {
    type PhysicalResult = PhysicalPos2<F>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        let scale_factor = F::from_scale_fac(scale_factor);
        Self::PhysicalResult::new(self.x * scale_factor, self.y * scale_factor)
    }
}

impl<F: CanScale> LogicalToPhysical for Vec2<F> {
    type PhysicalResult = PhysicalVec2<F>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        let scale_factor = F::from_scale_fac(scale_factor);
        Self::PhysicalResult::new(self.x * scale_factor, self.y * scale_factor)
    }
}

impl<F: CanScale> LogicalToPhysical for Size2<F> {
    type PhysicalResult = PhysicalSize2<F>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        let scale_factor = F::from_scale_fac(scale_factor);
        Self::PhysicalResult::new(self.width * scale_factor, self.height * scale_factor)
    }
}

impl<F: CanScale> LogicalToPhysical for Rect<F> {
    type PhysicalResult = PhysicalRect<F>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        Self::PhysicalResult::new(
            self.min.to_physical(scale_factor),
            self.max.to_physical(scale_factor),
        )
    }
}

impl<F: CanScale> LogicalToPhysical for RoundedRect<F> {
    type PhysicalResult = PhysicalRoundedRect<F>;

    fn to_physical(&self, scale_factor: f64) -> Self::PhysicalResult {
        Self::PhysicalResult::new(
            self.rect.to_physical(scale_factor),
            self.radius.map(|r| r.to_physical(scale_factor)),
        )
    }
}

impl<F: CanScale> PhysicalToLogical for PhysicalSize2<F> {
    type LogicalResult = Size2<F>;

    fn to_logical(&self, scale_factor: f64) -> Self::LogicalResult {
        let scale_factor = F::from_scale_fac(scale_factor);
        Self::LogicalResult::new(self.width / scale_factor, self.height / scale_factor)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RoundedBox2D<T, U> {
    pub rect: euclid::Box2D<T, U>,
    pub radius: Option<T>,
}

impl<T, U> RoundedBox2D<T, U> {
    pub fn new(rect: euclid::Box2D<T, U>, radius: Option<T>) -> Self {
        Self { rect, radius }
    }
}

impl<T: Num, U> RoundedBox2D<T, U> {
    pub fn from_rect(rect: euclid::Box2D<T, U>) -> Self {
        Self { rect, radius: None }
    }
}

impl<T: Num, U> From<euclid::Box2D<T, U>> for RoundedBox2D<T, U> {
    fn from(rect: euclid::Box2D<T, U>) -> Self {
        Self::from_rect(rect)
    }
}

impl<T, U> Deref for RoundedBox2D<T, U> {
    type Target = euclid::Box2D<T, U>;

    fn deref(&self) -> &Self::Target {
        &self.rect
    }
}

impl<T, U> DerefMut for RoundedBox2D<T, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rect
    }
}

impl<T: Float + Signed, U> Boundary<T, U> for RoundedBox2D<T, U> {
    fn sdf(&self, pos: &euclid::Point2D<T, U>) -> T {
        match self.radius {
            Some(radius) => {
                let c = self.center();
                let b = (self.max - c) - euclid::Vector2D::<T, U>::splat(radius);
                let pos = *pos - c;

                let q = pos.abs() - b;

                -(q.max(euclid::Vector2D::splat(T::zero())).length()
                    + T::min(T::zero(), T::max(q.x, q.y))
                    - radius)
            }

            None => self.rect.sdf(pos),
        }
    }
}

impl<T: Float + Signed, U> Boundary<T, U> for euclid::Box2D<T, U> {
    fn sdf(&self, pos: &euclid::Point2D<T, U>) -> T {
        let c = self.center();
        let b = self.max - c;
        let pos = *pos - c;

        let q = pos.abs() - b;

        -(q.max(euclid::Vector2D::splat(T::zero())).length() + T::min(T::zero(), T::max(q.x, q.y)))
    }
}

pub trait WgpuDescriptor<const N: usize>: Sized {
    const ATTRIBS: [wgpu::VertexAttribute; N];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub trait AsWinit {
    type Winit;

    unsafe fn as_winit(&self) -> &Self::Winit;
}

impl AsWinit for winit::monitor::MonitorHandle {
    type Winit = winit::monitor::MonitorHandle;

    unsafe fn as_winit(&self) -> &Self::Winit {
        return std::mem::transmute(self);
    }
}

pub trait Translate2DMut<F, U> {
    fn translate_mut(&mut self, x: F, y: F);
}

impl<F: Num + Copy, U> Translate2DMut<F, U> for euclid::Point2D<F, U> {
    fn translate_mut(&mut self, x: F, y: F) {
        self.x = self.x + x;
        self.y = self.y + y;
    }
}

impl<F: Num + Copy, U> Translate2DMut<F, U> for euclid::Box2D<F, U> {
    fn translate_mut(&mut self, x: F, y: F) {
        self.min.translate_mut(x, y);
        self.max.translate_mut(x, y);
    }
}

pub trait FromMinSize<F, U> {
    fn from_min_size(min: Point2D<F, U>, size: Size2D<F, U>) -> Self;
}

impl<F: Num + Copy, U> FromMinSize<F, U> for Box2D<F, U> {
    fn from_min_size(min: Point2D<F, U>, size: Size2D<F, U>) -> Self {
        Self {
            min,
            max: min + size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_sdf() {
        let rect = Rect::from_size(Size2::new(10., 10.));

        assert_eq!(rect.sdf(&Pos2::new(1., 1.)), 1.);

        let mut theta = (225.).to_radians();
        assert_eq!(rect.sdf(&Pos2::new(theta.cos(), theta.sin())), 1.)
    }
}
