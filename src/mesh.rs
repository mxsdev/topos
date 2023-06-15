use crate::util::WgpuDescriptor;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

impl WgpuDescriptor<2> for MeshVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
    ];
}

pub struct PaintMesh {
    vertices: Vec<MeshVertex>,
    indices: Vec<u16>,
}

pub struct MeshRenderer {
    buffer: wgpu::Buffer,
}
