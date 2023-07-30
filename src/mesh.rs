use std::num::NonZeroU64;

use swash::scale;

use crate::{
    color::ColorRgba,
    graphics::DynamicGPUMeshTriBuffer,
    math::{Pos, WindowScaleFactor},
    shape::BoxShaderVertex,
    surface::{ParamsBuffer, RenderingContext, SurfaceDependent},
    util::{svg::PosVertexBuffers, WgpuDescriptor},
};

pub struct PaintMeshVertex {
    pub pos: Pos,
    pub color: ColorRgba,
}

pub struct Mesh<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u16>,
}

pub type PaintMesh = Mesh<PaintMeshVertex>;
pub type GpuMesh = Mesh<BoxShaderVertex>;

impl PaintMesh {
    pub fn as_gpu_mesh(self, scale_fac: WindowScaleFactor) -> GpuMesh {
        Mesh {
            vertices: self
                .vertices
                .into_iter()
                .map(|v| BoxShaderVertex::mesh_tri(v.pos * scale_fac, v.color))
                .collect(),
            indices: self.indices,
        }
    }

    pub fn from_pos_vertex_buffers(
        buffers: PosVertexBuffers,
        color: impl Into<ColorRgba> + Copy,
        pos: Pos,
    ) -> Self {
        Self {
            indices: buffers.indices.clone(),
            vertices: buffers
                .vertices
                .iter()
                .map(|p| PaintMeshVertex {
                    pos: *p + pos.to_vector(),
                    color: color.into().into(),
                })
                .collect(),
        }
    }
}

pub struct MeshRenderer {
    gpu_buffer: DynamicGPUMeshTriBuffer<BoxShaderVertex>,
    bind_group: wgpu::BindGroup,
}

impl MeshRenderer {
    pub fn new(rendering_context: &RenderingContext) -> Self {
        let bind_group = Self::create_resources(rendering_context);

        Self {
            gpu_buffer: DynamicGPUMeshTriBuffer::new(&rendering_context.device),
            bind_group,
        }
    }

    pub fn prepare_meshes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        meshes: impl Iterator<Item = GpuMesh>,
        num_vertices: u64,
        num_indices: u64,
    ) {
        let buf = &mut self.gpu_buffer;

        buf.set_num_verts(device, num_indices, num_vertices);

        buf.write_all_meshes(queue, meshes.into_iter());
    }

    pub fn render_indices<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, num_indices: u64) {
        self.gpu_buffer
            .render_indices(None, &self.bind_group, render_pass, num_indices, 0..1)
    }

    fn create_resources(render_ctx: &RenderingContext) -> wgpu::BindGroup {
        let dummy_texture_view = render_ctx
            .dummy_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_texture_sampler = render_ctx
            .device
            .create_sampler(&wgpu::SamplerDescriptor::default());

        render_ctx.create_shape_bind_group(&dummy_texture_view, &dummy_texture_sampler)
    }
}

impl SurfaceDependent for MeshRenderer {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        _size: winit::dpi::PhysicalSize<u32>,
        _scale_factor: WindowScaleFactor,
    ) {
        let bind_group = Self::create_resources(context);
        self.bind_group = bind_group;
    }
}
