use num_traits::{Float, Num};

use crate::util::{LogicalUnit, PhysicalUnit};

// pub trait ScaleFactor: Float {
//     fn from_scale_fac(scale_factor: impl ScaleFactor) -> Self;
//     fn as_f32(self) -> f32;
//     fn as_f64(self) -> f64;
// }

// impl ScaleFactor for f64 {
//     fn from_scale_fac(scale_factor: impl ScaleFactor) -> Self {
//         scale_factor.as_f64()
//     }

//     fn as_f32(self) -> f32 {
//         self as f32
//     }

//     fn as_f64(self) -> f64 {
//         self
//     }
// }

// impl ScaleFactor for f32 {
//     fn from_scale_fac(scale_factor: impl ScaleFactor) -> Self {
//         scale_factor.as_f32()
//     }

//     fn as_f32(self) -> f32 {
//         self
//     }

//     fn as_f64(self) -> f64 {
//         self as f64
//     }
// }

// pub trait AsPhysical<SF> {
//     type PhysicalResult;
//     fn as_physical(self, scale_factor: SF) -> Self::PhysicalResult;
// }

// pub trait AsLogical<SF> {
//     type LogicalResult;
//     fn as_logical(self, scale_factor: SF) -> Self::LogicalResult;
// }

pub trait CastUnit {
    type UnitSelf<Unit>;
    fn cast_unit<U>(self) -> Self::UnitSelf<U>;
}

pub trait MultiplyNumericFields<F> {
    fn multiply_numeric_fields(self, rhs: F) -> Self;
}

// impl<SF: ScaleFactor, X: CastUnit + MultiplyNumericFields<SF>> AsPhysical<SF> for X {
//     type PhysicalResult = X::UnitSelf<PhysicalUnit>;

//     fn as_physical(self, scale_factor: SF) -> Self::PhysicalResult {
//         let sf = SF::from_scale_fac(scale_factor);
//         self.multiply_numeric_fields(sf).cast_unit()
//     }
// }

// impl<SF: ScaleFactor, X: CastUnit + MultiplyNumericFields<SF>> AsLogical<SF> for X {
//     type LogicalResult = X::UnitSelf<LogicalUnit>;

//     fn as_logical(self, scale_factor: SF) -> Self::LogicalResult {
//         let sf = SF::one() / SF::from_scale_fac(scale_factor);
//         self.multiply_numeric_fields(sf).cast_unit()
//     }
// }

// #[macro_export]
// macro_rules! impl_euclid_wrapper {
//     ($ident: ident, $euclid: ident) => {
//         impl<F, U> $ident<F, U> {
//             #[inline]
//             pub(super) const fn from_euclid(inner: euclid::$euclid<F, U>) -> Self {
//                 Self { inner }
//             }

//             #[inline(always)]
//             pub(super) fn to_euclid(self) -> euclid::$euclid<F, U> {
//                 self.inner
//             }
//         }

//         impl<F: Copy, U> CastUnit for $ident<F, U> {
//             type UnitSelf<Unit> = $ident<F, Unit>;

//             #[inline]
//             fn cast_unit<NU>(self) -> Self::UnitSelf<NU> {
//                 Self::UnitSelf::<NU> {
//                     inner: self.inner.cast_unit(),
//                 }
//             }
//         }

//         // impl<F: Copy + Float, U> MultiplyNumericFields<F> for $ident<F, U> {
//         //     #[inline]
//         //     fn multiply_numeric_fields(self, rhs: F) -> Self {
//         //         Self {
//         //             inner: self.inner.multiply_numeric_fields(rhs),
//         //         }
//         //     }
//         // }

//         impl<F, U> Into<euclid::$euclid<F, U>> for $ident<F, U> {
//             #[inline(always)]
//             fn into(self) -> euclid::$euclid<F, U> {
//                 self.to_euclid()
//             }
//         }

//         impl<F, U> From<euclid::$euclid<F, U>> for $ident<F, U> {
//             #[inline(always)]
//             fn from(inner: euclid::$euclid<F, U>) -> Self {
//                 Self::from_euclid(inner)
//             }
//         }
//     };
// }

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

// lerp
pub trait Lerp<F> {
    fn lerp(self, to: F, fac: F) -> F;
}

impl<F: Num + Copy> Lerp<F> for F {
    fn lerp(self, to: F, fac: F) -> F {
        to * fac + (F::one() - fac) * self
    }
}
