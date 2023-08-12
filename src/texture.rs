use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, RwLock, Weak,
};

use itertools::Itertools;
use shrinkwraprs::Shrinkwrap;

use crate::surface::RenderingContext;

use drain_filter_polyfill::VecExt;

#[derive(Debug)]
pub struct TextureRefInner {
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub(crate) binding_idx: AtomicU32,
}

#[derive(Clone, Debug, Shrinkwrap)]
pub struct TextureRef {
    pub inner: Arc<TextureRefInner>,
}

type TextureWeakRef = Weak<TextureRefInner>;

impl TextureRef {
    fn new(
        texture: wgpu::Texture,
        texture_view: Option<wgpu::TextureView>,
        binding_idx: u32,
    ) -> Self {
        Self {
            inner: Arc::new(TextureRefInner {
                texture_view: texture_view.unwrap_or_else(|| {
                    texture.create_view(&wgpu::TextureViewDescriptor::default())
                }),
                texture,
                binding_idx: AtomicU32::new(binding_idx),
            }),
        }
    }

    fn get_weak_ref(&self) -> TextureWeakRef {
        Arc::downgrade(&self.inner)
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.inner.texture
    }
}

impl PartialEq for TextureRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for TextureRef {}

#[derive(Debug)]
pub enum TextureManagerError {
    /// The texture manager has run out of texture slots.
    /// The maximum number of textures is provided in the error.
    OutOfTextureSlots(u32),
}

pub type TextureManagerResult<T> = Result<T, TextureManagerError>;

#[derive(Shrinkwrap, Debug, Clone)]
pub struct TextureManagerRef(Arc<RwLock<TextureManager>>);

impl TextureManagerRef {
    pub(crate) fn new(max_textures: u32, ctx: &RenderingContext) -> Self {
        Self::from(TextureManager::new(max_textures, ctx))
    }
}

impl From<TextureManager> for TextureManagerRef {
    #[inline(always)]
    fn from(value: TextureManager) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
}

#[derive(Debug)]
pub struct TextureManager {
    textures: Vec<TextureWeakRef>,
    sampler: wgpu::Sampler,

    max_textures: u32,

    _dummy_texture: wgpu::Texture,
    dummy_texture_view: wgpu::TextureView,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler_bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureManager {
    pub(crate) fn new(
        max_textures: u32,
        RenderingContext { device, .. }: &RenderingContext,
    ) -> Self {
        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture manager dummy texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                ..Default::default()
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let dummy_texture_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(""),
                entries: &(0..max_textures)
                    .map(|i| wgpu::BindGroupLayoutEntry {
                        binding: i,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::default(),
                        },
                        count: None,
                    })
                    .collect_vec(),
            });

        let sampler_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture atlas bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    visibility: wgpu::ShaderStages::FRAGMENT,
                }],
            });

        Self {
            textures: Default::default(),
            max_textures,

            _dummy_texture: dummy_texture,
            sampler,
            dummy_texture_view,

            texture_bind_group_layout,
            sampler_bind_group_layout,
        }
    }

    pub(crate) fn register_texture_with_view(
        &mut self,
        texture: wgpu::Texture,
        texture_view: impl Into<Option<wgpu::TextureView>>,
    ) -> TextureManagerResult<TextureRef> {
        let new_idx = self.textures.len() as u32;

        if new_idx >= self.max_textures {
            return Err(TextureManagerError::OutOfTextureSlots(self.max_textures));
        }

        let texture_ref = TextureRef::new(texture, texture_view.into(), new_idx);
        self.textures.push(texture_ref.get_weak_ref());

        Ok(texture_ref)
    }

    pub(crate) fn register_texture(
        &mut self,
        texture: wgpu::Texture,
    ) -> TextureManagerResult<TextureRef> {
        self.register_texture_with_view(texture, None)
    }

    pub(crate) fn generate_texture_bind_group(&mut self, device: &wgpu::Device) -> wgpu::BindGroup {
        let _changed = self
            .textures
            .drain_filter(|t| t.upgrade().is_none())
            .next()
            .is_some();

        let textures_to_allocate = self
            .textures
            .iter()
            .map(Weak::upgrade)
            .flatten()
            .collect_vec();

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture atlas bind group"),
            layout: &self.texture_bind_group_layout,
            entries: &(0..self.max_textures)
                .map(|i| wgpu::BindGroupEntry {
                    binding: i,

                    resource: wgpu::BindingResource::TextureView(
                        match textures_to_allocate.get(i as usize) {
                            Some(tex) => {
                                tex.binding_idx.store(i, Ordering::Relaxed);
                                &tex.texture_view
                            }
                            None => &self.dummy_texture_view,
                        },
                    ),
                })
                .collect_vec(),
        })
    }

    pub(crate) fn generate_sampler_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture atlas bind group"),
            layout: &self.sampler_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&self.sampler),
            }],
        })
    }

    pub fn get_max_textures(&self) -> u32 {
        self.max_textures
    }

    pub fn get_texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    pub fn get_sampler_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.sampler_bind_group_layout
    }
}
