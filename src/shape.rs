use std::num::NonZeroU64;

use swash::scale::Render;

use crate::{
    surface::{ParamsBuffer, RenderingContext},
    util::WgpuDescriptor,
};

struct RenderResources {
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

pub struct ShapeRenderer {
    box_resources: RenderResources,
}

impl ShapeRenderer {
    pub fn new(rendering_context: &RenderingContext) -> Self {
        Self {
            box_resources: Self::box_resources(rendering_context),
        }
    }

    fn box_resources(
        RenderingContext {
            device,
            texture_format,
            params_buffer,
            ..
        }: &RenderingContext,
    ) -> RenderResources {
        let shader = device.create_shader_module(wgpu::include_wgsl!("box.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("box bind group"),
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
            label: Some("box bind pipeline"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("box render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[BoxShaderVertex::desc()],
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("box render bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(params_buffer.as_entire_buffer_binding()),
            }],
        });

        RenderResources {
            render_pipeline,
            bind_group,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BoxShaderVertex {
    pos: [f32; 2],
    color: [f32; 4],
    rounding: f32,
    depth: f32,
}

impl WgpuDescriptor<4> for BoxShaderVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2,
        2 => Float32x4,
        3 => Float32,
        4 => Float32,
    ];
}
