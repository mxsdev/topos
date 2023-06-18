use crate::{color::ColorRgba, surface::SurfaceDependent};

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
    num::NonZeroU64,
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use euclid::size2;

use rayon::prelude::*;

use cosmic_text::{FontSystem, LayoutGlyph, SubpixelBin};
use etagere::{Allocation as AtlasAllocation, BucketedAtlasAllocator};
use rustc_hash::FxHashMap;
use swash::scale::ScaleContext;

use crate::{
    debug::{DebugAssert, HashU64},
    debug_panic,
    graphics::DynamicGPUQuadBuffer,
    num::NextPowerOfTwo,
    surface::{ParamsBuffer, RenderingContext},
    text::GlyphContentType,
    util::{
        CanScale, LogicalToPhysical, LogicalToPhysicalInto, LogicalUnit, PhysicalPos2,
        PhysicalRect, PhysicalSize2, PhysicalUnit, PhysicalVec2, Pos2, Rect, WgpuDescriptor,
    },
};

type GlyphCacheKey = cosmic_text::CacheKey;

pub struct GlyphToRender {
    size: PhysicalSize2<u32>,
    draw_rect: PhysicalRect,
    alloc: AtlasAllocation, // uv: Option<Size2>,
    color: ColorRgba,
    // clip_rect: Option<PhysicalRect>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FontVertex {
    pos: [f32; 2],
    uv: [u32; 2],
    color: [f32; 4],
    content_type: u32,
    depth: f32,
}

impl WgpuDescriptor<5> for FontVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Uint32x2,
        2 => Float32x4,
        3 => Uint32,
        4 => Float32
    ];
}

pub(crate) struct FontAtlas {
    allocator: BucketedAtlasAllocator,
    texture: wgpu::Texture,
    sampler: wgpu::Sampler,
    texture_view: wgpu::TextureView,
    atlas_type: GlyphContentType,
    width: i32,
    height: i32,

    shader: wgpu::ShaderModule,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,

    gpu_buffer: DynamicGPUQuadBuffer<FontVertex>,

    vertex_buffer_glyphs: u64,

    num_glyphs: u64,
}

impl FontAtlas {
    const MIN_NUM_VERTS: u64 = 32;

