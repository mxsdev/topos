use serde::Serialize;

use crate::{
    color::ColorRgba,
    graphics::{DynamicGPUBuffer, DynamicGPUMeshTriBuffer, Mesh, PushVertices, VertexBuffers},
    math::{CoordinateTransform, PhysicalPos, PhysicalRect, Pos, RoundedRect, ScaleFactor},
    surface::ParamsBuffer,
    texture::{TextureManagerRef, TextureRef},
    util::{
        svg::PosVertexBuffers,
        template::{HandlebarsTemplater, Templater},
        text::{GlyphContentType, PlacedTextBox},
    },
};

use std::{
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroU64,
    ops::{Mul, Range},
    sync::atomic::Ordering,
};

use num_traits::{Float, Num};
use wgpu::{BufferUsages, ShaderModuleDescriptor};

use crate::{
    num::{MaxNum, Two},
    surface::RenderingContext,
    util::{math::Rect, LogicalUnit, PhysicalUnit, WgpuDescriptor},
};

pub struct ShapeRenderer {
    pub shape_bind_group_layout: wgpu::BindGroupLayout,
    pub shape_render_pipeline: wgpu::RenderPipeline,
    pub shape_bind_group: wgpu::BindGroup,

    // shader storage
    clip_rects: DynamicGPUBuffer<ShaderClipRect>,
    transformations: DynamicGPUBuffer<CoordinateTransform>,
    transformation_inversions: DynamicGPUBuffer<CoordinateTransform>,

    // vertex buffers
    shape_buffer: DynamicGPUMeshTriBuffer<BoxShaderVertex>,
}

impl ShapeRenderer {
    pub fn new(rendering_context: &RenderingContext, texture_manager: &TextureManagerRef) -> Self {
        let RenderingContext {
            params_buffer,
            device,
            texture_format,
            texture_info,
            ..
        } = rendering_context;

        let shape_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("box bind group"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        count: None,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(
                                std::mem::size_of::<ParamsBuffer>() as u64
                            ),
                        },
                        visibility: wgpu::ShaderStages::VERTEX,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        count: None,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        visibility: wgpu::ShaderStages::FRAGMENT,
                    },
                    // TODO: combine these two as one buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        count: None,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        visibility: wgpu::ShaderStages::VERTEX,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        count: None,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        visibility: wgpu::ShaderStages::FRAGMENT,
                    },
                ],
            });

        let num_atlas_textures = texture_manager.read().unwrap().get_max_textures();

        let shape_shader_module = {
            #[derive(Serialize)]
            struct TemplateData {
                num_atlas_textures: u32,
            }

            let shader_template_src = include_str!("box.wgsl");
            let templater = HandlebarsTemplater::new(TemplateData { num_atlas_textures });

            let shader_src = templater.render_template(shader_template_src).unwrap();

            device.create_shader_module(ShaderModuleDescriptor {
                label: Some("box shader"),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            })
        };

        let shape_render_pipeline_layout = {
            let texture_manager = texture_manager.read().unwrap();

            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("box pipeline layout"),
                bind_group_layouts: &[
                    &shape_bind_group_layout,
                    texture_manager.get_texture_bind_group_layout(),
                    texture_manager.get_sampler_bind_group_layout(),
                ],
                push_constant_ranges: &[],
            })
        };

        let shape_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("box render pipeline"),
                layout: Some(&shape_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shape_shader_module,
                    entry_point: "vs_main",
                    buffers: &[BoxShaderVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shape_shader_module,
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
                multisample: wgpu::MultisampleState {
                    count: texture_info.get_num_samples(),
                    ..Default::default()
                },
                multiview: None,
            });

        let shader_storage_caps =
            BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST;

        let clip_rects = DynamicGPUBuffer::new(device, 4, shader_storage_caps);
        let transformations = DynamicGPUBuffer::new(device, 4, shader_storage_caps);
        let transformation_inversions = DynamicGPUBuffer::new(device, 4, shader_storage_caps);

        let shape_bind_group = Self::create_bind_group(
            device,
            &shape_bind_group_layout,
            params_buffer,
            &clip_rects.buffer,
            &transformations.buffer,
            &transformation_inversions.buffer,
        );

        let shape_buffer = DynamicGPUMeshTriBuffer::new(device);

        Self {
            shape_bind_group,
            shape_bind_group_layout,
            shape_render_pipeline,

            clip_rects,
            transformations,
            transformation_inversions,

            shape_buffer,
        }
    }

    fn create_bind_group(
        device: &wgpu::Device,
        shape_bind_group_layout: &wgpu::BindGroupLayout,
        params_buffer: &wgpu::Buffer,
        clip_rects_buffer: &wgpu::Buffer,
        transformations_buffer: &wgpu::Buffer,
        transformation_inversions_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("box bind group"),
            layout: shape_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: clip_rects_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: transformations_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: transformation_inversions_buffer.as_entire_binding(),
                },
            ],
        })
    }

    pub fn write_all_shapes(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        buffers: &VertexBuffers<BoxShaderVertex>,
    ) {
        self.shape_buffer.write_all(queue, device, buffers)
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: Range<u32>) {
        self.shape_buffer.render(render_pass, instances)
    }

    pub fn write_all_clip_rects(
        &mut self,
        RenderingContext {
            device,
            queue,
            params_buffer,
            ..
        }: &RenderingContext,
        clip_rects: &[ShaderClipRect],
    ) {
        if self.clip_rects.write(device, queue, clip_rects) {
            self.shape_bind_group = Self::create_bind_group(
                device,
                &self.shape_bind_group_layout,
                params_buffer,
                &self.clip_rects.buffer,
                &self.transformations.buffer,
                &self.transformation_inversions.buffer,
            );
        }
    }

    pub fn write_all_transformations(
        &mut self,
        RenderingContext {
            device,
            queue,
            params_buffer,
            ..
        }: &RenderingContext,
        transformations: &[CoordinateTransform],
        transformation_inversions: &[CoordinateTransform],
    ) {
        if self.transformations.write(device, queue, transformations)
            || self
                .transformation_inversions
                .write(device, queue, transformations)
        {
            self.shape_bind_group = Self::create_bind_group(
                device,
                &self.shape_bind_group_layout,
                params_buffer,
                &self.clip_rects.buffer,
                &self.transformations.buffer,
                &self.transformation_inversions.buffer,
            );
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, Default)]
pub enum ShapeType {
    Rectangle = 0,
    #[default]
    Mesh = 1,
}

