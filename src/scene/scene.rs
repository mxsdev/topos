use std::{
    cell::{RefCell, RefMut},
    ops::DerefMut,
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard},
};

use cosmic_text::{fontdb::Query, Attrs, Family, FontSystem};
use enum_as_inner::EnumAsInner;
use rustc_hash::FxHashMap;
use swash::scale;

use crate::{
    atlas::{
        self, BatchedAtlasRender, BatchedAtlasRenderBoxIterator, BatchedAtlasRenderBoxesEntry,
        FontManagerRenderResources,
    },
    element::{Element, ElementEvent, ElementRef, RootConstructor, SizeConstraint},
    input::{input_state::InputState, winit::WinitState},
    scene::update::UpdatePass,
    shape::{self, BoxShaderVertex, PaintRectangle, PaintShape},
    surface::{RenderSurface, RenderingContext},
    util::{
        LogicalToPhysical, LogicalToPhysicalInto, PhysicalRect, PhysicalToLogical, Pos2,
        RoundToInt, Size2, ToEuclid,
    },
};

use super::{ctx::SceneContext, layout::LayoutPass, PaintPass};

#[derive(Clone)]
pub struct SceneResources {
    font_system: Arc<Mutex<FontSystem>>,
    scale_factor: f32,
}

impl SceneResources {
    pub fn font_system(&self) -> impl DerefMut<Target = FontSystem> + '_ {
        self.font_system.lock().unwrap()
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }
}

pub struct Scene<Root: RootConstructor + 'static> {
    font_manager: atlas::FontManager,
    shape_renderer: shape::ShapeRenderer,

    last_mouse_pos: Option<Pos2>,

    // elements: Vec<Box<dyn Element>>,
    root: ElementRef<Root>,
}

impl<Root: RootConstructor + 'static> Scene<Root> {
    pub fn new(rendering_context: Arc<RenderingContext>, scale_fac: f64) -> Self {
        let shape_renderer = shape::ShapeRenderer::new(&rendering_context);
        let mut font_manager = atlas::FontManager::new(rendering_context);

        {
            let mut font_system = font_manager.get_font_system();
            font_system.db_mut().load_system_fonts();
        }

        // let mut elements: Vec<Box<dyn Element>> = Default::default();
        // elements.push(Box::new(TestElement::new()));

        let scene_resources = SceneResources {
            font_system: font_manager.get_font_system_ref(),
            scale_factor: scale_fac as f32,
        };

        let root = Root::new(&scene_resources).into();

        Self {
            font_manager,
            shape_renderer,

            last_mouse_pos: None,
            root,
        }
    }

    // fn iter_elements(&mut self) -> impl Iterator<Item = &mut Box<dyn Element>> {
    //     self.elements.iter_mut()
    // }

    // pub fn handle_window_event(&mut self, event: winit::event::Event<()>, sf: f64) {
    //     use winit::event::*;
    // }

    pub fn render(
        &mut self,
        render_surface: &RenderSurface,
        output: wgpu::SurfaceTexture,
        mut input: InputState,
    ) -> InputState {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let RenderingContext { device, queue, .. } = render_surface.rendering_context();
        let scale_fac = render_surface.scale_factor();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let physical_screen_size = render_surface.get_size().to_euclid();

        let screen_size = physical_screen_size.to_f32().to_logical(scale_fac);

        // layout pass
        let scene_resources = self.generate_scene_resources(scale_fac as f32);

        let layout_pass = LayoutPass::new(&mut self.root, scene_resources);

        let mut scene_layout = layout_pass.do_layout_pass(screen_size, &mut self.root);

        scene_layout.do_input_pass(&mut input);

        let mut scene_context = SceneContext::new(scale_fac as f32);
        scene_layout.do_ui_pass(&mut scene_context);

        // render pass
        let shapes = scene_context.drain();

        let mut batcher = BatchedRenderCollector::new();

        let mut rects = Vec::new();
        let mut text_boxes = Vec::new();

        let mut last_clip_rect = None;

        for shape in shapes.into_iter() {
            match shape {
                shape::PaintShape::Rectangle(paint_rect) => {
                    let physical_paint_rect = paint_rect.to_physical(scale_fac);

                    // physical_paint_rect.rect.rect = physical_paint_rect.rect.rect.round();

                    if let Some(clip_rect) = last_clip_rect {
                        if physical_paint_rect
                            .get_bounding_box()
                            .intersection(&clip_rect)
                            .is_none()
                        {
                            continue;
                        }
                    }

                    let (draw_rects, num_rects) =
                        BoxShaderVertex::from_paint_rect(physical_paint_rect);

                    batcher.add_rects(num_rects);

                    rects.extend(draw_rects);
                }
                shape::PaintShape::Text(text_box) => {
                    batcher.add_text_box();
                    text_boxes.push(
                        text_box
                            .to_physical(scale_fac)
                            .with_clip_rect(last_clip_rect),
                    );
                }
                shape::PaintShape::ClipRect(rect) => {
                    let physical_rect = rect.map(|r| r.to_physical(scale_fac));
                    last_clip_rect = physical_rect;

                    batcher.set_clip_rect(physical_rect.map(|r| r.round_to_int()))
                }
            }
        }

        self.shape_renderer
            .prepare_boxes(device, queue, rects.into_iter());

        let mut text_box_iterator = self.font_manager.prepare(text_boxes);

        let batches = batcher.finalize();

        let font_resources = self.font_manager.render_resources();

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

            for x in batches {
                match x {
                    BatchedRender::Rectangles(num_boxes) => {
                        self.shape_renderer
                            .render_boxes(&mut render_pass, num_boxes);
                    }

                    BatchedRender::TextBox => {
                        for text_box_batch in &mut text_box_iterator {
                            self.font_manager.render(
                                &mut render_pass,
                                &font_resources,
                                &text_box_batch,
                            )
                        }
                    }

                    BatchedRender::ClipRect(Some(rect)) => render_pass.set_scissor_rect(
                        rect.min.x,
                        rect.min.x,
                        rect.width(),
                        rect.height(),
                    ),

                    BatchedRender::ClipRect(None) => render_pass.set_scissor_rect(
                        0,
                        0,
                        physical_screen_size.width,
                        physical_screen_size.height,
                    ),
                }
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        input

        // Ok(())
    }

    fn generate_scene_resources(&self, scale_factor: f32) -> SceneResources {
        SceneResources {
            font_system: self.font_manager.get_font_system_ref(),
            scale_factor,
        }
    }
}