    pub fn new(
        context: &RenderingContext,
        atlas_type: GlyphContentType,
        width: u32,
        height: u32,
    ) -> Self {
        let RenderingContext { device, .. } = context;

        let allocator = BucketedAtlasAllocator::new(etagere::size2(width as i32, height as i32));

        let max_texture_dimension_2d = device.limits().max_texture_dimension_2d;

        let width = width.min(max_texture_dimension_2d);
        let height = height.min(max_texture_dimension_2d);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FontAtlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: match atlas_type {
                GlyphContentType::Mask => wgpu::TextureFormat::R8Unorm,
                GlyphContentType::Color => wgpu::TextureFormat::Rgba8UnormSrgb,
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shader = device.create_shader_module(wgpu::include_wgsl!("text.wgsl"));

        let (render_pipeline, bind_group) =
            Self::render_pipeline(&shader, &sampler, &texture_view, context);

        let vertex_buffer_glyphs = Self::MIN_NUM_VERTS;

        Self {
            allocator,
            texture,
            sampler,
            texture_view,
            atlas_type,
            width: width as i32,
            height: width as i32,
            shader,
            render_pipeline,
            bind_group,
            vertex_buffer_glyphs,
            num_glyphs: 0,
            gpu_buffer: DynamicGPUQuadBuffer::new(device),
        }
    }

    pub fn prepare(
        &mut self,
        RenderingContext { queue, device, .. }: &RenderingContext,
        glyphs: Vec<GlyphToRender>,
    ) {
        self.gpu_buffer.set_num_quads(device, glyphs.len() as u64);

        self.gpu_buffer.write_all_quads(
            queue,
            glyphs.iter().map(
                |GlyphToRender {
                     alloc: AtlasAllocation { rectangle: uv, .. },
                     draw_rect,
                     size,
                     color,
                     ..
                 }| {
                    let alloc_pos = PhysicalPos2::new(uv.min.x as u32, uv.min.y as u32);
                    let uv = PhysicalRect::new(alloc_pos, alloc_pos + *size);
                    let color = (*color).into();

                    [
                        FontVertex {
                            pos: [draw_rect.min.x, draw_rect.min.y],
                            uv: [uv.min.x as u32, uv.min.y as u32],
                            color,
                            content_type: self.atlas_type as u32,
                            depth: 0.,
                        },
                        FontVertex {
                            pos: [draw_rect.max.x, draw_rect.min.y],
                            uv: [uv.max.x as u32, uv.min.y as u32],
                            color,
                            content_type: self.atlas_type as u32,
                            depth: 0.,
                        },
                        FontVertex {
                            pos: [draw_rect.min.x, draw_rect.max.y],
                            uv: [uv.min.x as u32, uv.max.y as u32],
                            color,
                            content_type: self.atlas_type as u32,
                            depth: 0.,
                        },
                        FontVertex {
                            pos: [draw_rect.max.x, draw_rect.max.y],
                            uv: [uv.max.x as u32, uv.max.y as u32],
                            color,
                            content_type: self.atlas_type as u32,
                            depth: 0.,
                        },
                    ]
                },
            ),
        );
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, quads: u64) {
        self.gpu_buffer.render_quads(
            &self.render_pipeline,
            &self.bind_group,
            render_pass,
            quads,
            0..1,
        );
    }

    fn try_allocate_space(&mut self, space: &PhysicalSize2<u32>) -> Option<AtlasAllocation> {
        let space = PhysicalSize2::new(space.width as i32, space.height as i32);

        if !self.can_fit(space) {
            return None;
        }

        self.allocator.allocate(size2(space.width, space.height))
    }

    pub fn allocate_glyph(
        &mut self,
        image: &cosmic_text::SwashImage,
        RenderingContext { queue, .. }: &RenderingContext,
    ) -> Option<AtlasAllocation> {
        let size = PhysicalSize2::new(image.placement.width, image.placement.height);

        let alloc = self.try_allocate_space(&size)?;

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: alloc.rectangle.min.x as u32,
                    y: alloc.rectangle.min.y as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::default(),
            },
            &image.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(image.placement.width * self.atlas_type.num_channels()),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: image.placement.width,
                height: image.placement.height,
                depth_or_array_layers: 1,
            },
        );

        Some(alloc)
    }

    fn can_fit(&self, space: PhysicalSize2<i32>) -> bool {
        return space.width <= self.width && space.height <= self.height;
    }

    pub fn render_pipeline(
        shader: &wgpu::ShaderModule,
        sampler: &wgpu::Sampler,
        texture_view: &wgpu::TextureView,
        RenderingContext {
            device,
            params_buffer,
            texture_format,
            num_samples,
            ..
        }: &RenderingContext,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("font atlas bind group"),
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
            label: Some("font atlas pipeline"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("font atlas"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[FontVertex::desc()],
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
                // cull_mode: Some(wgpu::Face::Front),
                cull_mode: None,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: *num_samples.read().unwrap(),
                ..Default::default()
            },
            multiview: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("font atlas"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        params_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        (render_pipeline, bind_group)
    }
}

impl SurfaceDependent for FontAtlas {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        let (render_pipeline, bind_group) =
            Self::render_pipeline(&self.shader, &self.sampler, &self.texture_view, context);

        self.render_pipeline = render_pipeline;
        self.bind_group = bind_group;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AtlasId(GlyphContentType, u32);

#[derive(Clone, Copy)]
struct GlyphAllocation {
    atlas_id: AtlasId,
    allocation: AtlasAllocation,
    size: PhysicalSize2<u32>,
    placement: PhysicalPos2<i32>,
}

enum GlyphCacheEntry {
    GlyphAllocation(GlyphAllocation),
    Noop,
}

// TODO: use no hash hasher
type FontAtlasCollection = HashMap<AtlasId, FontAtlas>;

struct FontAtlasManager {
    mask_atlases: FontAtlasCollection,
    color_atlases: FontAtlasCollection,

    glyphs: FxHashMap<GlyphCacheKey, GlyphCacheEntry>,

    id: u32,

    rendering_context: Arc<RenderingContext>,
}

pub struct PlacedTextBox<F = f32, U = LogicalUnit> {
    glyphs: Vec<PlacedGlyph>,
    clip_rect: Option<euclid::Box2D<F, U>>,
    color: ColorRgba,
    pub pos: euclid::Point2D<F, U>,
}

impl<U> PlacedTextBox<f32, U> {
    pub fn from_buffer(
        buffer: &cosmic_text::Buffer,
        pos: euclid::Point2D<f32, U>,
        color: ColorRgba,
    ) -> Self {
        Self {
            glyphs: PlacedGlyph::from_buffer(buffer).collect(),
            clip_rect: None,
            pos,
            color,
        }
    }
}

impl PlacedTextBox<f32, PhysicalUnit> {
    pub fn recalculate_subpixel_offsets(&mut self) {
        for glyph in self.glyphs.iter_mut() {
            let x = self.pos.x + glyph.cache_key.x_bin.as_float();
            let y = self.pos.y + glyph.cache_key.y_bin.as_float();

            let (x_pos, x_bin) = SubpixelBin::new(x);
            let (y_pos, y_bin) = SubpixelBin::new(y);

            glyph.cache_key.x_bin = x_bin;
            glyph.cache_key.y_bin = y_bin;

            self.pos.x = x_pos as f32;
            self.pos.y = y_pos as f32;
        }
    }
}

impl<F, U> PlacedTextBox<F, U> {
    pub fn with_clip_rect(mut self, rect: impl Into<Option<euclid::Box2D<F, U>>>) -> Self {
        self.clip_rect = rect.into();
        self
    }
}

impl<F: CanScale> LogicalToPhysicalInto for PlacedTextBox<F, LogicalUnit> {
    type PhysicalResult = PlacedTextBox<F, PhysicalUnit>;

    fn to_physical(self, scale_factor: impl CanScale) -> Self::PhysicalResult {
        Self::PhysicalResult {
            clip_rect: self.clip_rect.map(|x| x.to_physical(scale_factor)),
            color: self.color,
            glyphs: self.glyphs,
            pos: self.pos.to_physical(scale_factor),
        }
    }
}

#[derive(Debug)]
pub struct PlacedGlyph {
    // pub glyph: &'a LayoutGlyph,
    pub x_int: i32,
    pub y_int: i32,
    pub line_offset: f32,
    pub cache_key: GlyphCacheKey,
    pub depth: f32,
}

impl PlacedGlyph {
    pub fn from_layout_glyph(glyph: &LayoutGlyph, line_offset: f32) -> Self {
        Self {
            x_int: glyph.x_int,
            y_int: glyph.y_int,
            cache_key: glyph.cache_key,
            line_offset,
            depth: 0.,
        }
    }

    pub fn from_buffer(buffer: &cosmic_text::Buffer) -> impl Iterator<Item = Self> + '_ {
        buffer.layout_runs().flat_map(|r| {
            let line_y = r.line_y;

            r.glyphs
                .iter()
                .map(move |g| Self::from_layout_glyph(g, line_y.clone()))
        })
    }
}

// impl<'a> Deref for LayoutGlyphWithContext<'a> {
//     type Target = LayoutGlyph;

//     fn deref(&self) -> &Self::Target {
//         &self.glyph
//     }
// }

macro_rules! get_coll_mut {
    ($self:ident, $($arg:tt)*) => {
        match $($arg)* {
            GlyphContentType::Color => &mut $self.color_atlases,
            GlyphContentType::Mask => &mut $self.mask_atlases,
        }
    };
}

#[derive(Debug)]
pub struct BatchedAtlasRender {
    pub atlas_id: AtlasId,
    pub num_quads: u64,
}

impl BatchedAtlasRender {
    pub fn new(atlas_id: AtlasId) -> Self {
        Self {
            atlas_id,
            num_quads: Default::default(),
        }
    }
}

#[derive(Debug)]
pub enum BatchedAtlasRenderBoxesEntry {
    Batch(BatchedAtlasRender),
    Done,
}

impl Into<BatchedAtlasRenderBoxesEntry> for BatchedAtlasRender {
    fn into(self) -> BatchedAtlasRenderBoxesEntry {
        BatchedAtlasRenderBoxesEntry::Batch(self)
    }
}

#[derive(Default)]
struct BatchedAtlasRenderBoxes {
    batches: Vec<BatchedAtlasRenderBoxesEntry>,
    render_batch: Option<BatchedAtlasRender>,
}

impl BatchedAtlasRenderBoxes {
    fn new() -> Self {
        Self::default()
    }

    fn new_quad(&mut self, atlas_id: AtlasId) {
        let current_render_batch = self
            .render_batch
            .get_or_insert(BatchedAtlasRender::new(atlas_id));

        if current_render_batch.atlas_id != atlas_id {
            let mut old_render_batch = BatchedAtlasRender::new(atlas_id);

            std::mem::swap(current_render_batch, &mut old_render_batch);

            self.batches.push(old_render_batch.into());
        }

        current_render_batch.num_quads += 1;
    }

    fn finish_text_box(&mut self) {
        self.batches
            .extend(self.render_batch.take().into_iter().map(Into::into));

        self.batches.push(BatchedAtlasRenderBoxesEntry::Done)
    }

    fn as_iterator(
        self,
    ) -> BatchedAtlasRenderBoxIterator<impl Iterator<Item = BatchedAtlasRenderBoxesEntry>> {
        BatchedAtlasRenderBoxIterator {
            batches: self.batches.into_iter(),
        }
    }
}

pub struct BatchedAtlasRenderBoxIterator<T: Iterator<Item = BatchedAtlasRenderBoxesEntry>> {
    batches: T,
}

impl<T: Iterator<Item = BatchedAtlasRenderBoxesEntry>> Iterator
    for BatchedAtlasRenderBoxIterator<T>
{
    type Item = BatchedAtlasRender;

    fn next(&mut self) -> Option<Self::Item> {
        match self.batches.next()? {
            BatchedAtlasRenderBoxesEntry::Batch(b) => Some(b),
            BatchedAtlasRenderBoxesEntry::Done => None,
        }
    }
}

impl FontAtlasManager {
    pub fn new(rendering_context: Arc<RenderingContext>) -> Self {
        return Self {
            mask_atlases: Default::default(),
            color_atlases: Default::default(),

            glyphs: Default::default(),

            id: 0,

            rendering_context,
        };
    }

    pub fn prepare<'a>(
        &mut self,
        boxes: impl Iterator<Item = PlacedTextBox<f32, PhysicalUnit>>,
    ) -> BatchedAtlasRenderBoxIterator<impl Iterator<Item = BatchedAtlasRenderBoxesEntry>> {
        // convert to renderable glyphs
        let mut partition = FxHashMap::<AtlasId, Vec<GlyphToRender>>::default();

        let mut render_batches = BatchedAtlasRenderBoxes::new();

        for text_box in boxes {
            for g in text_box.glyphs {
                let alloc = self.glyphs.get(&g.cache_key);

                match alloc {
                    Some(GlyphCacheEntry::GlyphAllocation(GlyphAllocation {
                        atlas_id,
                        size,
                        allocation,
                        placement,
                        ..
                    })) => {
                        let PlacedTextBox {
                            clip_rect,
                            color,
                            pos,
                            ..
                        } = text_box;

                        let mut glyph_pos = pos
                            + PhysicalVec2::new(
                                (g.x_int + placement.x) as f32,
                                (g.y_int - placement.y) as f32 + g.line_offset,
                            );

                        glyph_pos = glyph_pos.round();

                        let rect_size = PhysicalSize2::new(size.width as f32, size.height as f32);

                        let draw_rect = PhysicalRect::new(glyph_pos, glyph_pos + rect_size);

                        if clip_rect
                            .map(|clip_rect| clip_rect.intersection(&draw_rect).is_none())
                            .unwrap_or_default()
                        {
                            continue;
                        }

                        let glyph_to_render = GlyphToRender {
                            alloc: *allocation,
                            draw_rect,
                            size: *size,
                            color,
                        };

                        render_batches.new_quad(*atlas_id);

                        partition
                            .entry(*atlas_id)
                            .or_insert_with(|| Vec::new())
                            .push(glyph_to_render);
                    }
                    None => log::debug!("Glyph {} not cached", g.cache_key.glyph_id),
                    Some(GlyphCacheEntry::Noop) => {}
                }
            }

            render_batches.finish_text_box();
        }

        for (atlas_id, layout_glyphs) in partition.into_iter() {
            let render_context = self.rendering_context.clone();

            if let Some(atlas) = self.get_atlas_mut(atlas_id) {
                atlas.prepare(&render_context, layout_glyphs);
            }
        }

        render_batches.as_iterator()
    }

    pub fn render<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        atlas_id: AtlasId,
        glyphs: u64,
    ) {
        // TODO: make this a Result
        let atlas = self.get_atlas(atlas_id).unwrap();
        atlas.render(render_pass, glyphs);
    }

    fn get_coll(&self, kind: GlyphContentType) -> &FontAtlasCollection {
        match kind {
            GlyphContentType::Color => &self.color_atlases,
            GlyphContentType::Mask => &self.mask_atlases,
        }
    }

    fn get_coll_mut(&mut self, kind: GlyphContentType) -> &mut FontAtlasCollection {
        get_coll_mut!(self, kind)
    }

    fn get_atlas_mut(&mut self, id: AtlasId) -> Option<&mut FontAtlas> {
        self.get_coll_mut(id.0).get_mut(&id)
    }

    fn get_atlas(&self, id: AtlasId) -> Option<&FontAtlas> {
        self.get_coll(id.0).get(&id)
    }

    fn create_atlas(&mut self, kind: GlyphContentType, size: u32) -> AtlasId {
        let size = u32::max(size, 512);

        log::trace!("Creating new atlas of size {size}");

        let atlas_id = AtlasId(kind, self.id);
        self.id += 1;

        let atlas = FontAtlas::new(&self.rendering_context, kind, size, size);

        self.get_coll_mut(kind).insert(atlas_id, atlas);

        atlas_id
    }

    pub fn has_glyph(&self, key: &GlyphCacheKey) -> bool {
        self.glyphs.contains_key(key)
    }

    pub fn allocate_glyph(
        &mut self,
        kind: GlyphContentType,
        // glyph_size: PhysicalSize2<u32>,
        image: cosmic_text::SwashImage,
        cache_key: GlyphCacheKey,
    ) -> Option<GlyphAllocation> {
        let glyph_size = PhysicalSize2::<u32>::new(image.placement.width, image.placement.height);

        let glyph_placement = PhysicalPos2::<i32>::new(image.placement.left, image.placement.top);

        if glyph_size.is_empty() {
            self.glyphs.insert(cache_key, GlyphCacheEntry::Noop);
            return None;
        }

        let rendering_context = self.rendering_context.clone();

        let coll = self.get_coll_mut(kind);

        let alloc = coll
            .iter_mut()
            .map(|(id, atlas)| {
                atlas
                    .allocate_glyph(&image, &rendering_context)
                    .map(|res| (*id, res))
            })
            .flatten()
            .next()
            .or_else(|| {
                let size = u32::max(glyph_size.width, glyph_size.height).next_power_of_2();
                let atlas_id = self.create_atlas(kind, size);

                match self.get_atlas_mut(atlas_id) {
                    Some(atlas) => match atlas.allocate_glyph(&image, &rendering_context) {
                        Some(res) => Some((atlas_id, res)),
                        None => {
                            log::error!(
                                "Failed to allocate space for glyph {:x}",
                                cache_key.hash_u64()
                            );

                            None
                        }
                    },
                    None => {
                        debug_panic!("Failed to get atlas for glyph");

                        None
                    }
                }
            })
            .map(|(atlas_id, alloc)| GlyphAllocation {
                atlas_id,
                allocation: alloc,
                size: glyph_size,
                placement: glyph_placement,
            });

        self.glyphs
            .insert(cache_key, GlyphCacheEntry::GlyphAllocation(alloc?));

        let _atlas = self.get_atlas_mut(alloc?.atlas_id).debug_assert()?;

        // atlas.

        // match alloc {
        //     Some(alloc) => {
        //     }
        //     None => log::error!(
        //         "Failed to allocate buffer for glyph {:x}",
        //         cache_key.hash_u64()
        //     ),
        // }

        alloc
    }

    pub fn get_glyph_uv() {}
}

