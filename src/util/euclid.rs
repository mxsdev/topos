use std::{marker::PhantomData, ops::Mul};

pub use euclid::*;

use super::traits::{CastUnit, MultiplyNumericFields};

impl<F: Copy + Mul<F, Output = F>, U> MultiplyNumericFields<F> for Size2D<F, U> {
    fn multiply_numeric_fields(self, rhs: F) -> Self {
        Self {
            width: self.width * rhs,
            height: self.height * rhs,
            _unit: PhantomData,
        }
    }
}

impl<F: Copy + Mul<F, Output = F>, U> MultiplyNumericFields<F> for Point2D<F, U> {
    fn multiply_numeric_fields(self, rhs: F) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            _unit: PhantomData,
        }
    }
}

impl<F: Copy + Mul<F, Output = F>, U> MultiplyNumericFields<F> for Vector2D<F, U> {
    fn multiply_numeric_fields(self, rhs: F) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            _unit: PhantomData,
        }
    }
}

impl<F: Copy + Mul<F, Output = F>, U> MultiplyNumericFields<F> for Rect<F, U> {
    fn multiply_numeric_fields(self, rhs: F) -> Self {
        Self {
            origin: self.origin.multiply_numeric_fields(rhs),
            size: self.size.multiply_numeric_fields(rhs),
        }
    }
}

impl<F: Copy + Mul<F, Output = F>, U> MultiplyNumericFields<F> for Box2D<F, U> {
    fn multiply_numeric_fields(self, rhs: F) -> Self {
        Self {
            min: self.min.multiply_numeric_fields(rhs),
            max: self.max.multiply_numeric_fields(rhs),
        }
    }
}

impl<F: Copy + Mul<F, Output = F>, U> MultiplyNumericFields<F> for SideOffsets2D<F, U> {
    fn multiply_numeric_fields(self, rhs: F) -> Self {
        Self {
            top: self.top * rhs,
            right: self.right * rhs,
            left: self.left * rhs,
            bottom: self.bottom * rhs,
            _unit: PhantomData,
        }
    }
}

impl<F, U> CastUnit for SideOffsets2D<F, U> {
    type UnitSelf<Unit> = SideOffsets2D<F, Unit>;

    fn cast_unit<UNew>(self) -> Self::UnitSelf<UNew> {
        Self::UnitSelf::<UNew> {
            top: self.top,
            right: self.right,
            bottom: self.bottom,
            left: self.left,
            _unit: PhantomData,
        }
    }
}
