use crate::{
    color::ColorRgba,
    graphics::PushVertices,
    math::{
        PhysicalPos, PhysicalRect, PhysicalSize, PhysicalVector, Pos, Rect, ScaleFactor,
        WindowScaleFactor,
    },
    shape::BoxShaderVertex,
    surface::SurfaceDependent,
    util::text::{FontSystem, GlyphContentType, PlacedTextBox},
};

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard},
};

use rayon::prelude::*;

use etagere::{euclid::approxeq::ApproxEq, Allocation as AtlasAllocation, BucketedAtlasAllocator};
use rustc_hash::FxHashMap;
use swash::scale::ScaleContext;

use crate::{
    debug::{DebugAssert, HashU64},
    debug_panic,
    num::NextPowerOfTwo,
    surface::RenderingContext,
    util::PhysicalUnit,
};

type GlyphCacheKey = cosmic_text::CacheKey;

pub struct GlyphToRender {
    size: PhysicalSize<u32>,
    draw_rect: PhysicalRect,
    alloc: AtlasAllocation, // uv: Option<Size2>,
    color: ColorRgba,
    // clip_rect: Option<PhysicalRect>,
}

pub(crate) struct FontAtlas {
    allocator: BucketedAtlasAllocator,
    texture: wgpu::Texture,
    sampler: wgpu::Sampler,
    texture_view: wgpu::TextureView,
    atlas_type: GlyphContentType,
    width: i32,
    height: i32,

    bind_group: wgpu::BindGroup,

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

        let bind_group = Self::create_bind_group(&sampler, &texture_view, context);

        let vertex_buffer_glyphs = Self::MIN_NUM_VERTS;

        Self {
            allocator,
            texture,
            sampler,
            texture_view,
            atlas_type,
            width: width as i32,
            height: width as i32,
            bind_group,
            vertex_buffer_glyphs,
            num_glyphs: 0,
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, quads: u64) {
        // self.gpu_buffer
        //     .render_quads(None, (&self.bind_group).into(), render_pass, quads, 0..1);
    }

    fn try_allocate_space(&mut self, space: &PhysicalSize<u32>) -> Option<AtlasAllocation> {
        let space = PhysicalSize::new(space.width as i32, space.height as i32);

        if !self.can_fit(space) {
            return None;
        }

        self.allocator
            .allocate(etagere::size2(space.width, space.height))
    }

    pub fn allocate_glyph(
        &mut self,
        image: &cosmic_text::SwashImage,
        RenderingContext { queue, .. }: &RenderingContext,
    ) -> Option<AtlasAllocation> {
        let size = PhysicalSize::new(image.placement.width, image.placement.height);

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

    fn can_fit(&self, space: PhysicalSize<i32>) -> bool {
        return space.width <= self.width && space.height <= self.height;
    }

    pub fn create_bind_group(
        sampler: &wgpu::Sampler,
        texture_view: &wgpu::TextureView,
        render_ctx: &RenderingContext,
    ) -> wgpu::BindGroup {
        render_ctx.create_shape_bind_group(texture_view, sampler)
    }
}

impl SurfaceDependent for FontAtlas {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        size: winit::dpi::PhysicalSize<u32>,
        scale_factor: WindowScaleFactor,
    ) {
        let bind_group = Self::create_bind_group(&self.sampler, &self.texture_view, context);
        self.bind_group = bind_group;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AtlasId(GlyphContentType, u32);

#[derive(Clone, Copy)]
struct GlyphAllocation {
    atlas_id: AtlasId,
    allocation: AtlasAllocation,
    size: PhysicalSize<u32>,
    placement: PhysicalPos<i32>,
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
        boxes: impl IntoIterator<Item = PlacedTextBox<PhysicalUnit>>,
        output: &mut impl PushVertices<BoxShaderVertex>,
    ) {
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
                        let color = g.color;

                        let PlacedTextBox { clip_rect, pos, .. } = text_box;
                        let draw_rect = g.to_draw_glyph(pos, *size, *placement);

                        if clip_rect
                            .map(|clip_rect| clip_rect.intersection(&draw_rect).is_none())
                            .unwrap_or_default()
                        {
                            continue;
                        }

                        let uv = allocation.rectangle;

                        let alloc_pos = PhysicalPos::new(uv.min.x as u32, uv.min.y as u32);
                        let uv = PhysicalRect::new(alloc_pos, alloc_pos + *size);
                        let color = (*color).into();

                        let (vertices, indices) =
                            BoxShaderVertex::glyph_rect(draw_rect, uv, atlas_id.0, color);

                        output.push_vertices(vertices, indices);
                    }
                    None => log::trace!("Glyph {} not cached", g.cache_key.glyph_id),
                    Some(GlyphCacheEntry::Noop) => {}
                }
            }
        }
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
        // glyph_size: PhysicalSize<u32>,
        image: cosmic_text::SwashImage,
        cache_key: GlyphCacheKey,
    ) -> Option<GlyphAllocation> {
        let glyph_size = PhysicalSize::<u32>::new(image.placement.width, image.placement.height);

        let glyph_placement = PhysicalPos::<i32>::new(image.placement.left, image.placement.top);

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

        alloc
    }

    pub fn get_glyph_uv() {}
}