impl SurfaceDependent for FontAtlasManager {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        for atlas in [
            self.color_atlases.values_mut(),
            self.mask_atlases.values_mut(),
        ]
        .into_iter()
        .flatten()
        {
            atlas.reconfigure(context, size, scale_factor);
        }
    }
}

pub struct FontManager {
    font_system: Arc<Mutex<FontSystem>>,
    atlas_manager: Arc<RwLock<FontAtlasManager>>,
}

pub struct FontManagerRenderResources<'a> {
    atlas_manager: RwLockWriteGuard<'a, FontAtlasManager>,
}

impl FontManager {
    pub fn new(rendering_context: Arc<RenderingContext>) -> Self {
        let font_system = FontSystem::new();

        let atlas_manager = FontAtlasManager::new(rendering_context);

        return Self {
            font_system: Arc::new(Mutex::new(font_system)),
            atlas_manager: Arc::new(RwLock::new(atlas_manager)),
        };
    }

    pub fn prepare<'a>(
        &mut self,
        mut text_boxes: Vec<PlacedTextBox<f32, PhysicalUnit>>,
    ) -> BatchedAtlasRenderBoxIterator<impl Iterator<Item = BatchedAtlasRenderBoxesEntry>> {
        for text_box in text_boxes.iter_mut() {
            text_box.recalculate_subpixel_offsets();
        }

        self.generate_textures(
            text_boxes
                .iter()
                .flat_map(|b| b.glyphs.iter())
                .map(|g| g.cache_key)
                .collect(),
        );

        self.atlas_manager
            .write()
            .unwrap()
            .prepare(text_boxes.into_iter())
    }

    pub fn get_font_system_ref(&self) -> Arc<Mutex<FontSystem>> {
        self.font_system.clone()
    }

    pub fn get_font_system(&mut self) -> MutexGuard<'_, cosmic_text::FontSystem> {
        return self.font_system.lock().unwrap();
    }

    pub fn render_resources(&self) -> FontManagerRenderResources<'_> {
        FontManagerRenderResources {
            atlas_manager: self.atlas_manager.write().unwrap(),
        }
    }

    pub fn render<'a, 'b, 'c>(
        &self,
        render_pass: &'a mut wgpu::RenderPass<'b>,
        resources: &'b FontManagerRenderResources<'c>,
        batch: &BatchedAtlasRender,
    ) {
        resources
            .atlas_manager
            .render(render_pass, batch.atlas_id, batch.num_quads);
    }

    pub fn generate_textures<'a>(&mut self, mut glyphs: HashSet<GlyphCacheKey>) {
        let atlas_manager = self.atlas_manager.clone();
        let font_system = self.font_system.clone();

        std::thread::spawn(move || {
            let results: Vec<(GlyphCacheKey, cosmic_text::SwashImage)> = glyphs
                .par_drain()
                .map(|g| {
                    if atlas_manager.read().unwrap().has_glyph(&g) {
                        return None;
                    }

                    match rasterize_glyph(&g, font_system.as_ref()) {
                        Some(image) => Some((g, image)),
                        None => {
                            log::error!("failed to render glyph {}!", g.glyph_id);

                            None
                        }
                    }
                })
                .flatten()
                .collect();

            for (cache_key, image) in results {
                if let Some(kind) = match image.content {
                    cosmic_text::SwashContent::Mask => Some(GlyphContentType::Mask),
                    cosmic_text::SwashContent::Color => Some(GlyphContentType::Color),
                    cosmic_text::SwashContent::SubpixelMask => {
                        debug_panic!("Found subpixel mask!");
                        None
                    }
                } {
                    atlas_manager
                        .write()
                        .unwrap()
                        .allocate_glyph(kind, image, cache_key);
                }
            }
        });
    }
}

