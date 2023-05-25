use std::{
    borrow::{BorrowMut, Cow},
    cell::RefCell,
    collections::HashMap,
    hash::Hash,
    num::NonZeroU64,
    sync::{Arc, Mutex, RwLock},
};

use rayon::{prelude::*, ThreadPool};

use cosmic_text::{Buffer as TextBuffer, Font, FontSystem, LayoutGlyph, SwashCache};
use etagere::{AllocId, Allocation as AtlasAllocation, BucketedAtlasAllocator};
use futures::future::join_all;
use rustc_hash::FxHashMap;
use swash::scale::{Render, ScaleContext};
use tao::dpi::LogicalSize;
use wgpu::PipelineLayout;

use crate::{
    surface::{ParamsBuffer, RenderingContext},
    text::{self, GlyphContentType},
    util::PhysicalRect,
};

type GlyphCacheKey = cosmic_text::CacheKey;

pub struct GlyphToRender {
    // cache_key: GlyphCacheKey,
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
    width: u32,
    height: u32,
    shader: wgpu::ShaderModule,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,

    // font_system: Arc<Mutex<FontSystem>>,
    // scale_context: Arc<Mutex<ScaleContext>>,
    // swash_cache: SwashCache,
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
            width,
            height,
            shader,
            render_pipeline,
            bind_group,

            vertex_buffer,
            index_buffer,
            vertex_buffer_glyphs,

            // swash_cache: SwashCache::new(),
            // scale_context: Arc::new(Mutex::new(ScaleContext::new())),
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
            size: vertex_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("font atlas index buffer"),
            size: index_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        return (vertex_buffer, index_buffer);
    }

    // async fn render_glyph(&mut self, glyph: &LayoutGlyph) {
    //     let LayoutGlyph { cache_key, .. } = glyph;

    //     let font = self.font_system.lock().unwrap().get_font(cache_key.font_id);

    //     let font = match font {
    //         Some(some) => some,
    //         None => {
    //             // todo: error here
    //             // log::warn!("did not find font {:?}", cache_key.font_id);
    //             return;
    //         }
    //     };
    // }

    pub fn prepare(
        &mut self,
        RenderingContext { queue, .. }: &RenderingContext,
        glyphs: Vec<GlyphToRender>,
    ) {
        self.num_glyphs = glyphs.len() as u64;

        let mut data = Vec::<u8>::with_capacity(glyphs.len() * std::mem::size_of::<FontVertex>());

        let draw_rects = glyphs.iter().map(
            |GlyphToRender {
                 alloc: AtlasAllocation { rectangle: uv, .. },
                 draw_rect,
             }| {
                [
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
                ]
            },
        );

        for draw_rect in draw_rects {
            data.extend_from_slice(bytemuck::bytes_of(&draw_rect))
        }
        // .map(|v| bytemuck::bytes_of(&v))
        // .flatten()
        // .copied()
        // .collect();

        let indices = [0u16, 1, 2, 1, 2, 3];

        queue.write_buffer(&self.vertex_buffer, 0, data.as_slice());
        queue.write_buffer(&self.index_buffer, 0, bytemuck::bytes_of(&indices));

        // queue.write_texture(texture, data, data_layout, size)
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        let (vertex_size, index_size) = Self::buffer_sizes(self.num_glyphs);

        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(0..vertex_size));
        // render_pass.set_index_buffer(
        //     self.index_buffer.slice(0..index_size),
        //     wgpu::IndexFormat::Uint16,
        // );

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        // render_pass.draw(0..index_size as u32, 0..1);
        render_pass.draw_indexed(0..(self.num_glyphs * 6) as u32, 0, 0..1);

        // render_pass.multi_draw_indexed_indirect(indirect_buffer, indirect_offset, count)
        // render_pass.draw_indexed(indices, base_vertex, instances)
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
                    blend: Some(wgpu::BlendState::REPLACE),
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

struct GlyphAllocation(AtlasId, AtlasAllocation);

