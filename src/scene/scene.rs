use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
    sync::Arc,
};

use rustc_hash::FxHashMap;
use swash::scale;

use crate::{
    atlas,
    element::{Element, ElementEvent, SizeConstraint, TestElement},
    input::{input_state::InputState, winit::WinitState},
    scene::update::UpdatePass,
    shape::{self, BoxShaderVertex, PaintRectangle},
    surface::{RenderSurface, RenderingContext},
    util::{LogicalToPhysical, PhysicalToLogical, Pos2, Size2, ToEuclid},
};

use super::{
    ctx::{SceneContext, SceneContextInternal},
    layout::{ElementPlacement, LayoutPass},
    PaintPass,
};

pub struct Scene<Root: Element> {
    font_manager: atlas::FontManager,
    shape_renderer: shape::ShapeRenderer,

    last_mouse_pos: Option<Pos2>,

    // elements: Vec<Box<dyn Element>>,
    root: Root,

    scene_context: Rc<RefCell<SceneContextInternal>>,
}

impl<Root: Element> Scene<Root> {
    pub fn new(rendering_context: Arc<RenderingContext>, root: Root) -> Self {
        let shape_renderer = shape::ShapeRenderer::new(&rendering_context);
        let font_manager = atlas::FontManager::new(rendering_context);

        // let mut elements: Vec<Box<dyn Element>> = Default::default();
        // elements.push(Box::new(TestElement::new()));

        Self {
            font_manager,
            shape_renderer,

            last_mouse_pos: None,

            root,

            scene_context: Rc::new(RefCell::new(SceneContextInternal::default())),
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
    ) {
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
        let mut layout_pass = LayoutPass::create(&self.root);
        layout_pass.layout_child(&mut self.root, default_constraints);

        self.scene_context
            .borrow_mut()
            .set_placement(layout_pass.finish());

        // render pass
        let mut scene_context = SceneContext::new(self.scene_context.clone());

        // for element in self.iter_elements() {
        //     element.ui(
        //         &mut scene_context,
        //         SizeConstraint {
        //             max: Size2::zero(),
        //             min: Size2::zero(),
        //         },
        //     );
        // }

        let rects: Vec<_> = scene_context
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

// #[derive(Hash)]
// struct ElementId(usize);

// impl ElementId {
//     pub fn from_element(element: &Box<dyn Element>) -> Self {
//         let (ptr, ..) = (&**element as *const dyn Element).to_raw_parts();
//         Self(ptr as usize)
//     }
// }

// struct ElementTree {
//     elements: FxHashMap<ElementId, Box<dyn Element>>,
//     children: FxHashMap<ElementId, Vec<ElementId>>,
// }
