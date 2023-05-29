use std::sync::Arc;

// lib.rs
use tao::{event::WindowEvent, window::Window};
use wgpu::util::DeviceExt;

use crate::{
    atlas::{self},
    util::PhysicalPos2,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ParamsBuffer {
    screen_resolution: [u32; 2],
}

struct ScreenDescriptor {
    size: tao::dpi::PhysicalSize<u32>,
    scale_factor: f64,
}

pub struct State {
    surface: wgpu::Surface,
    font_manager: atlas::FontManager,

    rendering_context: Arc<RenderingContext>,

    screen_descriptor: ScreenDescriptor,

    config: wgpu::SurfaceConfiguration,
}

pub struct RenderingContext {
    pub params_buffer: wgpu::Buffer,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub texture_format: wgpu::TextureFormat,
}

impl State {
    // Creating some of the wgpu types requires async code
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
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
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

        let rendering_context: Arc<_> = RenderingContext {
            params_buffer: params_buffer,
            device: device,
            queue: queue,
            texture_format: surface_format,
        }
        .into();

        let font_manager = atlas::FontManager::new(rendering_context.clone());

        Self {
            rendering_context,
            surface,
            font_manager,
            config,
            screen_descriptor,
        }
    }

    pub fn resize(&mut self, new_size: tao::dpi::PhysicalSize<u32>, scale_factor: Option<f64>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.screen_descriptor.size = new_size;
            self.config_mut().width = new_size.width;
            self.config_mut().height = new_size.height;
            self.surface.configure(self.device(), self.config());

            if let Some(scale_factor) = scale_factor {
                self.screen_descriptor.scale_factor = scale_factor;
            }

            self.queue().write_buffer(
                self.params_buffer(),
                0,
                bytemuck::bytes_of(&Into::<[u32; 2]>::into(new_size)),
            )
        }
    }

    pub fn input(&mut self, _event: &WindowEvent) -> bool {
        todo!()
    }

    pub fn update(&mut self) {}

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let render_context = &self.rendering_context;
        let RenderingContext { device, queue, .. } = render_context.as_ref();

        //prepare
        // Text metrics indicate the font size and line height of a buffer
        let metrics = cosmic_text::Metrics::new(60.0, 80.0);

        let buffer = {
            let mut font_system = self.font_manager.get_font_system();

            // A Buffer provides shaping and layout for a UTF-8 string, create one per text widget
            let mut buffer = cosmic_text::Buffer::new(&mut font_system, metrics);

            // Borrow buffer together with the font system for more convenient method calls
            // let mut buffer = buffer.borrow_with(&mut font_system);

            // Set a size for the text buffer, in pixels
            buffer.set_size(&mut font_system, 700.0, 200.0);

            // Attributes indicate what font to choose
            let attrs = cosmic_text::Attrs::new();

            // attrs.family(cosmic_text::Family::Name(()))
            // attrs.family(Family)

            // Perform shaping as desired
            buffer.shape_until_scroll(&mut font_system);

            // Default text color (0xFF, 0xFF, 0xFF is white)
            // let text_color = Color::rgb(0xFF, 0xFF, 0xFF);

            // Add some text!
            buffer.set_text(&mut font_system, "Hello world! 🦀", attrs);

            buffer
        };

        let buffers = Arc::new(vec![buffer]);

        self.font_manager.generate_textures(buffers.clone());

        self.font_manager
            .prepare(buffers.iter(), PhysicalPos2::new(0., 0.));

        //render
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let font_manager_resources = self.font_manager.render_resources();

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.font_manager
                .render(&mut render_pass, &font_manager_resources);
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn get_render_context(&self) -> &RenderingContext {
        return &self.rendering_context;
    }

    pub fn get_size(&self) -> tao::dpi::PhysicalSize<u32> {
        self.screen_descriptor.size
    }

    fn config_mut(&mut self) -> &mut wgpu::SurfaceConfiguration {
        &mut self.config
    }

    fn device(&self) -> &wgpu::Device {
        &self.rendering_context.device
    }

    fn queue(&self) -> &wgpu::Queue {
        &self.rendering_context.queue
    }

    fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    fn params_buffer(&self) -> &wgpu::Buffer {
        &self.rendering_context.params_buffer
    }
}