type FontAtlasCollection = HashMap<AtlasId, FontAtlas, crate::hash::BuildNoHashHasher<AtlasId>>;

struct FontAtlasManager {
    mask_atlases: FontAtlasCollection,
    color_atlases: FontAtlasCollection,

    glyphs: FxHashMap<GlyphCacheKey, GlyphAllocation>,

    id: u32,

    rendering_context: Arc<RenderingContext>,
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

    // pub fn allocate_glyph(

    // )

    fn create_atlas(&mut self, kind: GlyphContentType, size: u32) {
        let coll: &mut FontAtlasCollection = match kind {
            GlyphContentType::Color => &mut self.color_atlases,
            GlyphContentType::Mask => &mut self.mask_atlases,
        };

        let atlas_id = self.id;
        self.id += 1;

        coll.insert()
    }

    pub fn has_glyph(&self, key: &GlyphCacheKey) -> bool {
        self.glyphs.contains_key(key)
    }

    // pub fn find_or_create_atlas(
    //     &mut self,
    //     kind: GlyphContentType,
    //     size: u32,
    // ) -> Option<(AtlasId, &mut FontAtlas)> {
    // }
}

pub struct FontManager {
    font_system: Arc<Mutex<FontSystem>>,
    scale_context: Arc<Mutex<ScaleContext>>,

    atlas_manager: Arc<RwLock<FontAtlasManager>>,
}

impl FontManager {
    pub fn new(rendering_context: Arc<RenderingContext>) -> Self {
        let font_system = FontSystem::new();
        let scale_context = ScaleContext::new();

        let atlas_manager = FontAtlasManager::new(rendering_context);

        return Self {
            font_system: Arc::new(Mutex::new(font_system)),
            scale_context: Arc::new(Mutex::new(scale_context)),

            atlas_manager: Arc::new(RwLock::new(atlas_manager)),
        };
    }

    // heavy computation; should be run off main thread
    pub fn generate_textures<'a>(
        &mut self,
        glyphs: impl IntoParallelIterator<Item = &'a LayoutGlyph>,
    ) {
        let results = glyphs
            .into_par_iter()
            .filter(|g| !self.atlas_manager.read().unwrap().has_glyph(&g.cache_key))
            .map(|g| {
                let cache_key = g.cache_key;

                (
                    cache_key,
                    render_glyph(
                        cache_key,
                        self.font_system.as_ref(),
                        // self.scale_context.as_ref(),
                    ),
                )
            });

        // for glyph in glyphs {
        //     // self.swash_cache.get_image_uncached(font_system, cache_key)
        //     // join_all(iter)
        // }
    }

    // pub(crate) fn gen_atlas(&self, context: &RenderingContext) -> FontAtlas {
    //     FontAtlas::new(context, GlyphContentType::Mask, 512, 512)
    // }
}

// let mut atlas = AtlasAllocator::new(size2(1000, 1000));

// let a = atlas.allocate(size2(100, 1000)).unwrap();
// let b = atlas.allocate(size2(900, 200)).unwrap();

// atlas.deallocate(a.id);

// let c = atlas.allocate(size2(300, 200)).unwrap();

// assert_eq!(c.rectangle, atlas[c.id]);

// atlas.deallocate(c.id);
// atlas.deallocate(b.id);

thread_local! {
    static SCALE_CONTEXT: RefCell<ScaleContext> = RefCell::new(ScaleContext::new())
}

fn render_glyph(
    cache_key: GlyphCacheKey,
    font_system: &Mutex<FontSystem>,
) -> Option<cosmic_text::SwashImage> {
    use swash::{
        scale::{Render, Source, StrikeWith},
        zeno::{Format, Vector},
    };

    let font = font_system.lock().unwrap().get_font(cache_key.font_id);

    let font = match font {
        Some(some) => some,
        None => {
            // todo: error here
            // log::warn!("did not find font {:?}", cache_key.font_id);
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