unsafe impl bytemuck::Zeroable for ShapeType {}
unsafe impl bytemuck::Pod for ShapeType {}

#[repr(u32)]
#[derive(Copy, Clone, Debug, Default)]
pub enum FillMode {
    #[default]
    Color,
    Texture,
    TextureMaskColor,
}

unsafe impl bytemuck::Zeroable for FillMode {}
unsafe impl bytemuck::Pod for FillMode {}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoxShaderVertex {
    shape_type: ShapeType,
    fill_mode: FillMode,

    depth: f32,

    pos: [f32; 2],

    dims: [f32; 2],
    origin: [f32; 2],

    uv: [u32; 2],
    atlas_idx: u32,

    color: [f32; 4],

    rounding: f32,
    stroke_width: f32,
    blur_radius: f32,

    clip_rect_idx: u32,

    transform_idx: u32,
}

impl WgpuDescriptor<14> for BoxShaderVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 14] = wgpu::vertex_attr_array![
        // shape_type
        0 => Uint32,
        // fill_mode
        1 => Uint32,

        // depth
        2 => Float32,

        // pos
        3 => Float32x2,

        // dims
        4 => Float32x2,
        // origin
        5 => Float32x2,

        // uv
        6 => Uint32x2,
        // atlas_idx
        7 => Uint32,

        // color
        8 => Float32x4,

        // rounding
        9 => Float32,
        // stroke_width
        10 => Float32,
        // blur_radius
        11 => Float32,

        // clip_rect_idx
        12 => Uint32,

        // transform_idx
        13 => Uint32,
    ];
}

