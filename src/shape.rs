use serde::Serialize;

use crate::{
    atlas::{
        AtlasAllocation, AtlasAllocationId, HasAtlasAllocationId, TextureAtlas,
        TextureAtlasManager, TextureAtlasManagerRef,
    },
    color::{ColorRgba, ColorSrgba},
    graphics::{DynamicGPUBuffer, DynamicGPUMeshTriBuffer, Mesh, PushVertices, VertexBuffers},
    math::{
        CoordinateTransform, PhysicalPos, PhysicalRect, PhysicalSize, Pos, RoundedRect,
        ScaleFactor, Size, Vector, WindowScaleFactor,
    },
    surface::ParamsBuffer,
    texture::{TextureManagerRef, TextureRef},
    util::{
        guard::ReadLockable,
        svg::PosVertexBuffers,
        template::{HandlebarsTemplater, Templater},
        text::{AtlasContentType, PlacedTextBox, TextBox},
        PhysicalUnit,
    },
};

use std::{
    borrow::Borrow,
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroU64,
    ops::{Deref, Mul, Range},
    sync::{atomic::Ordering, Arc, RwLock},
};

use num_traits::{Float, Num};
use wgpu::{BufferUsages, ShaderModuleDescriptor};

use crate::{
    num::{MaxNum, Two},
    surface::RenderingContext,
    util::{math::Rect, LogicalUnit, WgpuDescriptor},
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
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
    TextureMaskTexture,
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

    uv: [f32; 2],
    atlas_idx_alt: u16,
    atlas_idx: u16,

    color: [f32; 4],

    rounding: f32,
    stroke_width: f32,
    blur_radius: f32,

    clip_rect_idx: u32,

    transform_idx: u32,

    uv_alt: [f32; 2],
}

impl WgpuDescriptor<15> for BoxShaderVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 15] = wgpu::vertex_attr_array![
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
        6 => Float32x2,
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

        // uv_alt
        14 => Float32x2,
    ];
}

#[derive(Default)]
struct BoxShaderVertexFill {
    color: [f32; 4],
    atlas_idx: u16,
    atlas_idx_alt: u16,
    uv: PhysicalRect,
    uv_alt: PhysicalRect,
    fill_mode: FillMode,
}

