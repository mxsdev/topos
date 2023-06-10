use std::{fmt::Debug, marker::PhantomData, num::NonZeroU64, ops::Range};

use bytemuck::Pod;
use palette::Srgba;

use crate::{
    atlas::PlacedTextBox,
    graphics::DynamicGPUQuadBuffer,
    surface::{ParamsBuffer, RenderingContext},
    util::{
        CanScale, LogicalToPhysical, LogicalUnit, PhysicalUnit, RoundedBox2D, Translate2DMut,
        WgpuDescriptor,
    },
};

pub struct RenderResources<T: Sized + Pod + Debug> {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,

    pub gpu_buffer: DynamicGPUQuadBuffer<T>,
}

impl<T: Sized + Pod + Debug> RenderResources<T> {
    pub fn render_quads<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        quads: u64,
        instances: Range<u32>,
    ) {
        self.gpu_buffer.render_quads(
            &self.render_pipeline,
            &self.bind_group,
            render_pass,
            quads,
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
        boxes: impl ExactSizeIterator<Item = [BoxShaderVertex; 4]>,
    ) {
        let buf = &mut self.box_resources.gpu_buffer;

        buf.set_num_quads(device, boxes.len() as u64);

        buf.write_all_quads(queue, boxes);
    }

    pub fn render_boxes<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, num_boxes: u64) {
        self.box_resources
            .render_quads(render_pass, num_boxes, 0..1);
    }

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
    stroke_width: f32,
    blur_radius: f32,
}

impl WgpuDescriptor<7> for BoxShaderVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
        3 => Float32,
        4 => Float32,
        5 => Float32,
        6 => Float32,
    ];
}

impl BoxShaderVertex {
    pub fn from_paint_rect(
        paint_rect: PaintRectangle<f32, PhysicalUnit>,
    ) -> (impl Iterator<Item = [Self; 4]>, u64) {
        let fill_rect = paint_rect
            .fill
            .map(|f| Self::from_rect_stroked(paint_rect.rect, f, None, None));

        let stroke_rect =
            paint_rect
                .stroke_color
                .zip(paint_rect.stroke_width)
                .map(|(color, width)| {
                    Self::from_rect_stroked(paint_rect.rect, color, Some(width), None)
                });

        let blur_rect = paint_rect.blur.map(
            |PaintBlur {
                 blur_radius, color, ..
             }| {
                Self::from_rect_stroked(paint_rect.rect, color, None, Some(blur_radius))
            },
        );

        let rects = [blur_rect, fill_rect, stroke_rect];

        let num_rects = rects.iter().filter(|x| x.is_some()).count();

        (rects.into_iter().flatten(), num_rects as u64)
    }

    fn from_rect_stroked(
        rect: RoundedBox2D<f32, PhysicalUnit>,
        color: Srgba,
        stroke_width: Option<f32>,
        blur_radius: Option<f32>,
    ) -> [Self; 4] {
        let dims = rect.max - rect.center();

        let color: [f32; 4] = color.into();
        let stroke_width = stroke_width.unwrap_or(0.);
        let blur_radius = blur_radius.unwrap_or(0.);

        return [
            Self {
                pos: [rect.min.x, rect.min.y],
                dims: [dims.x, dims.y],
                color,
                depth: 0.,
                rounding: rect.radius.unwrap_or(0.),
                stroke_width,
                blur_radius,
            },
            Self {
                pos: [rect.max.x, rect.min.y],
                dims: [dims.x, dims.y],
                color,
                depth: 0.,
                rounding: rect.radius.unwrap_or(0.),
                stroke_width,
                blur_radius,
            },
            Self {
                pos: [rect.min.x, rect.max.y],
                dims: [dims.x, dims.y],
                color,
                depth: 0.,
                rounding: rect.radius.unwrap_or(0.),
                stroke_width,
                blur_radius,
            },
            Self {
                pos: [rect.max.x, rect.max.y],
                dims: [dims.x, dims.y],
                color,
                depth: 0.,
                rounding: rect.radius.unwrap_or(0.),
                stroke_width,
                blur_radius,
            },
        ];
    }
}

pub struct PaintBlur<F: CanScale = f32, U = LogicalUnit> {
    pub blur_radius: F,
    pub color: Srgba,
    _unit: PhantomData<U>,
}

impl<F: CanScale, U> PaintBlur<F, U> {
    pub fn new(blur_radius: F, color: Srgba) -> Self {
        Self {
            blur_radius,
            color,
            _unit: PhantomData,
        }
    }
}

pub struct PaintRectangle<F: CanScale = f32, U = LogicalUnit> {
    pub rect: RoundedBox2D<F, U>,
    pub fill: Option<Srgba>,
    pub stroke_color: Option<Srgba>,
    pub stroke_width: Option<F>,
    pub blur: Option<PaintBlur>,
}

custom_derive! {
    #[derive(EnumFromInner)]
    pub enum PaintShape {
        Rectangle(PaintRectangle),
        Text(PlacedTextBox),
    }
}

// impl Into<PaintShape> for PaintRectangle {
//     fn into(self) -> PaintShape {
//         PaintShape::Rectangle(self)
//     }
// }

impl<F: CanScale> LogicalToPhysical for PaintRectangle<F, LogicalUnit> {
    type PhysicalResult = PaintRectangle<F, PhysicalUnit>;

    fn to_physical(&self, scale_factor: impl CanScale) -> Self::PhysicalResult {
        Self::PhysicalResult {
            fill: self.fill,
            stroke_color: self.stroke_color,
            stroke_width: self.stroke_width.map(|w| w.to_physical(scale_factor)),
            rect: self.rect.to_physical(scale_factor),
            blur: self
                .blur
                .as_ref()
                .map(|b| PaintBlur::new(b.blur_radius.to_physical(scale_factor), b.color)),
        }
    }
}

impl<F: CanScale, U> Translate2DMut<F, U> for PaintRectangle<F, U> {
    fn translate_mut(&mut self, x: F, y: F) {
        self.rect.translate_mut(x, y);
    }
}

impl Translate2DMut<f32, LogicalUnit> for PaintShape {
    fn translate_mut(&mut self, x: f32, y: f32) {
        match self {
            PaintShape::Rectangle(rect) => rect.translate_mut(x, y),
            PaintShape::Text(text_box) => text_box.pos.translate_mut(x, y),
        }
    }
}
