use core::num;
use std::sync::{Arc, Mutex, RwLock};

use euclid::default;
use swash::scale;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::{event::WindowEvent, window::Window};

use crate::{
    atlas::{self},
    element::boundary::Boundary,
    shape::{BoxShaderVertex, ShapeRenderer},
    time::FramerateCounter,
    util::{
        LogicalToPhysical, PhysicalPos2, PhysicalRect, PhysicalRoundedRect, Pos2, Rect,
        RoundedRect, ToEuclid,
    },
};

use crate::shape::{self};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ParamsBuffer {
    screen_resolution: [u32; 2],
}

struct ScreenDescriptor {
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum MultisampleMode {
    None,
    MSAA2x,
    #[default]
    MSAA4x,
    MSAA8x,
    MSAA16x,
}

impl MultisampleMode {
    pub const fn num_samples(&self) -> u32 {
        match self {
            MultisampleMode::None => 1,
            MultisampleMode::MSAA2x => 2,
            MultisampleMode::MSAA4x => 4,
            MultisampleMode::MSAA8x => 8,
            MultisampleMode::MSAA16x => 16,
        }
    }
}

pub struct RenderSurface {
    surface: wgpu::Surface,
    screen_descriptor: ScreenDescriptor,
    config: wgpu::SurfaceConfiguration,
    rendering_context: Arc<RenderingContext>,

    multisampled_framebuffer: Option<wgpu::Texture>,
    multisample_mode: MultisampleMode,
}

pub struct RenderingContext {
    pub params_buffer: wgpu::Buffer,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub texture_format: wgpu::TextureFormat,
    pub num_samples: RwLock<u32>,
}

pub trait SurfaceDependent {
    fn reconfigure(
        &mut self,
        context: &RenderingContext,
        size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    );
}

pub struct RenderAttachment {
    pub window_texture: wgpu::SurfaceTexture,
    pub msaa_view: Option<wgpu::TextureView>,
    pub num_samples: u32,
}

impl RenderSurface {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let screen_descriptor = ScreenDescriptor {
            scale_factor: window.scale_factor(),
            size,
        };

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        log::trace!("Allowed present modes: {:?}", surface_caps.present_modes);

        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let texture_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            // present_mode: surface_caps
            //     .present_modes
            //     .into_iter()
            //     .find(|m| *m == wgpu::PresentMode::Immediate)
            //     .unwrap_or(wgpu::PresentMode::Fifo),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let params = ParamsBuffer {
            screen_resolution: size.into(),
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let multisample_mode = MultisampleMode::default();

        let rendering_context = RenderingContext {
            device,
            params_buffer,
            queue,
            texture_format,
            num_samples: RwLock::new(multisample_mode.num_samples()),
        }
        .into();

        let mut render_surface = Self {
            config,
            rendering_context,
            screen_descriptor,
            surface,

            multisample_mode,
            multisampled_framebuffer: None,
        };

        render_surface.configure_multisampled_framebuffer();

        render_surface
    }

    fn configure_multisampled_framebuffer(&mut self) {
        let num_samples = self.multisample_mode.num_samples();

        self.multisampled_framebuffer = (num_samples > 1).then(|| {
            let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: num_samples,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                label: None,
                view_formats: &[],
            };

            self.rendering_context
                .device
                .create_texture(&multisampled_frame_descriptor)
        });
    }

    fn reconfigure_dependents<'a>(
        &self,
        dependents: impl Iterator<Item = &'a mut dyn SurfaceDependent>,
    ) {
        for child in dependents {
            child.reconfigure(
                &self.rendering_context,
                self.screen_descriptor.size,
                self.scale_factor(),
            );
        }
    }

    pub fn reconfigure<'a>(
        &mut self,
        dependents: impl Iterator<Item = &'a mut dyn SurfaceDependent>,
    ) {
        self.resize(self.get_size(), None, dependents)
    }

    pub fn resize<'a>(
        &mut self,
        new_size: PhysicalSize<u32>,
        scale_factor: Option<f64>,
        dependents: impl Iterator<Item = &'a mut dyn SurfaceDependent>,
    ) {
        if new_size.width > 0 && new_size.height > 0 {
            self.screen_descriptor.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface
                .configure(&self.rendering_context.device, &self.config);

            self.configure_multisampled_framebuffer();

            if let Some(scale_factor) = scale_factor {
                self.screen_descriptor.scale_factor = scale_factor;
            }

            self.rendering_context.queue.write_buffer(
                &self.rendering_context.params_buffer,
                0,
                bytemuck::bytes_of(&Into::<[u32; 2]>::into(new_size)),
            );

            self.reconfigure_dependents(dependents)
        }
    }

    pub fn get_output(&self) -> Result<RenderAttachment, wgpu::SurfaceError> {
        let window_texture = self.surface.get_current_texture()?;
        let msaa_view = self
            .multisampled_framebuffer
            .as_ref()
            .map(|tex| tex.create_view(&wgpu::TextureViewDescriptor::default()));

        Ok(RenderAttachment {
            window_texture,
            msaa_view,
            num_samples: self.multisample_mode.num_samples(),
        })
    }

    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }

    pub fn clone_rendering_context(&self) -> Arc<RenderingContext> {
        self.rendering_context.clone()
    }

    pub fn rendering_context(&self) -> &RenderingContext {
        &self.rendering_context
    }

    pub fn scale_factor(&self) -> f64 {
        self.screen_descriptor.scale_factor
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.screen_descriptor.size
    }
}
