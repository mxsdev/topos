use std::{os::raw::c_void, sync::Arc};

use itertools::Itertools;
use palette::Srgba;
use swash::scale;

use crate::{
    atlas,
    element::{Element, ElementEvent, TestElement},
    paint::ScenePainter,
    shape::{self, BoxShaderVertex, PaintRectangle},
    surface::{RenderSurface, RenderingContext},
    util::{LogicalToPhysical, Pos2, ToEuclid},
};

pub struct Scene {
    font_manager: atlas::FontManager,
    shape_renderer: shape::ShapeRenderer,

    last_mouse_pos: Option<Pos2>,

    elements: Vec<Box<dyn Element>>,
}

impl Scene {
    pub fn new(rendering_context: Arc<RenderingContext>) -> Self {
        let shape_renderer = shape::ShapeRenderer::new(&rendering_context);
        let font_manager = atlas::FontManager::new(rendering_context);

        let mut elements: Vec<Box<dyn Element>> = Default::default();

        elements.push(Box::new(TestElement::new()));

        Self {
            font_manager,
            shape_renderer,

            last_mouse_pos: None,

            elements,
        }
    }

    pub fn handle_window_event(&mut self, event: tao::event::Event<()>, sf: f64) {
        use tao::event::*;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CursorMoved { position, .. } => {
                    let logical_mouse_pos = position.to_logical(sf).to_euclid();

                    self.update(&ElementEvent::CursorMove {
                        pos: logical_mouse_pos,
                        del: self.last_mouse_pos.map(|p| logical_mouse_pos - p),
                    });

                    self.last_mouse_pos = Some(logical_mouse_pos);
                }

                WindowEvent::MouseInput {
                    state: tao::event::ElementState::Pressed,
                    button,
                    ..
                } => {
                    self.update(&ElementEvent::MouseDown { button });
                }

                WindowEvent::MouseInput {
                    state: tao::event::ElementState::Released,
                    button,
                    ..
                } => {
                    self.update(&ElementEvent::MouseUp { button });
                }

                _ => {}
            },
            _ => {}
        }
    }

    pub fn render(&mut self, render_surface: &RenderSurface, output: wgpu::SurfaceTexture) {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let RenderingContext { device, queue, .. } = render_surface.rendering_context();
        let scale_fac = render_surface.scale_factor();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let mut scene_painter = ScenePainter::default();
        self.paint(&mut scene_painter);

        let rects: Vec<_> = scene_painter
            .drain()
            .flat_map(|p| match p {
                shape::PaintShape::Rectangle(paint_rect) => {
                    BoxShaderVertex::from_paint_rect(paint_rect.to_physical(scale_fac))
                }
            })
            .collect();

        self.shape_renderer
            .prepare_boxes(device, queue, rects.into_iter());

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

            self.shape_renderer.render_all_boxes(&mut render_pass);
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Ok(())
    }
}

impl Element for Scene {
    fn update(&mut self, event: &ElementEvent) -> bool {
        for element in self.elements.iter_mut() {
            element.update(event);
        }

        false
    }

    fn paint(&mut self, painter: &mut ScenePainter) {
        for element in self.elements.iter_mut() {
            element.paint(painter);
        }
    }
}
