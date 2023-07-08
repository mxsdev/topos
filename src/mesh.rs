use std::num::NonZeroU64;

use swash::scale;

use crate::{
    color::ColorRgba,
    graphics::DynamicGPUMeshTriBuffer,
    surface::{ParamsBuffer, RenderingContext, SurfaceDependent},
    util::{Pos, WgpuDescriptor, WindowScaleFactor},
};

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

impl MeshVertex {
    pub fn from_paint_vertex(
        vertex: impl Into<PaintMeshVertex>,
        scale_fac: WindowScaleFactor,
    ) -> Self {
        let vertex = vertex.into();

        Self {
            pos: (vertex.pos * scale_fac).into(),
            color: vertex.color.into(),
        }
    }
}

pub struct PaintMeshVertex {
    pub pos: Pos,
    pub color: ColorRgba,
}

pub struct Mesh<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u16>,
}

pub type PaintMesh = Mesh<PaintMeshVertex>;
pub type GpuMesh = Mesh<MeshVertex>;

impl PaintMesh {
    pub fn as_gpu_mesh(self, scale_fac: WindowScaleFactor) -> GpuMesh {
        Mesh {
            vertices: self
                .vertices
                .into_iter()
                .map(|v| MeshVertex::from_paint_vertex(v, scale_fac))
                .collect(),
            indices: self.indices,
        }
    }
}

pub struct MeshRenderer {
    gpu_buffer: DynamicGPUMeshTriBuffer<MeshVertex>,

    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl MeshRenderer {
    pub fn new(rendering_context: &RenderingContext) -> Self {
        let (render_pipeline, bind_group) = Self::create_resources(rendering_context);

        Self {
            gpu_buffer: DynamicGPUMeshTriBuffer::new(&rendering_context.device),

            render_pipeline,
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
        self.gpu_buffer.render_indices(
            &self.render_pipeline,
            &self.bind_group,
            render_pass,
            num_indices,
            0..1,
        )
    }

    fn create_resources(
        RenderingContext {
            device,
            texture_format,
            params_buffer,
            texture_info,
            ..
        }: &RenderingContext,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
        let shader = device.create_shader_module(wgpu::include_wgsl!("mesh.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mesh bind group"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<ParamsBuffer>() as u64),
                },
                visibility: wgpu::ShaderStages::VERTEX,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh bind pipeline"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[MeshVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: *texture_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
                cull_mode: None,
            },
            depth_stencil: None,
            multisample: texture_info.default_multisample_state(),
            multiview: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh render bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(params_buffer.as_entire_buffer_binding()),
            }],
        });

        (render_pipeline, bind_group)
    }
}

impl SurfaceDependent for MeshRenderer {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        _size: winit::dpi::PhysicalSize<u32>,
        _scale_factor: WindowScaleFactor,
    ) {
        let (render_pipeline, bind_group) = Self::create_resources(context);

        self.render_pipeline = render_pipeline;
        self.bind_group = bind_group;
    }
}
