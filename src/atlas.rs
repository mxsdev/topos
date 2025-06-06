use crate::{
    color::{ColorRgb, ColorRgba},
    graphics::PushVertices,
    math::{PhysicalPos, PhysicalRect, PhysicalSize, Pos, Rect, ScaleFactor, Sides, Size},
    shape::BoxShaderVertex,
    texture::{TextureManagerError, TextureManagerRef, TextureRef, TextureWeakRef},
    util::{
        guard::{ReadLockable, WritableLock, WriteLockable},
        text::{AtlasContentType, FontSystem, FontSystemRef, PlacedTextBox},
    },
};

use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, RwLock, Weak},
};

use std::sync::mpsc;

use cosmic_text::fontdb;
use itertools::Itertools;
use palette::{Alpha, IntoColor};
use rayon::prelude::*;

use etagere::{AllocId as EtagereAllocId, Allocation as EtagereAllocation, BucketedAtlasAllocator};
use rustc_hash::{FxBuildHasher, FxHashMap, FxHasher};
use shrinkwraprs::Shrinkwrap;
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
    alloc: EtagereAllocation, // uv: Option<Size2>,
    color: ColorRgba,
    // clip_rect: Option<PhysicalRect>,
}

pub struct TextureAtlas {
    allocator: BucketedAtlasAllocator,

    texture_ref: TextureRef,

    atlas_type: AtlasContentType,
    width: i32,
    height: i32,

    num_glyphs: usize,
}

impl TextureAtlas {
    const TEXTURE_PADDING: u32 = 1;

