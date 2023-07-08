use num_traits::{Float, Signed};

use crate::util::{LogicalUnit, Pos, Rect, RoundedRect, Vector};

pub trait Boundary<T: Float = f32, U = LogicalUnit> {
    // might want to think about "on boundary" at some point
    fn is_inside(&self, pos: &Pos<T, U>) -> bool;
}

pub trait SDF<T: Float = f32, U = LogicalUnit> {
    fn sdf(&self, pos: &Pos<T, U>) -> T;
}

impl<U, F: Float, T: SDF<F, U>> Boundary<F, U> for T {
    fn is_inside(&self, pos: &Pos<F, U>) -> bool {
        self.sdf(pos).is_sign_positive()
    }
}

pub trait RectLikeBoundary<T: Float = f32, U = LogicalUnit>: Boundary<T, U> {
    fn as_rect(&self) -> Rect<T, U>;
    fn set_rect(&mut self, rect: Rect<T, U>);
}

pub struct EmptyBoundary;

impl<T: Float, U> SDF<T, U> for EmptyBoundary {
    fn sdf(&self, _pos: &Pos<T, U>) -> T {
        T::neg_infinity()
    }
}

impl<T: num_traits::Float + Signed, U> SDF<T, U> for RoundedRect<T, U> {
    fn sdf(&self, pos: &Pos<T, U>) -> T {
        match self.radius {
            Some(radius) => {
                let c = self.inner.center();
                let b = (self.inner.max - c) - Vector::<T, U>::splat(radius);
                let pos = *pos - c;

                let q = pos.abs() - b;

                -(q.max(Vector::splat(T::zero())).length() + T::min(T::zero(), T::max(q.x, q.y))
                    - radius)
            }

            None => self.inner.sdf(pos),
        }
    }
}

impl<T: num_traits::Float + Signed, U> RectLikeBoundary<T, U> for RoundedRect<T, U> {
    fn as_rect(&self) -> Rect<T, U> {
        self.inner
    }

    fn set_rect(&mut self, rect: Rect<T, U>) {
        self.inner = rect
    }
}

impl<T: num_traits::Float + Signed, U> SDF<T, U> for Rect<T, U> {
    fn sdf(&self, pos: &Pos<T, U>) -> T {
        let c = self.center();
        let b = self.max - c;
        let pos = *pos - c;

        let q = pos.abs() - b;

        -(q.max(Vector::splat(T::zero())).length() + T::min(T::zero(), T::max(q.x, q.y)))
    }
}

impl<T: num_traits::Float + Signed, U> RectLikeBoundary<T, U> for Rect<T, U> {
    fn as_rect(&self) -> Rect<T, U> {
        *self
    }

    fn set_rect(&mut self, rect: Rect<T, U>) {
        *self = rect
    }
}