impl SurfaceDependent for FontAtlasManager {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        size: winit::dpi::PhysicalSize<u32>,
        scale_factor: WindowScaleFactor,
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
        text_box: PlacedTextBox<PhysicalUnit>,
        output: &mut impl PushVertices<BoxShaderVertex>,
    ) {
        self.generate_textures(text_box.glyph_cache_keys().collect());

        self.atlas_manager
            .write()
            .unwrap()
            .prepare([text_box], output);
    }

    pub fn get_font_system_ref(&self) -> Arc<Mutex<FontSystem>> {
        self.font_system.clone()
    }

    pub fn get_font_system(&mut self) -> MutexGuard<'_, FontSystem> {
        return self.font_system.lock().unwrap();
    }

    // pub fn render<'a, 'b, 'c>(
    //     &self,
    //     render_pass: &'a mut wgpu::RenderPass<'b>,
    //     batch: &BatchedAtlasRender,
    // ) {
    //     self.atlas_manager
    //         .write()
    //         .render(render_pass, batch.atlas_id, batch.num_quads);
    // }

    fn generate_textures_worker(
        mut glyphs: HashSet<GlyphCacheKey>,
        atlas_manager: Arc<RwLock<FontAtlasManager>>,
        font_system: Arc<Mutex<FontSystem>>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        let drain_iter = glyphs.par_drain();

        // TODO: support rayon on wasm32
        #[cfg(target_arch = "wasm32")]
        let drain_iter = glyphs.drain();

        let results: Vec<(GlyphCacheKey, cosmic_text::SwashImage)> = drain_iter
            .map(|g| {
                if atlas_manager.read().unwrap().has_glyph(&g) {
                    return None;
                }

                match rasterize_glyph(&g, font_system.as_ref()) {
                    Some(image) => {
                        log::trace!("rasterized glyph {:?}", g.glyph_id);
                        Some((g, image))
                    }
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
    }

    pub fn generate_textures<'a>(&mut self, mut glyphs: HashSet<GlyphCacheKey>) {
        let atlas_manager = self.atlas_manager.clone();
        let font_system = self.font_system.clone();

        // TODO: support threading on wasm
        #[cfg(target_arch = "wasm32")]
        {
            Self::generate_textures_worker(glyphs, atlas_manager, font_system);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                Self::generate_textures_worker(glyphs, atlas_manager, font_system);
            });
        }
    }
}

thread_local! {
    static SCALE_CONTEXT: RefCell<ScaleContext> = RefCell::new(ScaleContext::new())
}

fn rasterize_glyph(
    cache_key: &GlyphCacheKey,
    font_system: &Mutex<FontSystem>,
) -> Option<cosmic_text::SwashImage> {
    log::trace!("Rasterizing glyph {:x}", cache_key.hash_u64());

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
        scale_factor: WindowScaleFactor,
    ) {
        self.atlas_manager
            .write()
            .unwrap()
            .reconfigure(context, size, scale_factor)
    }
}
