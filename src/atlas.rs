use crate::{
    color::ColorRgba,
    graphics::PushVertices,
    math::{PhysicalPos, PhysicalRect, PhysicalSize, Pos, Rect},
    shape::BoxShaderVertex,
    texture::{TextureManagerError, TextureManagerRef, TextureRef},
    util::text::{FontSystem, FontSystemRef, GlyphContentType, PlacedTextBox},
};

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
};

use std::sync::mpsc;

use itertools::Itertools;
use rayon::prelude::*;

use etagere::{AllocId, Allocation as AtlasAllocation, BucketedAtlasAllocator};
use rustc_hash::FxHashMap;
use swash::scale::ScaleContext;

use crate::{
    debug::{DebugAssert, HashU64},
    debug_panic,
    num::NextPowerOfTwo,
    surface::RenderingContext,
};

type GlyphCacheKey = cosmic_text::CacheKey;

const MAX_ATLAS_SIZE: u32 = 4096;

pub struct GlyphToRender {
    size: PhysicalSize<u32>,
    draw_rect: Rect,
    alloc: AtlasAllocation, // uv: Option<Size2>,
    color: ColorRgba,
    // clip_rect: Option<PhysicalRect>,
}

pub(crate) struct FontAtlas {
    allocator: BucketedAtlasAllocator,

    texture_ref: TextureRef,

    atlas_type: GlyphContentType,
    width: i32,
    height: i32,

    num_glyphs: usize,
}

impl FontAtlas {
    pub fn new(
        context: &RenderingContext,
        texture_manager: &TextureManagerRef,
        atlas_type: GlyphContentType,
        width: u32,
        height: u32,
    ) -> Result<Self, TextureManagerError> {
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

        let texture_ref = texture_manager.write().unwrap().register_texture(texture)?;

        Ok(Self {
            allocator,
            atlas_type,
            width: width as i32,
            height: width as i32,
            texture_ref,
            num_glyphs: 0,
        })
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
                texture: &self.texture_ref.texture,
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

        self.num_glyphs += 1;

        Some(alloc)
    }

    pub fn deallocate_glyph(&mut self, alloc: AllocId) {
        self.num_glyphs -= 1;
        self.allocator.deallocate(alloc);
    }

    fn can_fit(&self, space: PhysicalSize<i32>) -> bool {
        return space.width <= self.width && space.height <= self.height;
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
    // mask_atlases: FontAtlasCollection,
    // color_atlases: FontAtlasCollection,
    atlases: FontAtlasCollection,

    glyphs: FxHashMap<GlyphCacheKey, GlyphCacheEntry>,
    used_glyphs_this_frame: HashSet<GlyphCacheKey>,

    id: u32,

    rendering_context: Arc<RenderingContext>,
}

impl FontAtlasManager {
    pub fn new(rendering_context: Arc<RenderingContext>) -> Self {
        return Self {
            atlases: Default::default(),

            glyphs: Default::default(),
            used_glyphs_this_frame: Default::default(),

            id: 0,

            rendering_context,
        };
    }