    fn new(
        context: &RenderingContext,
        texture_manager: &TextureManagerRef,
        atlas_type: AtlasContentType,
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
                AtlasContentType::Mask => wgpu::TextureFormat::R8Unorm,
                AtlasContentType::Color => wgpu::TextureFormat::Rgba8UnormSrgb,
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

    fn try_allocate_space(&mut self, space: &PhysicalSize<u32>) -> Option<EtagereAllocation> {
        let padded_space = *space + PhysicalSize::splat(Self::TEXTURE_PADDING * 2);

        let space = PhysicalSize::new(padded_space.width as i32, padded_space.height as i32);

        if !self.can_fit(space) {
            return None;
        }

        let allocation = self
            .allocator
            .allocate(etagere::size2(space.width, space.height))?;

        self.num_glyphs += 1;

        allocation.into()
    }

    pub fn get_texture_ref(&self) -> &TextureRef {
        &self.texture_ref
    }

    pub fn write_texture(
        &self,
        rendering_context: &RenderingContext,
        alloc: &impl HasAtlasAllocationId,
        image_data: &[u8],
        // image_size: PhysicalSize<u32>,
    ) {
        let bytes_per_pixel = self.atlas_type.num_channels() * self.atlas_type.bytes_per_channel();

        let alloc_id = alloc.get_id();

        let rect = alloc_id.draw_rect();

        let image_width = rect.width() as u32;
        let image_height = rect.height() as u32;

        rendering_context.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture_ref.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.min.x as u32,
                    y: rect.min.y as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::default(),
            },
            &image_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: (image_width * bytes_per_pixel).into(),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: image_width,
                height: image_height,
                depth_or_array_layers: 1,
            },
        );

        enum Sides {
            HorizontalTop,
            HorizontalBottom,
            VerticalLeft,
            VerticalRight,
        }

        impl Sides {
            fn origin(&self, rect: &PhysicalRect<u32>, padding: u32) -> wgpu::Origin3d {
                match self {
                    Self::HorizontalTop => wgpu::Origin3d {
                        x: rect.min.x,
                        y: rect.min.y,
                        z: 0,
                    },
                    Self::HorizontalBottom => wgpu::Origin3d {
                        x: rect.min.x,
                        y: rect.max.y - padding,
                        z: 0,
                    },
                    Self::VerticalLeft => wgpu::Origin3d {
                        x: rect.min.x,
                        y: rect.min.y,
                        z: 0,
                    },
                    Self::VerticalRight => wgpu::Origin3d {
                        x: rect.max.x - padding,
                        y: rect.min.y,
                        z: 0,
                    },
                }
            }

            fn extend_3d(&self, rect: &PhysicalRect<u32>, padding: u32) -> wgpu::Extent3d {
                match self {
                    Self::HorizontalTop | Self::HorizontalBottom => wgpu::Extent3d {
                        width: rect.width(),
                        height: padding,
                        depth_or_array_layers: 1,
                    },
                    Self::VerticalLeft | Self::VerticalRight => wgpu::Extent3d {
                        width: padding,
                        height: rect.height(),
                        depth_or_array_layers: 1,
                    },
                }
            }

            fn pixels_per_row(&self, rect: &PhysicalRect<u32>) -> u32 {
                match self {
                    Self::HorizontalTop | Self::HorizontalBottom => rect.width(),
                    Self::VerticalLeft | Self::VerticalRight => 1,
                }
            }
        }

        if alloc_id.padding != 0 {
            let rect = alloc_id.atlas_rect.map(|x| x as u32);

            let image_width = rect.width() as u32;
            let image_height = rect.height() as u32;

            let image_pixels = image_width.max(image_height);
            let zeroed = vec![0u8; image_pixels as usize * bytes_per_pixel as usize];

            for side in [
                Sides::HorizontalTop,
                Sides::HorizontalBottom,
                Sides::VerticalLeft,
                Sides::VerticalRight,
            ] {
                rendering_context.queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &self.texture_ref.texture,
                        mip_level: 0,
                        origin: side.origin(&rect, alloc_id.padding),
                        aspect: wgpu::TextureAspect::default(),
                    },
                    &bytemuck::cast_slice(&zeroed),
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: (side.pixels_per_row(&rect) * bytes_per_pixel).into(),
                        rows_per_image: None,
                    },
                    side.extend_3d(&rect, alloc_id.padding),
                )
            }
        }
    }

    fn deallocate_glyph(&mut self, alloc: EtagereAllocId) {
        self.num_glyphs -= 1;
        self.allocator.deallocate(alloc);
    }

    fn can_fit(&self, space: PhysicalSize<i32>) -> bool {
        return space.width <= self.width && space.height <= self.height;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AtlasId(AtlasContentType, u32);

#[derive(Debug, Default)]
struct DeallocationQueue {
    inner: Arc<Mutex<VecDeque<(AtlasId, EtagereAllocId)>>>,
}

impl DeallocationQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn drain(&self) -> Vec<(AtlasId, EtagereAllocId)> {
        self.inner.lock().unwrap().drain(..).collect()
    }

    pub fn get_ref(&self) -> DeallocationQueueSender {
        DeallocationQueueSender {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

#[derive(Debug)]
struct DeallocationQueueSender {
    inner: Weak<Mutex<VecDeque<(AtlasId, EtagereAllocId)>>>,
}

impl DeallocationQueueSender {
    pub fn send(&self, data: (AtlasId, EtagereAllocId)) {
        if let Some(inner) = self.inner.upgrade() {
            inner.lock().unwrap().push_back(data);
        }
    }
}

pub trait HasAtlasAllocationId {
    fn get_id(&self) -> AtlasAllocationId;
}

#[derive(Debug, Clone, Copy)]
pub struct AtlasAllocationId {
    pub(crate) atlas_id: AtlasId,
    pub(crate) allocation: EtagereAllocation,
    pub(crate) atlas_rect: PhysicalRect<i32>,
    pub(crate) padding: u32,
}

impl AtlasAllocationId {
    pub(crate) fn draw_rect(&self) -> PhysicalRect<i32> {
        return self.atlas_rect.inner_box(Sides::splat(self.padding as i32));
    }
}

impl HasAtlasAllocationId for AtlasAllocationId {
    fn get_id(&self) -> AtlasAllocationId {
        *self
    }
}

#[derive(Debug)]
pub struct AtlasAllocation {
    pub(crate) alloc_id: AtlasAllocationId,
    deallocation_queue_sender: DeallocationQueueSender,
}

impl AtlasAllocation {
    fn new(
        atlas_id: AtlasId,
        allocation: EtagereAllocation,
        atlas_rect: PhysicalRect<i32>,
        deallocation_queue_sender: DeallocationQueueSender,
        padding: u32,
    ) -> Self {
        Self {
            deallocation_queue_sender,
            alloc_id: AtlasAllocationId {
                atlas_id,
                allocation,
                atlas_rect,
                padding,
            },
        }
    }

    #[inline(always)]
    pub fn atlas_id(&self) -> AtlasId {
        self.alloc_id.atlas_id
    }

    #[inline(always)]
    fn etagere_allocation(&self) -> EtagereAllocation {
        self.alloc_id.allocation
    }

    #[inline(always)]
    pub fn draw_rect(&self) -> PhysicalRect<i32> {
        self.alloc_id.draw_rect()
    }

    // // FIXME: make this return a reference
    // #[inline(always)]
    // pub fn get_id(&self) -> AtlasAllocationId {
    //     self.alloc_id
    // }
}

impl Drop for AtlasAllocation {
    fn drop(&mut self) {
        self.deallocation_queue_sender
            .send((self.atlas_id(), self.etagere_allocation().id))
    }
}

impl HasAtlasAllocationId for AtlasAllocation {
    fn get_id(&self) -> AtlasAllocationId {
        self.alloc_id
    }
}

struct GlyphAllocation {
    // atlas_id: AtlasId,
    // allocation: EtagereAllocation,
    allocation: AtlasAllocation,
    size: PhysicalSize<u32>,
    placement: PhysicalPos<i32>,
}

enum GlyphCacheEntry {
    GlyphAllocation(GlyphAllocation),
    Noop,
}

// TODO: use no hash hasher
type FontAtlasCollection = HashMap<AtlasId, TextureAtlas>;

#[derive(Shrinkwrap, Clone)]
pub struct TextureAtlasManagerRef(Arc<RwLock<TextureAtlasManager>>);

impl From<Arc<RwLock<TextureAtlasManager>>> for TextureAtlasManagerRef {
    #[inline(always)]
    fn from(value: Arc<RwLock<TextureAtlasManager>>) -> Self {
        Self(value)
    }
}

impl From<TextureAtlasManager> for TextureAtlasManagerRef {
    fn from(value: TextureAtlasManager) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
}

impl ReadLockable<TextureAtlasManager> for &TextureAtlasManagerRef {
    fn read_lock(&self) -> crate::util::guard::ReadableLock<'_, TextureAtlasManager> {
        self.0.deref().read_lock()
    }
}

impl WriteLockable<TextureAtlasManager> for &TextureAtlasManagerRef {
    fn write_lock(&mut self) -> crate::util::guard::WritableLock<'_, TextureAtlasManager> {
        WritableLock::Rw(self.0.deref().write().unwrap())
    }
}

#[derive(Default)]
struct GlyphCache {
    glyphs: FxHashMap<GlyphCacheKey, GlyphCacheEntry>,

    // TODO: btreeset is excessive, just use a list?
    glyph_btreemap: FxHashMap<(fontdb::ID, u16), BTreeSet<GlyphCacheKey>>,
}

impl GlyphCache {
    fn get(&self, cache_key: &GlyphCacheKey) -> Option<&GlyphCacheEntry> {
        self.glyphs.get(cache_key)
    }

    fn contains_key(&self, key: &GlyphCacheKey) -> bool {
        self.glyphs.contains_key(key)
    }

    fn insert(&mut self, cache_key: cosmic_text::CacheKey, noop: GlyphCacheEntry) {
        self.glyphs.insert(cache_key, noop);
        self.glyph_btreemap.entry((cache_key.font_id, cache_key.glyph_id)).or_default().insert(cache_key);
    }

    fn remove(&mut self, cache_key: &GlyphCacheKey) -> Option<GlyphCacheEntry> {
        let entry = self.glyphs.remove(cache_key);

        let mut destroy = false;
        self.glyph_btreemap.entry((cache_key.font_id, cache_key.glyph_id)).and_modify(|set| {
            set.remove(cache_key);
            destroy = set.is_empty();
        });

        if destroy {
            self.glyph_btreemap.remove(&(cache_key.font_id, cache_key.glyph_id));
        }

        entry
    }

    fn keys(&self) -> impl Iterator<Item = &GlyphCacheKey> {
        self.glyphs.keys()
    }

    fn clear(&mut self) {
        self.glyphs.clear();
        self.glyph_btreemap.clear();
    }

    fn find_closest_key(&self, key: &GlyphCacheKey) -> Option<(&GlyphCacheKey, &GlyphCacheEntry)> {
        self.glyph_btreemap.get(&(key.font_id, key.glyph_id))
            .and_then(|set| set.last())
            .and_then(|key| self.glyphs.get(key).map(|entry| (key, entry)))
    }
}

pub struct TextureAtlasManager {
    atlases: FontAtlasCollection,

    glyphs: GlyphCache,

    used_glyphs_this_frame: HashSet<GlyphCacheKey, FxBuildHasher>,

    id: u32,

    rendering_context: Arc<RenderingContext>,

    deallocation_queue: DeallocationQueue,
}

impl TextureAtlasManager {
    pub(crate) fn new(rendering_context: Arc<RenderingContext>) -> Self {
        return Self {
            atlases: Default::default(),

            glyphs: Default::default(),
            used_glyphs_this_frame: Default::default(),

            id: 0,

            rendering_context,

            deallocation_queue: Default::default(),
        };
    }

    pub(crate) fn prepare<'a>(
        &'a mut self,
        boxes: impl IntoIterator<Item = PlacedTextBox> + 'a,
        output: &'a mut impl PushVertices<BoxShaderVertex>,
    ) -> impl Iterator<Item = GlyphCacheKey> + 'a {
        boxes
            .into_iter()
            .flat_map(move |text_box| {
                text_box.glyphs.into_iter().map(move |g| {
                    (
                        g,
                        text_box.clip_rect,
                        text_box.pos,
                        text_box.scale_fac,
                        text_box.bounding_size,
                    )
                })
            })
            .filter_map(|(g, clip_rect, pos, scale_fac, bounding_size)| {
                let alloc = self.glyphs.get(&g.glyph.cache_key).map(|entry| (&g.glyph.cache_key, entry, false))
                    .or_else(|| {
                        self.glyphs.find_closest_key(&g.glyph.cache_key)
                            .map(|(key, entry)| (key, entry, true))
                    });

                match alloc {
                    Some((key, GlyphCacheEntry::GlyphAllocation(GlyphAllocation {
                        size,
                        allocation,
                        placement,
                        ..
                    }), is_fallback)) => {
                        self.used_glyphs_this_frame.insert(*key);

                        if let Some(atlas) = self.get_atlas_by_id(&allocation.atlas_id()) {
                            let color = g.color;

                            let (adjusted_size, placement) = if is_fallback {
                                let scale_fac = f32::from_bits(g.glyph.cache_key.font_size_bits) / f32::from_bits(key.font_size_bits);

                                (
                                    size.map(|x| (x as f32 * scale_fac).round() as u32), 
                                    placement.map(|x| (x as f32 * scale_fac).round() as i32)
                                )
                            } else {
                                (*size, *placement)
                            };

                            let draw_rect =
                                g.to_draw_glyph(pos, adjusted_size, placement, scale_fac.inverse());

                            if clip_rect
                                .map(|clip_rect| clip_rect.inner.intersection(&draw_rect).is_none())
                                .unwrap_or_default()
                            {
                                return None;
                            }

                            let uv = allocation.draw_rect();

                            let alloc_pos = Pos::new(uv.min.x as u32, uv.min.y as u32);
                            let uv = PhysicalRect::new(alloc_pos, alloc_pos + *size);

                            let (vertices, indices) = BoxShaderVertex::glyph_rect(
                                self as &TextureAtlasManager,
                                draw_rect,
                                uv,
                                allocation.atlas_id().0,
                                color,
                                &atlas.texture_ref,
                                Rect::from_min_size(pos, bounding_size),
                            );

                            output.push_vertices(vertices, indices);

                            return Some(g.glyph.cache_key);
                        }
                    }
                    None => log::warn!("Glyph {} not cached, it will not be rendered this frame", g.glyph.cache_key.glyph_id),
                    Some((_, GlyphCacheEntry::Noop, _)) => {}
                };

                None
            })
    }

    pub(crate) fn get_atlas_mut(&mut self, id: &AtlasId) -> Option<&mut TextureAtlas> {
        self.atlases.get_mut(id)
    }

    pub(crate) fn get_atlas_by_id(&self, id: &AtlasId) -> Option<&TextureAtlas> {
        self.atlases.get(id)
    }

    pub fn get_atlas(&self, alloc: &impl HasAtlasAllocationId) -> &TextureAtlas {
        self.get_atlas_by_id(&alloc.get_id().atlas_id).unwrap()
    }

    fn create_atlas(
        &mut self,
        texture_manager: &TextureManagerRef,
        kind: AtlasContentType,
        size: u32,
    ) -> Result<AtlasId, TextureManagerError> {
        let size = u32::max(size, 512);

        log::trace!("Creating new atlas of size {size}");

        let atlas_id = AtlasId(kind, self.id);
        self.id += 1;

        let atlas = TextureAtlas::new(&self.rendering_context, texture_manager, kind, size, size)?;

        self.atlases.insert(atlas_id, atlas);

        Ok(atlas_id)
    }

    pub fn has_glyph(&self, key: &GlyphCacheKey) -> bool {
        self.glyphs.contains_key(key)
    }

    pub fn allocate(
        &mut self,
        texture_manager: &TextureManagerRef,
        kind: AtlasContentType,
        size: PhysicalSize<u32>,
    ) -> Option<AtlasAllocation> {
        self.atlases
            .iter_mut()
            .filter_map(|(id, atlas)| match id.0.eq(&kind) {
                true => Some(atlas.try_allocate_space(&size).map(|res| (*id, res))),
                false => None,
            })
            .flatten()
            .next()
            .or_else(|| {
                // let new_dimensions = u32::max(size.width, size.height).next_power_of_2();
                let atlas_id = self.create_atlas(texture_manager, kind, 4096).unwrap();

                match self.get_atlas_mut(&atlas_id) {
                    Some(atlas) => match atlas.try_allocate_space(&size) {
                        Some(res) => Some((atlas_id, res)),
                        None => {
                            // log::error!(
                            //     "Failed to allocate space for glyph {:x}",
                            //     cache_key.hash_u64()
                            // );

                            None
                        }
                    },
                    None => {
                        debug_panic!("Failed to get atlas for glyph");

                        None
                    }
                }
            })
            .map(|(atlas_id, alloc)| {
                AtlasAllocation::new(
                    atlas_id,
                    alloc,
                    // TODO: impl Into for euclid
                    Rect::from_min_size(
                        Pos::new(alloc.rectangle.min.x, alloc.rectangle.min.y),
                        size.map(|x| x as i32 + 2 * TextureAtlas::TEXTURE_PADDING as i32),
                    ),
                    self.deallocation_queue.get_ref(),
                    TextureAtlas::TEXTURE_PADDING,
                )
            })
    }

    pub(crate) fn allocate_glyph(
        &mut self,
        texture_manager: &TextureManagerRef,
        kind: AtlasContentType,
        image: cosmic_text::SwashImage,
        cache_key: GlyphCacheKey,
    ) {
        let glyph_size = PhysicalSize::<u32>::new(image.placement.width, image.placement.height);
        let glyph_placement = PhysicalPos::<i32>::new(image.placement.left, image.placement.top);

        if glyph_size.is_empty() {
            self.glyphs.insert(cache_key, GlyphCacheEntry::Noop);
            return;
        }

        let alloc = self
            .allocate(texture_manager, kind, glyph_size)
            .map(|allocation| {
                let atlas = self.atlases.get(&allocation.atlas_id()).unwrap();

                atlas.write_texture(
                    &self.rendering_context,
                    &allocation,
                    &image.data,
                    // Size::new(image.placement.width, image.placement.height),
                );

                allocation
            })
            .map(|allocation| GlyphAllocation {
                allocation,
                size: glyph_size,
                placement: glyph_placement,
            });

        match alloc {
            Some(alloc) => {
                let alloc_id = alloc.allocation.atlas_id();

                self.glyphs
                    .insert(cache_key, GlyphCacheEntry::GlyphAllocation(alloc));

                let _atlas = self.get_atlas_mut(&alloc_id).debug_assert();
            }
            _ => {}
        }
    }

    fn collect_garbage(&mut self) {
        let mut glyphs_to_remove: HashSet<_> = self.glyphs.keys().copied().collect();
        glyphs_to_remove.retain(|key| !self.used_glyphs_this_frame.contains(key));

        for key in glyphs_to_remove { self.glyphs.remove(&key); }

        self.used_glyphs_this_frame.clear();

        self.atlases.retain(|_, atlas| atlas.num_glyphs > 0);

        for (atlas_id, alloc_id) in self.deallocation_queue.drain() {
            if let Some(atlas) = self.get_atlas_mut(&atlas_id) {
                atlas.deallocate_glyph(alloc_id);
            }
        }

        // log::debug!(
        //     "Atlas GC result: total number of atlases: {}, allocated glyphs: {}, total glyphs: {}",
        //     self.atlases.len(),
        //     self.glyphs.len(),
        //     self.atlases.values().map(|x| x.num_glyphs).sum::<usize>()
        // )
    }
}

