use std::{
    borrow::{Borrow, BorrowMut, Cow},
    cell::RefCell,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    num::NonZeroU64,
    ops::Deref,
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use euclid::size2;
use itertools::Itertools;
use log::{error, info, trace};
use rayon::{prelude::*, ThreadPool};

use cosmic_text::{
    Buffer as TextBuffer, Font, FontSystem, LayoutGlyph, LayoutRun, Placement, SwashCache,
};
use etagere::{AllocId, Allocation as AtlasAllocation, BucketedAtlasAllocator};
use rustc_hash::FxHashMap;
use swash::{
    scale::{Render, ScaleContext},
    shape::cluster::Glyph,
};
use tao::dpi::{LogicalSize, PhysicalSize};
use wgpu::PipelineLayout;

use crate::{
    buffer,
    debug::{DebugAssert, HashU64},
    debug_panic,
    num::NextPowerOfTwo,
    surface::{ParamsBuffer, RenderingContext},
    text::{self, GlyphContentType},
    util::{PhysicalPos2, PhysicalRect, PhysicalSize2, PhysicalVec2, Size2},
};

type GlyphCacheKey = cosmic_text::CacheKey;

pub struct GlyphToRender {
    size: PhysicalSize2<u32>,
    draw_rect: PhysicalRect,
    alloc: AtlasAllocation, // uv: Option<Size2>,
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

impl FontVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32x2, 2 => Float32x4, 3 => Uint32, 4 => Float32];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
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

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
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

        // allocator.deallocate(id)

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
        let (vertex_buffer, index_buffer) = Self::create_buffers(device, vertex_buffer_glyphs);

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
            vertex_buffer,
            index_buffer,
            vertex_buffer_glyphs,
            num_glyphs: 0,
        }
    }

    fn buffer_sizes(count: u64) -> (u64, u64) {
        (
            std::mem::size_of::<FontVertex>() as u64 * count * 4,
            6 * count * 2,
        )
    }

    fn allocate_buffers(&mut self, device: &wgpu::Device, count: u64) {
        let (vertex_buffer, index_buffer) = Self::create_buffers(device, count);

        self.vertex_buffer = vertex_buffer;
        self.index_buffer = index_buffer;
    }

    fn create_buffers(device: &wgpu::Device, count: u64) -> (wgpu::Buffer, wgpu::Buffer) {
        let (vertex_size, index_size) = Self::buffer_sizes(count);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("font atlas vertex buffer"),
            size: vertex_size.next_power_of_two(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("font atlas index buffer"),
            size: index_size.next_power_of_two(),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        return (vertex_buffer, index_buffer);
    }

    pub fn prepare(
        &mut self,
        RenderingContext { queue, .. }: &RenderingContext,
        glyphs: Vec<GlyphToRender>,
    ) {
        self.num_glyphs = glyphs.len() as u64;

        let (vertex_buffer_size, index_buffer_size) = Self::buffer_sizes(self.num_glyphs);

        let mut data = Vec::<u8>::with_capacity(vertex_buffer_size as usize);
        let mut indices = Vec::<u8>::with_capacity(index_buffer_size as usize);

        for (
            i,
            GlyphToRender {
                alloc: AtlasAllocation { rectangle: uv, .. },
                draw_rect,
                size,
            },
        ) in glyphs.iter().enumerate()
        {
            let alloc_pos = PhysicalPos2::new(uv.min.x as u32, uv.min.y as u32);
            let uv = PhysicalRect::new(alloc_pos, alloc_pos + *size);

            data.extend_from_slice(bytemuck::bytes_of(&[
                FontVertex {
                    pos: [draw_rect.min.x, draw_rect.min.y],
                    uv: [uv.min.x as u32, uv.min.y as u32],
                    color: [1., 1., 1., 1.],
                    content_type: self.atlas_type as u32,
                    depth: 0.,
                },
                FontVertex {
                    pos: [draw_rect.max.x, draw_rect.min.y],
                    uv: [uv.max.x as u32, uv.min.y as u32],
                    color: [1., 1., 1., 1.],
                    content_type: self.atlas_type as u32,
                    depth: 0.,
                },
                FontVertex {
                    pos: [draw_rect.min.x, draw_rect.max.y],
                    uv: [uv.min.x as u32, uv.max.y as u32],
                    color: [1., 1., 1., 1.],
                    content_type: self.atlas_type as u32,
                    depth: 0.,
                },
                FontVertex {
                    pos: [draw_rect.max.x, draw_rect.max.y],
                    uv: [uv.max.x as u32, uv.max.y as u32],
                    color: [1., 1., 1., 1.],
                    content_type: self.atlas_type as u32,
                    depth: 0.,
                },
            ]));

            let index_slice = [0u16, 1, 2, 1, 2, 3].map(|n| n + 4 * i as u16);

            indices.extend_from_slice(bytemuck::bytes_of(&index_slice));
        }

        queue.write_buffer(&self.vertex_buffer, 0, data.as_slice());
        queue.write_buffer(&self.index_buffer, 0, indices.as_slice());
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.num_glyphs == 0 {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.draw_indexed(0..(self.num_glyphs * 6) as u32, 0, 0..1);
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
            queue,
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
            multisample: wgpu::MultisampleState::default(),
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct AtlasId(GlyphContentType, u32);

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

#[derive(Debug)]
pub struct LayoutGlyphWithContext {
    // pub glyph: &'a LayoutGlyph,
    pub x_int: i32,
    pub y_int: i32,
    pub line_offset: f32,
    pub cache_key: GlyphCacheKey,
}

impl LayoutGlyphWithContext {
    pub fn from_layout_glyph(glyph: &LayoutGlyph, line_offset: f32) -> Self {
        Self {
            x_int: glyph.x_int,
            y_int: glyph.y_int,
            cache_key: glyph.cache_key,
            line_offset,
        }
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
        glyphs: impl Iterator<Item = LayoutGlyphWithContext>,
        pos: PhysicalPos2,
    ) {
        // convert to renderable glyphs
        let mut partition = FxHashMap::<
            AtlasId,
            Vec<(
                LayoutGlyphWithContext,
                PhysicalSize2<u32>,
                PhysicalPos2<i32>,
                AtlasAllocation,
            )>,
        >::default();

        for glyph in glyphs {
            let alloc = self.glyphs.get(&glyph.cache_key);

            match alloc {
                Some(GlyphCacheEntry::GlyphAllocation(GlyphAllocation {
                    atlas_id,
                    size,
                    allocation,
                    placement,
                    ..
                })) => partition
                    .entry(*atlas_id)
                    .or_insert_with(|| Vec::new())
                    .push((glyph, *size, *placement, *allocation)),
                None => log::debug!("Glyph {} not cached", glyph.cache_key.glyph_id),
                Some(GlyphCacheEntry::Noop) => {}
            }
        }

        for (atlas_id, layout_glyphs) in partition.into_iter() {
            let render_context = self.rendering_context.clone();

            if let Some(atlas) = self.get_atlas_mut(atlas_id) {
                let glyphs_to_render = layout_glyphs.iter().map(|(g, size, placement, alloc)| {
                    // Log::trace!("{}", g.y_int);

                    let glyph_pos = pos
                        + PhysicalVec2::new(
                            g.x_int as f32 + placement.x as f32,
                            g.y_int as f32 - placement.y as f32 + g.line_offset,
                        );

                    let rect_size = PhysicalSize2::new(size.width as f32, size.height as f32);

                    let draw_rect = PhysicalRect::new(glyph_pos, glyph_pos + rect_size);

                    GlyphToRender {
                        alloc: *alloc,
                        draw_rect,
                        size: *size,
                    }
                });

                atlas.prepare(&render_context, glyphs_to_render.collect());
            }
        }
    }

    pub fn render<'a, 'b>(&'a self, render_pass: &'b mut wgpu::RenderPass<'a>) {
        for atlas in self
            .mask_atlases
            .values()
            .chain(self.color_atlases.values())
        {
            atlas.render(render_pass);
        }
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
                log::trace!("glyph size = {:?}", glyph_size);
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

        let atlas = self.get_atlas_mut(alloc?.atlas_id).debug_assert()?;

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

pub struct FontManager {
    font_system: Arc<Mutex<FontSystem>>,
    atlas_manager: Arc<RwLock<FontAtlasManager>>,
}

pub struct FontManagerRenderResources<'a> {
    atlas_manager: RwLockReadGuard<'a, FontAtlasManager>,
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
        buffers: impl Iterator<Item = &'a cosmic_text::Buffer>,
        pos: PhysicalPos2,
    ) {
        let glyphs = buffers
            .flat_map(|buffer| buffer.layout_runs())
            .map(|line| {
                let line_offset = line.line_y;

                line.glyphs
                    .into_iter()
                    .map(move |glyph| LayoutGlyphWithContext::from_layout_glyph(glyph, line_offset))
            })
            .flatten();

        self.atlas_manager.write().unwrap().prepare(glyphs, pos)
    }

    pub fn get_font_system(&mut self) -> MutexGuard<'_, cosmic_text::FontSystem> {
        return self.font_system.lock().unwrap();
    }

    pub fn render_resources(&self) -> FontManagerRenderResources<'_> {
        FontManagerRenderResources {
            atlas_manager: self.atlas_manager.read().unwrap(),
        }
    }

    pub fn render<'a, 'b>(
        &self,
        render_pass: &'a mut wgpu::RenderPass<'b>,
        resources: &'b FontManagerRenderResources<'b>,
    ) {
        resources.atlas_manager.render(render_pass);
    }

    pub fn generate_textures<'a>(&mut self, buffers: Arc<Vec<cosmic_text::Buffer>>) {
        let atlas_manager = self.atlas_manager.clone();
        let font_system = self.font_system.clone();

        std::thread::spawn(move || {
            let results: Vec<(GlyphCacheKey, cosmic_text::SwashImage)> = buffers
                .as_ref()
                .par_iter()
                .flat_map(|buffer| buffer.layout_runs().par_bridge())
                // .iter()
                // .flat_map(|buffer| buffer.layout_runs())
                .flat_map(|line| line.glyphs)
                .map(|g| {
                    if atlas_manager.read().unwrap().has_glyph(&g.cache_key) {
                        return None;
                    }

                    match render_glyph(g.cache_key, font_system.as_ref()) {
                        Some(image) => Some((g.cache_key, image)),
                        None => {
                            log::error!("failed to render glyph {}!", g.cache_key.glyph_id);

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

fn render_glyph(
    cache_key: GlyphCacheKey,
    font_system: &Mutex<FontSystem>,
) -> Option<cosmic_text::SwashImage> {
    log::debug!("Rendering glyph {:x}", cache_key.hash_u64());

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