thread_local! {
    static SCALE_CONTEXT: RefCell<ScaleContext> = RefCell::new(ScaleContext::new())
}

fn rasterize_glyph(
    cache_key: &GlyphCacheKey,
    font_system: &Mutex<FontSystem>,
) -> Option<cosmic_text::SwashImage> {
    log::debug!("Rasterizing glyph {:x}", cache_key.hash_u64());

    use swash::{
        scale::{Render, Source, StrikeWith},
        zeno::{Format, Vector},
    };

    let font = font_system.lock().unwrap().get_font(cache_key.font_id);

    let font = match font {
        Some(some) => some,
        None => {
            // todo: error here
            log::warn!("did not find font {:?}", cache_key.font_id);
            return None;
        }
    };

    // Compute the fractional offset-- you'll likely want to quantize this
    // in a real renderer
    let offset = Vector::new(cache_key.x_bin.as_float(), cache_key.y_bin.as_float());

    // Select our source order
    let mut render = Render::new(&[
        // Color outline with the first palette
        Source::ColorOutline(0),
        // Color bitmap with best fit selection mode
        Source::ColorBitmap(StrikeWith::BestFit),
        // Standard scalable outline
        Source::Outline,
    ]);

    // Select a subpixel format
    render.format(Format::Alpha);

    // Apply the fractional offset
    render.offset(offset);

    SCALE_CONTEXT.with(move |context| {
        let mut context = context.borrow_mut();

        // Build the scaler
        let mut scaler = context
            .builder(font.as_swash())
            .size(f32::from_bits(cache_key.font_size_bits))
            .hint(true)
            .build();

        render.render(&mut scaler, cache_key.glyph_id)
    })
}

impl SurfaceDependent for FontManager {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        self.atlas_manager
            .write()
            .unwrap()
            .reconfigure(context, size, scale_factor)
    }
}
