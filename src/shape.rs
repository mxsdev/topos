use std::{num::NonZeroU64, ops::Range};

use bytemuck::Pod;
use palette::Srgba;
use swash::scale::Render;

use crate::{
    graphics::DynamicGPUQuadBuffer,
    surface::{ParamsBuffer, RenderingContext},
    util::{PhysicalRoundedRect, RoundedRect, WgpuDescriptor},
};

pub struct RenderResources<T: Sized + Pod> {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,

    pub gpu_buffer: DynamicGPUQuadBuffer<T>,
}

impl<T: Sized + Pod> RenderResources<T> {
    pub fn render_all_quads<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: Range<u32>,
    ) {
        self.gpu_buffer.render_all_quads(
            &self.render_pipeline,
            &self.bind_group,
            render_pass,
            instances,
        )
    }
}

pub struct ShapeRenderer {
    box_resources: RenderResources<BoxShaderVertex>,
}

impl ShapeRenderer {
    pub fn new(rendering_context: &RenderingContext) -> Self {
        Self {
            box_resources: Self::create_box_resources(rendering_context),
        }
    }

    // TODO: introduce a "Shape" enum that includes color info
    pub fn prepare_boxes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        boxes: impl ExactSizeIterator<Item = (PhysicalRoundedRect, Srgba)>,
    ) {
        let buf = &mut self.box_resources.gpu_buffer;

        buf.set_num_quads(device, boxes.len() as u64);

        buf.write_all_quads(
            queue,
            boxes.map(|(rect, col)| BoxShaderVertex::from_rrect(rect, col)),
        );
    }

    pub fn render_all_boxes<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.box_resources.render_all_quads(render_pass, 0..1);
    }

    // pub fn box_resources_mut(&mut self) -> &mut RenderResources<BoxShaderVertex> {
    //     &mut self.box_resources
    // }

    // pub fn box_resources(&self) -> &RenderResources<BoxShaderVertex> {
    //     &self.box_resources
    // }

    fn create_box_resources(
        RenderingContext {
            device,
            texture_format,
            params_buffer,
            ..
        }: &RenderingContext,
    ) -> RenderResources<BoxShaderVertex> {
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
            gpu_buffer: DynamicGPUQuadBuffer::new(device),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoxShaderVertex {
    pos: [f32; 2],
    dims: [f32; 2],
    color: [f32; 4],
    rounding: f32,
    depth: f32,
}

impl WgpuDescriptor<5> for BoxShaderVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
        3 => Float32,
        4 => Float32,
    ];
}

impl BoxShaderVertex {
    pub fn from_rrect(rect: PhysicalRoundedRect<f32>, color: palette::Srgba) -> [Self; 4] {
        let dims = rect.center() - rect.max;

        return [
            Self {
                pos: [rect.min.x, rect.min.y],
                dims: [dims.x, dims.y],
                color: color.into(),
                depth: 0.,
                rounding: rect.radius.unwrap_or(0.),
            },
            Self {
                pos: [rect.max.x, rect.min.y],
                dims: [dims.x, dims.y],
                color: color.into(),
                depth: 0.,
                rounding: rect.radius.unwrap_or(0.),
            },
            Self {
                pos: [rect.min.x, rect.max.y],
                dims: [dims.x, dims.y],
                color: color.into(),
                depth: 0.,
                rounding: rect.radius.unwrap_or(0.),
            },
            Self {
                pos: [rect.max.x, rect.max.y],
                dims: [dims.x, dims.y],
                color: color.into(),
                depth: 0.,
                rounding: rect.radius.unwrap_or(0.),
            },
        ];
    }
}