#[derive(EnumAsInner, Debug)]
enum BatchedRender {
    Rectangles(u64),
    TextBox,
    ClipRect(Option<PhysicalRect<u32>>),
}

#[derive(Default)]
struct BatchedRenderCollector {
    batches: Vec<BatchedRender>,
    current: Option<BatchedRender>,
}

impl BatchedRenderCollector {
    fn new() -> Self {
        Default::default()
    }

    fn add_rects(&mut self, quantity: u64) {
        let el = self
            .current
            .get_or_insert(BatchedRender::Rectangles(Default::default()));

        if let Some(num_rects) = el.as_rectangles_mut() {
            *num_rects += quantity;
        } else {
            self.write_current();
            self.current = Some(BatchedRender::Rectangles(quantity));
        };
    }

    fn add_text_box(&mut self) {
        self.write_current();
        self.batches.push(BatchedRender::TextBox);
    }

    fn set_clip_rect(&mut self, rect: Option<PhysicalRect<u32>>) {
        if let Some(BatchedRender::ClipRect(current_rect)) = self.current {
            if current_rect == rect {
                return;
            }
        }

        self.write_current();
        self.current = Some(BatchedRender::ClipRect(rect));
    }

    fn write_current(&mut self) {
        self.batches.extend(self.current.take().into_iter());
    }

    fn finalize(mut self) -> Vec<BatchedRender> {
        self.write_current();
        self.batches
    }

    // fn try_incr(shape: &PaintShape, el: &mut BatchedRender) -> Option<()> {
    //     match shape {
    //         PaintShape::Rectangle(_) => {
    //             *el.as_rectangles_mut()? += 1;
    //         }
    //         PaintShape::Text(_) => {
    //             return None;
    //         }
    //     };

    //     Some(())
    // }
}

// struct BatchedRenderIterator<T: Iterator<Item = BatchedRender>> {
//     inner: T,
// }

struct BatchRenderer<
    T: Iterator<Item = BatchedRender>,
    K: Iterator<Item = BatchedAtlasRenderBoxesEntry>,
> {
    inner: T,
    text_box_iterator: BatchedAtlasRenderBoxIterator<K>,
}

// impl<T: Iterator<Item = BatchedRender>, K: Iterator<Item = BatchedAtlasRenderBoxesEntry>>
//     BatchRenderer<T, K>
// {
//     pub fn next<'a: 'b, 'b>(
//         &mut self,
//         render_pass: &'a mut wgpu::RenderPass<'b>,
//         resources: &'b mut FontManagerRenderResources<'b>,
//         scene: &'a mut Scene<impl Element>,
//     ) -> Option<()> {
//         match self.inner.next()? {
//             BatchedRender::Rectangles(num_boxes) => {
//                 scene.shape_renderer.render_boxes(render_pass, num_boxes);
//             }
//             BatchedRender::TextBox => {
//                 for text_box_batch in &mut self.text_box_iterator {
//                     scene
//                         .font_manager
//                         .render(render_pass, resources, &text_box_batch)
//                 }
//             }
//         };

//         Some(())
//     }
// }

// fn prepare_batch_renderer(
//     scene: &mut Scene<impl Element>,
//     device: &wgpu::Device,
//     queue: &wgpu::Queue,
//     shapes: Vec<PaintShape>,
//     scale_fac: f64,
// ) -> BatchRenderer<
//     impl Iterator<Item = BatchedRender>,
//     impl Iterator<Item = BatchedAtlasRenderBoxesEntry>,
// > {
//     // let mut batches = Vec::new();
//     let mut batcher = BatchedRenderCollector::new();

//     let mut rects = Vec::new();
//     let mut boxes = Vec::new();

//     for shape in shapes.into_iter().rev() {
//         match shape {
//             shape::PaintShape::Rectangle(paint_rect) => rects.extend(
//                 BoxShaderVertex::from_paint_rect(paint_rect.to_physical(scale_fac)),
//             ),
//             shape::PaintShape::Text(text_box) => boxes.push(text_box),
//         }
//     }

//     let num_rects = rects.len();

//     scene
//         .shape_renderer
//         .prepare_boxes(device, queue, rects.into_iter());

//     let text_box_iterator = scene.font_manager.prepare(boxes);

//     BatchRenderer {
//         inner: batcher.finalize().into_iter(),
//         text_box_iterator: text_box_iterator,
//     }
// }