impl BoxShaderVertexFill {
    #[inline]
    fn new(
        atlas_manager: impl ReadLockable<TextureAtlasManager>,
        fill: PaintFill,
        main_texture: Option<(&TextureRef, PhysicalRect, AtlasContentType)>,
        uv_mask: Option<Rect>,
    ) -> Self {
        let mut res = Self::default();

        let main_texture = main_texture.map(|(tex, uv, ty)| (tex.get_binding_idx() as u16, uv, ty));

        let mut atlas_idx_target = &mut res.atlas_idx;
        let mut uv_target = &mut res.uv;

        if let Some((binding_idx, uv, _)) = main_texture {
            *atlas_idx_target = binding_idx;
            atlas_idx_target = &mut res.atlas_idx_alt;

            *uv_target = uv.into();
            uv_target = &mut res.uv_alt;
        }

        match fill {
            PaintFill::Color(color) => {
                res.color = color.into();
                res.fill_mode = FillMode::Color;
            }
            PaintFill::Texture(_) => todo!(),
            PaintFill::TextureAtlas(alloc, uv) => {
                let binding_idx = atlas_manager
                    .borrow()
                    .read_lock()
                    .get_atlas_by_id(&alloc.atlas_id)
                    .unwrap()
                    .get_texture_ref()
                    .get_binding_idx();

                let alloc_rect = alloc.draw_rect();

                let mut uv_rect = alloc_rect.map(|x| x as f32);

                if let Some(uv) = uv {
                    uv_rect =
                        uv_rect.intersection_unchecked(&(uv.translate(uv_rect.min.to_vector())))
                }

                if let Some(uv_mask) = uv_mask {
                    let uv_rect_size = uv_rect.size();

                    uv_rect = Rect::new(
                        Pos::new(
                            uv_rect.min.x + uv_rect_size.width * uv_mask.min.x,
                            uv_rect.min.y + uv_rect_size.height * uv_mask.min.y,
                        ),
                        Pos::new(
                            uv_rect.min.x + uv_rect_size.width * uv_mask.max.x,
                            uv_rect.min.y + uv_rect_size.height * uv_mask.max.y,
                        ),
                    );
                }

                *atlas_idx_target = binding_idx as u16;
                res.fill_mode = FillMode::Texture;
                *uv_target = uv_rect.into();
            }
        };

        res.fill_mode = match (main_texture, res.fill_mode) {
            (Some((_, _, AtlasContentType::Mask)), FillMode::Color) => FillMode::TextureMaskColor,
            (Some((_, _, AtlasContentType::Mask)), FillMode::Texture) => {
                FillMode::TextureMaskTexture
            }

            (Some((_, _, AtlasContentType::Color)), FillMode::Color) => FillMode::Texture,
            (Some((_, _, AtlasContentType::Color)), FillMode::Texture) => {
                FillMode::TextureMaskTexture
            }

            (Some(_), _) => panic!("Invalid texture type"),

            (None, _) => res.fill_mode,
        };

        res
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RectPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

const RECT_POSITIONS: [RectPosition; 4] = [
    RectPosition::TopLeft,
    RectPosition::TopRight,
    RectPosition::BottomLeft,
    RectPosition::BottomRight,
];

const RECT_INDICES: [u16; 6] = [0, 1, 2, 1, 2, 3];

#[inline(always)]
fn rect_vertex<T: Copy, U>(rect: &Rect<T, U>, pos: RectPosition) -> [T; 2] {
    match pos {
        RectPosition::TopLeft => [rect.min.x, rect.min.y],
        RectPosition::TopRight => [rect.max.x, rect.min.y],
        RectPosition::BottomLeft => [rect.min.x, rect.max.y],
        RectPosition::BottomRight => [rect.max.x, rect.max.y],
    }
}

impl BoxShaderVertex {
    #[inline]
    fn with_fill(mut self, fill: &BoxShaderVertexFill, idx: RectPosition) -> Self {
        self.color = fill.color;
        self.atlas_idx = fill.atlas_idx;
        self.atlas_idx_alt = fill.atlas_idx_alt;
        self.fill_mode = fill.fill_mode;

        self.uv = rect_vertex(&fill.uv, idx);
        self.uv_alt = rect_vertex(&fill.uv_alt, idx);

        self
    }

    #[inline]
    fn with_position(mut self, pos: &Rect, idx: RectPosition) -> Self {
        self.pos = rect_vertex(pos, idx);

        self
    }

    pub(crate) fn from_paint_rect(
        atlas_manager: &TextureAtlasManagerRef,
        paint_rect: PaintRectangle,
    ) -> (impl Iterator<Item = [Self; 4]>, u64) {
        let fill_rect = paint_rect.fill.map(|f| {
            Self::from_rect_stroked(&atlas_manager, paint_rect.rounded_rect, f, None, None)
        });

        let stroke_rect =
            paint_rect
                .stroke_color
                .zip(paint_rect.stroke_width)
                .map(|(color, width)| {
                    Self::from_rect_stroked(
                        &atlas_manager,
                        paint_rect.rounded_rect,
                        color,
                        Some(width),
                        None,
                    )
                });

        let blur_rect = paint_rect.blur.map(
            |PaintBlur {
                 blur_radius, color, ..
             }| {
                Self::from_rect_stroked(
                    &atlas_manager,
                    paint_rect.rounded_rect,
                    color,
                    None,
                    Some(blur_radius),
                )
            },
        );

        let rects = [blur_rect, fill_rect, stroke_rect];

        let num_rects = rects.iter().filter(|x| x.is_some()).count();

        (rects.into_iter().flatten(), num_rects as u64)
    }

    pub(crate) fn mesh_tri(pos: Pos, color: ColorRgba) -> Self {
        Self {
            shape_type: ShapeType::Mesh,
            fill_mode: FillMode::Color,
            pos: [pos.x, pos.y],
            color: color.into(),
            ..Default::default()
        }
    }

    pub(crate) fn glyph_rect(
        atlas_manager: impl ReadLockable<TextureAtlasManager>,
        rect: Rect<f32>,
        uv: Rect<u32, PhysicalUnit>,
        glyph_type: AtlasContentType, // TODO: texture id
        fill: impl Into<PaintFill>,
        texture_ref: &TextureRef,
        box_rect: Rect<f32>,
    ) -> ([Self; 4], [u16; 6]) {
        let uv_mask = box_rect.intersection(&rect).map(|r| {
            r.map_points(|x| {
                let v = x - box_rect.min.to_vector();
                Pos::new(v.x / box_rect.width(), v.y / box_rect.height())
            })
        });

        let fill = BoxShaderVertexFill::new(
            atlas_manager,
            fill.into(),
            Some((texture_ref, uv.map(|x| x as f32), glyph_type)),
            uv_mask,
        );

        return (
            RECT_POSITIONS.map(|idx| {
                Self {
                    shape_type: ShapeType::Mesh,
                    ..Default::default()
                }
                .with_fill(&fill, idx)
                .with_position(&rect, idx)
            }),
            RECT_INDICES,
        );
    }

    fn from_rect_stroked(
        atlas_manager: &TextureAtlasManagerRef,
        rounded_rect: RoundedRect<f32>,
        color: impl Into<PaintFill>,
        stroke_width: Option<f32>,
        blur_radius: Option<f32>,
    ) -> [Self; 4] {
        let RoundedRect {
            inner: rect,
            radius,
        } = rounded_rect;

        let rounding = radius.unwrap_or(0.);

        let origin = rect.center();

        let dims = rect.max - origin;

        let stroke_width = stroke_width.unwrap_or(0.);
        let blur_radius = blur_radius.unwrap_or(0.);

        let origin = origin.into();

        let fill = BoxShaderVertexFill::new(atlas_manager, color.into(), None, None);

        return RECT_POSITIONS.map(|idx| {
            Self {
                shape_type: ShapeType::Rectangle,
                origin,
                dims: [dims.x, dims.y],
                rounding,
                stroke_width,
                blur_radius,
                ..Default::default()
            }
            .with_fill(&fill, idx)
            .with_position(&rect, idx)
        });
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
    pub fn new(rect: Rect, rounding: f32, transformation_idx: u32) -> Self {
        let origin = rect.center().to_vector();
        let half_size = rect.max - origin;

        Self {
            origin: origin.into(),
            half_size: half_size.into(),
            rounding,
            transformation_idx,
        }
    }

    pub fn from_clip_rect(rect: ClipRect<f32>, transformation_idx: u32) -> Self {
        Self::new(rect.inner, rect.radius.unwrap_or(0.), transformation_idx)
    }

    pub fn to_clip_rect_idx(self) -> (ClipRect, Option<usize>) {
        (
            ClipRect::new(
                Rect::new(
                    Pos::new(
                        self.origin[0] - self.half_size[0],
                        self.origin[1] - self.half_size[1],
                    ),
                    Pos::new(
                        self.half_size[0] + self.half_size[0],
                        self.half_size[1] - self.half_size[1],
                    ),
                ),
                Some(self.rounding),
            ),
            match self.transformation_idx {
                0 => Some(self.transformation_idx as usize),
                _ => None,
            },
        )
    }
}

impl Into<ClipRect> for ShaderClipRect {
    #[inline(always)]
    fn into(self) -> ClipRect {
        self.to_clip_rect_idx().0
    }
}

pub struct ClipRectList {
    pub rects: Vec<(ClipRect, Option<usize>)>,
}

impl Default for ClipRectList {
    fn default() -> Self {
        Self {
            rects: vec![Default::default()],
        }
    }
}

impl ClipRectList {
    pub fn finish(
        self,
        window_scale_fac: WindowScaleFactor,
    ) -> impl Iterator<Item = ShaderClipRect> {
        self.rects.into_iter().map(move |(r, idx)| {
            ShaderClipRect::from_clip_rect(r, idx.map(|x| x as u32).unwrap_or_default())
        })
    }

    pub fn push_clip_rect(&mut self, rect: ClipRect, transformation_idx: Option<usize>) -> usize {
        let idx = self.rects.len();
        self.rects.push((rect, transformation_idx));
        idx
    }

    pub fn get(&mut self, idx: usize) -> &(ClipRect, Option<usize>) {
        &self.rects[idx]
    }
}

#[derive(Clone, Debug)]
pub struct PaintBlur<F = f32, U = LogicalUnit> {
    pub blur_radius: F,
    pub color: PaintFill,
    _unit: PhantomData<U>,
}

impl<F: Default, U> Default for PaintBlur<F, U> {
    fn default() -> Self {
        Self {
            blur_radius: Default::default(),
            color: PaintFill::Color(Default::default()),
            _unit: Default::default(),
        }
    }
}

impl<F: Float, U> PaintBlur<F, U> {
    pub fn new(blur_radius: F, color: impl Into<PaintFill>) -> Self {
        Self {
            blur_radius,
            color: color.into(),
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

#[derive(Copy, Clone, Debug)]
pub struct TextureFill {
    binding_idx: u32,
    uv: PhysicalRect<f32>,
}

#[derive(Copy, Clone, Debug)]
pub enum PaintFill {
    Color(ColorRgba),
    Texture(TextureFill),
    TextureAtlas(AtlasAllocationId, Option<PhysicalRect<f32>>),
}

impl From<ColorRgba> for PaintFill {
    fn from(value: ColorRgba) -> Self {
        Self::Color(value)
    }
}

impl PaintFill {
    #[inline(always)]
    pub fn from_atlas_allocation(alloc: &AtlasAllocation) -> Self {
        Self::TextureAtlas(alloc.get_id(), None)
    }

    #[inline(always)]
    pub fn from_atlas_allocation_uv(alloc: &AtlasAllocation, uv: PhysicalRect<f32>) -> Self {
        Self::TextureAtlas(alloc.get_id(), uv.into())
    }

    #[inline(always)]
    pub fn from_texture(texture: &TextureRef, uv: PhysicalRect<f32>) -> Self {
        Self::Texture(TextureFill {
            binding_idx: texture.get_binding_idx(),
            uv,
        })
    }

    #[inline(always)]
    pub fn from_entire_texture(texture: &TextureRef) -> Self {
        Self::Texture(TextureFill {
            binding_idx: texture.get_binding_idx(),
            uv: PhysicalRect::new(
                Pos::zero(),
                Pos::new(texture.texture.width(), texture.texture.height()).map(|x| x as f32),
            ),
        })
    }
}

impl From<&AtlasAllocation> for PaintFill {
    #[inline(always)]
    fn from(value: &AtlasAllocation) -> Self {
        Self::from_atlas_allocation(value)
    }
}

// TODO: adopt builder pattern (with `impl` args)
#[derive(Clone, Default, Debug)]
pub struct PaintRectangle<F = f32, U = LogicalUnit> {
    pub rounded_rect: RoundedRect<F, U>,
    pub fill: Option<PaintFill>,
    pub stroke_color: Option<PaintFill>,
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
    pub fn with_fill(mut self, fill_color: impl Into<PaintFill>) -> Self {
        self.fill = fill_color.into().into();
        self
    }

    #[inline]
    pub fn without_fill(mut self) -> Self {
        self.fill = None;
        self
    }

    #[inline]
    pub fn with_stroke_color(mut self, stroke_color: impl Into<PaintFill>) -> Self {
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
        stroke_color: impl Into<PaintFill>,
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
    pub fn with_blur_color(mut self, color: impl Into<PaintFill>) -> Self
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

pub enum PaintShape<'a> {
    Rectangle(PaintRectangle),
    Text(&'a TextBox),
    Mesh(PaintMesh),
}

impl<'a> Into<PaintShape<'a>> for &'a TextBox {
    fn into(self) -> PaintShape<'a> {
        PaintShape::Text(self)
    }
}

impl<'a> Into<PaintShape<'a>> for PaintMesh {
    fn into(self) -> PaintShape<'a> {
        PaintShape::Mesh(self)
    }
}

impl<'a> Into<PaintShape<'a>> for PaintRectangle {
    fn into(self) -> PaintShape<'a> {
        PaintShape::Rectangle(self)
    }
}

impl<'a> PaintShape<'a> {
    pub(crate) fn compute_paint_shape(
        self,
        clip_rect: impl Into<Option<RoundedRect>>,
        scale_factor: WindowScaleFactor,
    ) -> ComputedPaintShape {
        match self {
            Self::Rectangle(rect) => ComputedPaintShape::Rectangle(rect),
            Self::Text(text) => {
                ComputedPaintShape::Text(text.calculate_placed_text_box(clip_rect, scale_factor))
            }
            Self::Mesh(mesh) => ComputedPaintShape::Mesh(mesh),
        }
    }
}

pub(crate) enum ComputedPaintShape {
    Rectangle(PaintRectangle),
    Text(PlacedTextBox),
    Mesh(PaintMesh),
}

// impl PaintShape {
//     pub fn get_bounding_box(&self) -> Rect {
//         match self {
//             Self::Rectangle(rect) => rect.get_bounding_box(),
//             Self::Text(text) => Rect::from_min_size(text.pos, text.bounding_size),
//             Self::Mesh(mesh) => mesh.get_bounding_box(),
//         }
//     }
// }

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
