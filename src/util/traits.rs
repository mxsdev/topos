use num_traits::Num;

pub trait CastUnit {
    type UnitSelf<Unit>;
    fn cast_unit<U>(self) -> Self::UnitSelf<U>;
}

pub trait MultiplyNumericFields<F> {
    fn multiply_numeric_fields(self, rhs: F) -> Self;
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

// lerp
pub trait Lerp<F> {
    fn lerp(self, to: F, fac: F) -> F;
}

impl<F: Num + Copy> Lerp<F> for F {
    fn lerp(self, to: F, fac: F) -> F {
        to * fac + (F::one() - fac) * self
    }
}
