use std::ops::{Deref, DerefMut};

use num_traits::{Float, Num, Signed};

use crate::element::boundary::Boundary;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalUnit;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalUnit;

pub type Rect<F = f32> = euclid::Box2D<F, LogicalUnit>;
pub type PhysicalRect<F = f32> = euclid::Box2D<F, PhysicalUnit>;

pub type RoundedRect<F = f32> = euclid::Box2D<F, LogicalUnit>;
pub type PhysicalRoundedRect<F = f32> = euclid::Box2D<F, PhysicalUnit>;

pub type Pos2<F = f32> = euclid::Point2D<F, LogicalUnit>;
pub type PhysicalPos2<F = f32> = euclid::Point2D<F, PhysicalUnit>;

pub type Vec2<F = f32> = euclid::Vector2D<F, LogicalUnit>;
pub type PhysicalVec2<F = f32> = euclid::Vector2D<F, PhysicalUnit>;

pub type Size2<F = f32> = euclid::Size2D<F, LogicalUnit>;
pub type PhysicalSize2<F = f32> = euclid::Size2D<F, PhysicalUnit>;

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

#[derive(Clone, Copy, Debug)]
pub struct RoundedBox2D<T, U> {
    pub rect: euclid::Box2D<T, U>,
    pub radius: T,
}

impl<T, U> RoundedBox2D<T, U> {
    pub fn new(rect: euclid::Box2D<T, U>, radius: T) -> Self {
        Self { rect, radius }
    }
}

impl<T: Num, U> RoundedBox2D<T, U> {
    pub fn from_rect(rect: euclid::Box2D<T, U>) -> Self {
        Self {
            rect,
            radius: T::zero(),
        }
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

impl<T: Float, U> Boundary<T, U> for euclid::Box2D<T, U> {
    fn sdf(&self, pos: euclid::Point2D<T, U>) -> T {
        let c = self.center();
        let b = self.max - c;
        let pos = pos - c;

        let q = euclid::Vector2D::splat(pos.length()) - b;

        q.max(euclid::Vector2D::splat(T::zero())).length() + T::min(T::zero(), T::max(q.x, q.y))
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