impl BoxShaderVertex {
    pub fn from_paint_rect(
        paint_rect: PaintRectangle<f32, PhysicalUnit>,
    ) -> (impl Iterator<Item = [Self; 4]>, u64) {
        let fill_rect = paint_rect
            .fill
            .map(|f| Self::from_rect_stroked(paint_rect.rounded_rect, f, None, None));

        let stroke_rect =
            paint_rect
                .stroke_color
                .zip(paint_rect.stroke_width)
                .map(|(color, width)| {
                    Self::from_rect_stroked(paint_rect.rounded_rect, color, Some(width), None)
                });

        let blur_rect = paint_rect.blur.map(
            |PaintBlur {
                 blur_radius, color, ..
             }| {
                Self::from_rect_stroked(paint_rect.rounded_rect, color, None, Some(blur_radius))
            },
        );

        let rects = [blur_rect, fill_rect, stroke_rect];

        let num_rects = rects.iter().filter(|x| x.is_some()).count();

        (rects.into_iter().flatten(), num_rects as u64)
    }

    pub(crate) fn mesh_tri(pos: PhysicalPos, color: ColorRgba) -> Self {
        Self {
            shape_type: ShapeType::Mesh,
            fill_mode: FillMode::Color,
            pos: [pos.x, pos.y],
            color: color.into(),
            ..Default::default()
        }
    }

    pub(crate) fn glyph_rect(
        rect: Rect<f32, PhysicalUnit>,
        uv: Rect<u32, PhysicalUnit>,
        glyph_type: GlyphContentType, // TODO: texture id
        color: ColorRgba,
        texture_ref: &TextureRef,
    ) -> ([Self; 4], [u16; 6]) {
        let color: [f32; 4] = color.into();

        let binding_idx = texture_ref.binding_idx.load(Ordering::Relaxed);

        let fill_mode = match glyph_type {
            GlyphContentType::Color => FillMode::Texture,
            GlyphContentType::Mask => FillMode::TextureMaskColor,
        };

        return (
            [
                Self {
                    shape_type: ShapeType::Mesh,
                    fill_mode,
                    pos: [rect.min.x, rect.min.y],
                    uv: [uv.min.x, uv.min.y],
                    color,
                    atlas_idx: binding_idx,
                    ..Default::default()
                },
                Self {
                    shape_type: ShapeType::Mesh,
                    fill_mode,
                    pos: [rect.max.x, rect.min.y],
                    uv: [uv.max.x, uv.min.y],
                    color,
                    atlas_idx: binding_idx,
                    ..Default::default()
                },
                Self {
                    shape_type: ShapeType::Mesh,
                    fill_mode,
                    pos: [rect.min.x, rect.max.y],
                    uv: [uv.min.x, uv.max.y],
                    color,
                    atlas_idx: binding_idx,
                    ..Default::default()
                },
                Self {
                    shape_type: ShapeType::Mesh,
                    fill_mode,
                    pos: [rect.max.x, rect.max.y],
                    uv: [uv.max.x, uv.max.y],
                    color,
                    atlas_idx: binding_idx,
                    ..Default::default()
                },
            ],
            [0, 1, 2, 1, 2, 3],
        );
    }

    fn from_rect_stroked(
        rounded_rect: RoundedRect<f32, PhysicalUnit>,
        color: ColorRgba,
        stroke_width: Option<f32>,
        blur_radius: Option<f32>,
    ) -> [Self; 4] {
        let RoundedRect {
            inner: rect,
            radius,
        } = rounded_rect;

        let origin = rect.center();

        let dims = rect.max - origin;

        let color: [f32; 4] = color.into();
        let stroke_width = stroke_width.unwrap_or(0.);
        let blur_radius = blur_radius.unwrap_or(0.);

        let origin = origin.into();

        return [
            Self {
                shape_type: ShapeType::Rectangle,
                fill_mode: FillMode::Color,
                origin,
                pos: [rect.min.x, rect.min.y],
                dims: [dims.x, dims.y],
                color,
                depth: 0.,
                rounding: radius.unwrap_or(0.),
                stroke_width,
                blur_radius,
                ..Default::default()
            },
            Self {
                shape_type: ShapeType::Rectangle,
                fill_mode: FillMode::Color,
                origin,
                pos: [rect.max.x, rect.min.y],
                dims: [dims.x, dims.y],
                color,
                depth: 0.,
                rounding: radius.unwrap_or(0.),
                stroke_width,
                blur_radius,
                ..Default::default()
            },
            Self {
                shape_type: ShapeType::Rectangle,
                fill_mode: FillMode::Color,
                origin,
                pos: [rect.min.x, rect.max.y],
                dims: [dims.x, dims.y],
                color,
                depth: 0.,
                rounding: radius.unwrap_or(0.),
                stroke_width,
                blur_radius,
                ..Default::default()
            },
            Self {
                shape_type: ShapeType::Rectangle,
                fill_mode: FillMode::Color,
                origin,
                pos: [rect.max.x, rect.max.y],
                dims: [dims.x, dims.y],
                color,
                depth: 0.,
                rounding: radius.unwrap_or(0.),
                stroke_width,
                blur_radius,
                ..Default::default()
            },
        ];
    }
}

