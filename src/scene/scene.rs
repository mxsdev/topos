use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
    sync::Arc,
};

use enum_as_inner::EnumAsInner;
use rustc_hash::FxHashMap;
use swash::scale;

use crate::{
    atlas::{
        self, BatchedAtlasRender, BatchedAtlasRenderBoxIterator, BatchedAtlasRenderBoxesEntry,
        FontManagerRenderResources,
    },
    element::{Element, ElementEvent, ElementRef, SizeConstraint},
    input::{input_state::InputState, winit::WinitState},
    scene::update::UpdatePass,
    shape::{self, BoxShaderVertex, PaintRectangle, PaintShape},
    surface::{RenderSurface, RenderingContext},
    util::{LogicalToPhysical, PhysicalToLogical, Pos2, Size2, ToEuclid},
};

use super::{
    ctx::SceneContext,
    layout::{ElementPlacement, LayoutPass},
    PaintPass,
};

pub struct Scene<Root: Element + 'static> {
    font_manager: atlas::FontManager,
    shape_renderer: shape::ShapeRenderer,

    last_mouse_pos: Option<Pos2>,

    // elements: Vec<Box<dyn Element>>,
    root: ElementRef<Root>,
}

impl<Root: Element + 'static> Scene<Root> {
    pub fn new(rendering_context: Arc<RenderingContext>, root: Root) -> Self {
        let shape_renderer = shape::ShapeRenderer::new(&rendering_context);
        let font_manager = atlas::FontManager::new(rendering_context);

        // let mut elements: Vec<Box<dyn Element>> = Default::default();
        // elements.push(Box::new(TestElement::new()));

        Self {
            font_manager,
            shape_renderer,

            last_mouse_pos: None,

            root: root.into(),
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
        input: InputState,
    ) -> InputState {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let RenderingContext { device, queue, .. } = render_surface.rendering_context();
        let scale_fac = render_surface.scale_factor();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let size = render_surface
            .get_size()
            .to_euclid()
            .to_f32()
            .to_logical(scale_fac);

        let default_constraints = SizeConstraint {
            min: Size2::zero(),
            max: size,
        };

        // layout pass
        let mut layout_pass = LayoutPass::create(&mut self.root);
        self.root.layout(default_constraints, &mut layout_pass);

        let scene_layout = layout_pass.finish();

        // render pass
        let mut scene_context = SceneContext::new(input, scene_layout, scale_fac as f32);

        self.root.ui(&mut scene_context, Pos2::zero());

        let (shapes, input) = scene_context.drain();

        let mut batcher = BatchedRenderCollector::new();

        let mut rects = Vec::new();
        let mut text_boxes = Vec::new();

        for shape in shapes.into_iter().rev() {
            match shape {
                shape::PaintShape::Rectangle(paint_rect) => {
                    let (draw_rects, num_rects) =
                        BoxShaderVertex::from_paint_rect(paint_rect.to_physical(scale_fac));

                    batcher.add_rects(num_rects);

                    rects.extend(draw_rects)
                }
                shape::PaintShape::Text(text_box) => {
                    batcher.add_text_box();
                    text_boxes.push(text_box)
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

                    _ => {}
                }
            }

            // self.shape_renderer
            //     .render_boxes(&mut render_pass, num_rects as u64);
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        input

        // Ok(())
    }
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
            self.batches.extend(self.current.take().into_iter());
            self.current = Some(BatchedRender::Rectangles(quantity));
        };
    }

    fn add_text_box(&mut self) {
        self.batches.extend(self.current.take().into_iter());
        self.batches.push(BatchedRender::TextBox);
    }

    fn finalize(mut self) -> Vec<BatchedRender> {
        self.batches.extend(self.current.into_iter());
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

#[derive(EnumAsInner, Debug)]
enum BatchedRender {
    Rectangles(u64),
    TextBox,
}

impl BatchedRender {
    pub fn default_for_shape(shape: &PaintShape) -> Self {
        match shape {
            PaintShape::Rectangle(_) => Self::Rectangles(Default::default()),
            PaintShape::Text(_) => Self::TextBox,
        }
    }
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
