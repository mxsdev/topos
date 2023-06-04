use std::sync::{Arc, Mutex, RwLock};

use palette::Srgba;
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

pub struct RenderSurface {
    surface: wgpu::Surface,
    screen_descriptor: ScreenDescriptor,
    config: wgpu::SurfaceConfiguration,
    rendering_context: Arc<RenderingContext>,
}

pub struct RenderingContext {
    pub params_buffer: wgpu::Buffer,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub texture_format: wgpu::TextureFormat,
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

        let rendering_context = RenderingContext {
            device,
            params_buffer,
            queue,
            texture_format,
        }
        .into();

        Self {
            config,
            rendering_context,
            screen_descriptor,
            surface,
        }
    }

    pub fn reconfigure(&mut self) {
        self.resize(self.get_size(), None)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>, scale_factor: Option<f64>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.screen_descriptor.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface
                .configure(&self.rendering_context.device, &self.config);

            if let Some(scale_factor) = scale_factor {
                self.screen_descriptor.scale_factor = scale_factor;
            }

            self.rendering_context.queue.write_buffer(
                &self.rendering_context.params_buffer,
                0,
                bytemuck::bytes_of(&Into::<[u32; 2]>::into(new_size)),
            )
        }
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