pub struct FontManager {
    font_system: FontSystemRef,
    atlas_manager: TextureAtlasManagerRef,
    texture_manager: TextureManagerRef,
}

impl FontManager {
    pub fn new(
        rendering_context: Arc<RenderingContext>,
        texture_manager: TextureManagerRef,
    ) -> Self {
        let font_system = FontSystem::new().into();

        let atlas_manager = TextureAtlasManager::new(rendering_context).into();

        return Self {
            font_system,
            atlas_manager,
            texture_manager,
        };
    }

    pub fn process_glyphs(&mut self, text_box: &PlacedTextBox) {
        let glyph_cache_keys: HashSet<_> =
            text_box.glyphs.iter().map(|g| g.glyph.cache_key).collect();

        self.atlas_manager
            .write()
            .unwrap()
            .used_glyphs_this_frame
            .extend(glyph_cache_keys.iter());

        self.generate_textures(glyph_cache_keys);
    }

    pub fn prepare<'a>(
        &mut self,
        text_box: PlacedTextBox,
        output: &mut impl PushVertices<BoxShaderVertex>,
    ) {
        // if (text_box.clip_rect.map(|x| x.is_empty()).unwrap_or_default()) {
        //     return;
        // }

        self.process_glyphs(&text_box);

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
        mut atlas_manager: impl WriteLockable<TextureAtlasManager> + Sync,
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
                if atlas_manager.read_lock().has_glyph(&cache_key) {
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
                cosmic_text::SwashContent::Mask => Some(AtlasContentType::Mask),
                cosmic_text::SwashContent::Color => Some(AtlasContentType::Color),
                cosmic_text::SwashContent::SubpixelMask => {
                    debug_panic!("Found subpixel mask!");
                    None
                }
            } {
                atlas_manager
                    .write_lock()
                    .allocate_glyph(&texture_manager, kind, image, cache_key);
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
                Self::generate_textures_worker(
                    glyphs,
                    &atlas_manager,
                    font_system,
                    texture_manager,
                );
            });
        }
    }

    pub(crate) fn atlas_manager_ref(&self) -> TextureAtlasManagerRef {
        self.atlas_manager.clone()
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