pub type ClipRect<F = f32, U = LogicalUnit> = RoundedRect<F, U>;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderClipRect {
    origin: [f32; 2],
    half_size: [f32; 2],
    rounding: f32,
    transformation_idx: u32,
}

impl ShaderClipRect {
    pub fn new(rect: PhysicalRect, rounding: f32, transformation_idx: u32) -> Self {
        let origin = rect.center().to_vector();
        let half_size = rect.max - origin;

        Self {
            origin: origin.into(),
            half_size: half_size.into(),
            rounding,
            transformation_idx,
        }
    }

    pub fn from_clip_rect(rect: ClipRect<f32, PhysicalUnit>, transformation_idx: u32) -> Self {
        Self::new(rect.inner, rect.radius.unwrap_or(0.), transformation_idx)
    }
}

#[derive(Clone, Default, Debug)]
pub struct PaintBlur<F = f32, U = LogicalUnit> {
    pub blur_radius: F,
    pub color: ColorRgba,
    _unit: PhantomData<U>,
}

impl<F: Float, U> PaintBlur<F, U> {
    pub fn new(blur_radius: F, color: ColorRgba) -> Self {
        Self {
            blur_radius,
            color,
            _unit: PhantomData,
        }
    }
}

impl<T: Copy + Mul, U1, U2> Mul<ScaleFactor<T, U1, U2>> for PaintBlur<T, U1> {
    type Output = PaintBlur<T::Output, U2>;

    #[inline]
    fn mul(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        Self::Output {
            blur_radius: self.blur_radius * scale.get(),
            color: self.color,
            _unit: PhantomData,
        }
    }
}

// TODO: adopt builder pattern (with `impl` args)
#[derive(Clone, Default, Debug)]
pub struct PaintRectangle<F = f32, U = LogicalUnit> {
    pub rounded_rect: RoundedRect<F, U>,
    pub fill: Option<ColorRgba>,
    pub stroke_color: Option<ColorRgba>,
    pub stroke_width: Option<F>,
    pub blur: Option<PaintBlur<F, U>>,
}

impl<F, U> PaintRectangle<F, U> {
    pub fn from_rect(rect: impl Into<RoundedRect<F, U>>) -> Self
    where
        F: Default,
        U: Default,
    {
        Self {
            rounded_rect: rect.into(),
            ..Default::default()
        }
    }

    #[inline]
    pub fn with_rect(mut self, rect: impl Into<Rect<F, U>>) -> Self {
        self.rounded_rect.inner = rect.into();
        self
    }

    #[inline]
    pub fn with_rounding(mut self, radius: impl Into<F>) -> Self {
        self.rounded_rect.radius = radius.into().into();
        self
    }

    pub fn without_rounding(mut self) -> Self {
        self.rounded_rect.radius = None;
        self
    }

    #[inline]
    pub fn with_rounded_rect(mut self, rounded_rect: impl Into<RoundedRect<F, U>>) -> Self {
        self.rounded_rect = rounded_rect.into();
        self
    }

    #[inline]
    pub fn with_fill(mut self, fill_color: impl Into<ColorRgba>) -> Self {
        self.fill = fill_color.into().into();
        self
    }

    #[inline]
    pub fn without_fill(mut self) -> Self {
        self.fill = None;
        self
    }

    #[inline]
    pub fn with_stroke_color(mut self, stroke_color: impl Into<ColorRgba>) -> Self {
        self.stroke_color = stroke_color.into().into();
        self
    }

    #[inline]
    pub fn with_stroke_width(mut self, stroke_width: impl Into<F>) -> Self {
        self.stroke_width = stroke_width.into().into();
        self
    }

