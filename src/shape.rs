use crate::{
    color::ColorRgba,
    math::{PhysicalPos, RoundedRect, ScaleFactor, WindowScaleFactor},
    mesh::PaintMesh,
    surface::SurfaceDependent,
    util::text::{GlyphContentType, PlacedTextBox},
};

use std::{
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroU64,
    ops::{Add, Mul, Range},
};

use bytemuck::Pod;
use num_traits::{Float, Num};
use wgpu::VertexFormat;

use crate::{
    graphics::DynamicGPUQuadBuffer,
    num::{MaxNum, Two},
    surface::{ParamsBuffer, RenderingContext},
    util::{math::Rect, LogicalUnit, PhysicalUnit, WgpuDescriptor},
};

pub struct RenderResources {
    dummy_texture_view: wgpu::TextureView,
    dummy_texture_sampler: wgpu::Sampler,

    pub bind_group: wgpu::BindGroup,
}

pub struct ShapeRenderer {
    box_resources: RenderResources,
    gpu_buffer: DynamicGPUQuadBuffer<BoxShaderVertex>,
}

impl ShapeRenderer {
    pub fn new(rendering_context: &RenderingContext) -> Self {
        Self {
            gpu_buffer: DynamicGPUQuadBuffer::new(&rendering_context.device),
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
        let buf = &mut self.gpu_buffer;

        buf.set_num_quads(device, boxes.len() as u64);

        buf.write_all_quads(queue, boxes);
    }

    pub fn render_boxes<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, num_boxes: u64) {
        self.gpu_buffer.render_quads(
            None,
            (&self.box_resources.bind_group).into(),
            render_pass,
            num_boxes,
            0..1,
        );
    }

    fn create_box_resources(render_ctx: &RenderingContext) -> RenderResources {
        let RenderingContext {
            device,
            texture_format,
            params_buffer,
            texture_info,
            ..
        } = render_ctx;

        let shader = device.create_shader_module(wgpu::include_wgsl!("box.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::default(),
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("box bind pipeline"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let dummy_texture_view = render_ctx
            .dummy_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let bind_group =
            render_ctx.create_shape_bind_group(&dummy_texture_view, &dummy_texture_sampler);

        RenderResources {
            bind_group,
            dummy_texture_view,
            dummy_texture_sampler,
        }
    }
}

impl SurfaceDependent for ShapeRenderer {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        _size: winit::dpi::PhysicalSize<u32>,
        _scale_factor: WindowScaleFactor,
    ) {
        self.box_resources = Self::create_box_resources(context)
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

    color: [f32; 4],

    rounding: f32,
    stroke_width: f32,
    blur_radius: f32,
}

impl WgpuDescriptor<11> for BoxShaderVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 11] = wgpu::vertex_attr_array![
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

        // color
        7 => Float32x4,

        // rounding
        8 => Float32,
        // stroke_width
        9 => Float32,
        // blur_radius
        10 => Float32,
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
    ) -> ([Self; 4], [u16; 6]) {
        let color: [f32; 4] = color.into();

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
                    ..Default::default()
                },
                Self {
                    shape_type: ShapeType::Mesh,
                    fill_mode,
                    pos: [rect.max.x, rect.min.y],
                    uv: [uv.max.x, uv.min.y],
                    color,
                    ..Default::default()
                },
                Self {
                    shape_type: ShapeType::Mesh,
                    fill_mode,
                    pos: [rect.min.x, rect.max.y],
                    uv: [uv.min.x, uv.max.y],
                    color,
                    ..Default::default()
                },
                Self {
                    shape_type: ShapeType::Mesh,
                    fill_mode,
                    pos: [rect.max.x, rect.max.y],
                    uv: [uv.max.x, uv.max.y],
                    color,
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

#[derive(Clone, Default)]
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
#[derive(Clone, Default)]
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

custom_derive! {
    #[derive(EnumFromInner)]
    pub enum PaintShape {
        Rectangle(PaintRectangle),
        Text(PlacedTextBox),
        ClipRect(Option<Rect>),
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

// FIXME
// impl<F: Float, U> Translate2DMut<F, U> for PaintRectangle<F, U> {
//     fn translate_mut(&mut self, x: F, y: F) {
//         self.rect.translate_mut(x, y);
//     }
// }

// impl Translate2DMut<f32, LogicalUnit> for PaintShape {
//     fn translate_mut(&mut self, x: f32, y: f32) {
//         match self {
//             PaintShape::Rectangle(rect) => rect.translate_mut(x, y),
//             PaintShape::Text(text_box) => text_box.pos.translate_mut(x, y),
//             PaintShape::ClipRect(rect) => {
//                 if let Some(rect) = rect.as_mut() {
//                     rect.translate_mut(x, y)
//                 }
//             }
//             PaintShape::Mesh(PaintMesh { vertices, .. }) => {
//                 vertices.iter_mut().for_each(|v| {
//                     v.pos.translate_mut(x, y);
//                 });
//             }
//         }
//     }
// }