    pub fn prepare<'a>(
        &'a mut self,
        boxes: impl IntoIterator<Item = PlacedTextBox> + 'a,
        output: &'a mut impl PushVertices<BoxShaderVertex>,
    ) -> impl Iterator<Item = GlyphCacheKey> + 'a {
        boxes
            .into_iter()
            .flat_map(move |text_box| {
                text_box
                    .glyphs
                    .into_iter()
                    .map(move |g| (g, text_box.clip_rect, text_box.pos, text_box.scale_fac))
            })
            .filter_map(|(g, clip_rect, pos, scale_fac)| {
                let alloc = self.glyphs.get(&g.cache_key);

                match alloc {
                    Some(GlyphCacheEntry::GlyphAllocation(GlyphAllocation {
                        atlas_id,
                        size,
                        allocation,
                        placement,
                        ..
                    })) => {
                        if let Some(atlas) = self.get_atlas(atlas_id) {
                            let color = g.color;

                            // FIXME: scale this properly
                            let draw_rect = g.to_draw_glyph(
                                pos,
                                ((*size).map(|x| x as f32) / scale_fac)
                                    .map(|x| x.round() as u32)
                                    .cast_unit(),
                                ((*placement).map(|x| x as f32) / scale_fac)
                                    .map(|x| x.round() as i32)
                                    .cast_unit(),
                            );

                            if clip_rect
                                .map(|clip_rect| clip_rect.inner.intersection(&draw_rect).is_none())
                                .unwrap_or_default()
                            {
                                return None;
                            }

                            let uv = allocation.rectangle;

                            let alloc_pos = Pos::new(uv.min.x as u32, uv.min.y as u32);
                            let uv = PhysicalRect::new(alloc_pos, alloc_pos + *size);
                            let color = (*color).into();

                            let (vertices, indices) = BoxShaderVertex::glyph_rect(
                                draw_rect,
                                uv,
                                atlas_id.0,
                                color,
                                &atlas.texture_ref,
                            );

                            output.push_vertices(vertices, indices);

                            return Some(g.cache_key);
                        }
                    }
                    None => log::trace!("Glyph {} not cached", g.cache_key.glyph_id),
                    Some(GlyphCacheEntry::Noop) => {}
                };

                None
            })
    }

    fn get_atlas_mut(&mut self, id: &AtlasId) -> Option<&mut FontAtlas> {
        self.atlases.get_mut(id)
    }

    fn get_atlas(&self, id: &AtlasId) -> Option<&FontAtlas> {
        self.atlases.get(id)
    }

    fn create_atlas(
        &mut self,
        texture_manager: &TextureManagerRef,
        kind: GlyphContentType,
        size: u32,
    ) -> Result<AtlasId, TextureManagerError> {
        let size = u32::max(size, 512);

        log::trace!("Creating new atlas of size {size}");

        let atlas_id = AtlasId(kind, self.id);
        self.id += 1;

        let atlas = FontAtlas::new(&self.rendering_context, texture_manager, kind, size, size)?;

        self.atlases.insert(atlas_id, atlas);

        Ok(atlas_id)
    }

    pub fn has_glyph(&self, key: &GlyphCacheKey) -> bool {
        self.glyphs.contains_key(key)
    }

    pub fn allocate_glyph(
        &mut self,
        texture_manager: &TextureManagerRef,
        kind: GlyphContentType,
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

        let alloc = self
            .atlases
            .iter_mut()
            .filter_map(|(id, atlas)| match id.0.eq(&kind) {
                true => Some(
                    atlas
                        .allocate_glyph(&image, &rendering_context)
                        .map(|res| (*id, res)),
                ),
                false => None,
            })
            .flatten()
            .next()
            .or_else(|| {
                let size = u32::max(glyph_size.width, glyph_size.height).next_power_of_2();
                let atlas_id = self.create_atlas(texture_manager, kind, size).unwrap();

                match self.get_atlas_mut(&atlas_id) {
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

        let _atlas = self.get_atlas_mut(&alloc?.atlas_id).debug_assert()?;

        alloc
    }

    pub fn collect_garbage(&mut self) {
        let mut glyphs_to_remove: HashSet<_> = self.glyphs.keys().copied().collect();
        glyphs_to_remove.retain(|key| !self.used_glyphs_this_frame.contains(key));

        for key in glyphs_to_remove.iter() {
            if let Some((atlas_id, allocation)) = match self.glyphs.get(key).unwrap() {
                GlyphCacheEntry::GlyphAllocation(alloc) => Some((alloc.atlas_id, alloc.allocation)),
                GlyphCacheEntry::Noop => None,
            } {
                if let Some(atlas) = self.get_atlas_mut(&atlas_id) {
                    atlas.deallocate_glyph(allocation.id);
                }
            }
        }

        self.glyphs.retain(|key, _| !glyphs_to_remove.contains(key));
        self.used_glyphs_this_frame.clear();

        self.atlases.retain(|_, atlas| atlas.num_glyphs > 0);
    }
}

pub struct FontManager {
    font_system: FontSystemRef,
    atlas_manager: Arc<RwLock<FontAtlasManager>>,
    texture_manager: TextureManagerRef,
}

impl FontManager {
    pub fn new(
        rendering_context: Arc<RenderingContext>,
        texture_manager: TextureManagerRef,
    ) -> Self {
        let font_system = FontSystem::new().into();

        let atlas_manager = FontAtlasManager::new(rendering_context);

        return Self {
            font_system,
            atlas_manager: Arc::new(RwLock::new(atlas_manager)),
            texture_manager,
        };
    }

    pub fn prepare<'a>(
        &mut self,
        text_box: PlacedTextBox,
        output: &mut impl PushVertices<BoxShaderVertex>,
    ) {
        // if (text_box.clip_rect.map(|x| x.is_empty()).unwrap_or_default()) {
        //     return;
        // }

        let glyph_cache_keys: HashSet<_> = text_box.glyphs.iter().map(|g| g.cache_key).collect();

        self.atlas_manager
            .write()
            .unwrap()
            .used_glyphs_this_frame
            .extend(glyph_cache_keys.iter());

        self.generate_textures(glyph_cache_keys);

        self.atlas_manager
            .write()
            .unwrap()
            .prepare([text_box], output)
            // TODO: this shouldnt be needed
            .for_each(drop);
    }

    pub fn get_font_system_ref(&self) -> FontSystemRef {
        self.font_system.clone()
    }

    pub fn get_font_system(&mut self) -> &FontSystemRef {
        return &self.font_system;
    }

    pub fn collect_garbage(&mut self) {
        self.atlas_manager.write().unwrap().collect_garbage();
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
        font_system: FontSystemRef,
        texture_manager: TextureManagerRef,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        let drain_iter = glyphs.par_drain();

        // TODO: support rayon on wasm32
        #[cfg(target_arch = "wasm32")]
        let drain_iter = glyphs.drain();

        let results: Vec<(GlyphCacheKey, cosmic_text::SwashImage)> = drain_iter
            .map(|cache_key| {
                if atlas_manager.read().unwrap().has_glyph(&cache_key) {
                    return None;
                }

                match rasterize_glyph(&cache_key, font_system.as_ref()) {
                    Some(image) => {
                        log::trace!("rasterized glyph {:?}", cache_key.glyph_id);
                        Some((cache_key, image))
                    }
                    None => {
                        log::error!("failed to render glyph {}!", cache_key.glyph_id);

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
                atlas_manager.write().unwrap().allocate_glyph(
                    &texture_manager,
                    kind,
                    image,
                    cache_key,
                );
            }
        }
    }

    pub fn generate_textures<'a>(&mut self, glyphs: HashSet<GlyphCacheKey>) {
        let atlas_manager = self.atlas_manager.clone();
        let font_system = self.font_system.clone();
        let texture_manager = self.texture_manager.clone();

        // TODO: support threading on wasm
        #[cfg(target_arch = "wasm32")]
        {
            Self::generate_textures_worker(glyphs, atlas_manager, font_system);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                Self::generate_textures_worker(glyphs, atlas_manager, font_system, texture_manager);
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