    #[inline]
    pub fn with_stroke(
        self,
        stroke_color: impl Into<ColorRgba>,
        stroke_width: impl Into<F>,
    ) -> Self {
        self.with_stroke_width(stroke_width)
            .with_stroke_color(stroke_color)
    }

    pub fn without_stroke(mut self) -> Self {
        self.stroke_color = None;
        self.stroke_width = None;
        self
    }

    #[inline]
    pub fn with_blur(mut self, radius: impl Into<F>, color: impl Into<ColorRgba>) -> Self
    where
        F: Float,
    {
        self.blur = Some(PaintBlur::new(radius.into(), color.into()));
        self
    }

    #[inline]
    pub fn with_blur_radius(mut self, blur: impl Into<F>) -> Self
    where
        F: Default,
        U: Default,
    {
        self.blur.get_or_insert_with(Default::default).blur_radius = blur.into();
        self
    }

    #[inline]
    pub fn with_blur_color(mut self, color: impl Into<ColorRgba>) -> Self
    where
        F: Default,
        U: Default,
    {
        self.blur.get_or_insert_with(Default::default).color = color.into();
        self
    }

    pub fn without_blur(mut self) -> Self {
        self.blur = None;
        self
    }
}

#[derive(Debug)]
pub struct PaintMeshVertex {
    pub pos: Pos,
    pub color: ColorRgba,
}

pub type PaintMesh = Mesh<PaintMeshVertex>;

impl PaintMesh {
    pub fn from_pos_vertex_buffers(
        vertex_buffers: &PosVertexBuffers,
        color: impl Into<ColorRgba>,
        offset: Pos,
    ) -> Self {
        let color = color.into();

        Self {
            vertices: vertex_buffers
                .vertices
                .iter()
                .map(|pos| PaintMeshVertex {
                    pos: *pos + offset.to_vector(),
                    color,
                })
                .collect(),
            indices: vertex_buffers.indices.clone(),
        }
    }
}

custom_derive! {
    #[derive(EnumFromInner, Debug)]
    pub enum PaintShape {
        Rectangle(PaintRectangle),
        Text(PlacedTextBox),
        Mesh(PaintMesh),
    }
}

impl<F: Num + Copy + Default + Two + MaxNum, U> PaintRectangle<F, U> {
    pub fn get_bounding_box(&self) -> Rect<F, U> {
        let fac = [
            self.stroke_width.map(|w| w / F::TWO),
            self.blur.as_ref().map(|b| b.blur_radius),
            Some(F::one() / F::TWO), // feathering
        ]
        .into_iter()
        .flatten()
        .reduce(MaxNum::max_num)
        .unwrap_or_default();

        self.rounded_rect.inner.inflate(fac, fac)
    }
}

impl<T: Copy + Mul, U1, U2> Mul<ScaleFactor<T, U1, U2>> for PaintRectangle<T, U1> {
    type Output = PaintRectangle<T::Output, U2>;

    #[inline]
    fn mul(self, scale: ScaleFactor<T, U1, U2>) -> Self::Output {
        Self::Output {
            blur: self.blur.map(|x| x * scale),
            fill: self.fill,
            rounded_rect: self.rounded_rect * scale,
            stroke_color: self.stroke_color,
            stroke_width: self.stroke_width.map(|x| x * scale.get()),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct ShapeBufferWithContext {
    pub(super) vertex_buffers: VertexBuffers<BoxShaderVertex>,
    pub(super) clip_rect_idx: u32,
    pub(super) transformation_idx: u32,
}

impl ShapeBufferWithContext {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PushVertices<BoxShaderVertex> for ShapeBufferWithContext {
    fn push_vertices(
        &mut self,
        vertices: impl IntoIterator<Item = BoxShaderVertex>,
        indices: impl IntoIterator<Item = u16>,
    ) {
        let clip_rect_idx = self.clip_rect_idx;
        let transformation_idx = self.transformation_idx;

        self.vertex_buffers.push_vertices(
            vertices.into_iter().map(|mut x| {
                x.clip_rect_idx = clip_rect_idx;
                x.transform_idx = transformation_idx;
                x
            }),
            indices,
        )
    }
}
