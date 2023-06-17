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
    input::{input_state::InputState, output::PlatformOutput, winit::WinitState},
    mesh::{self, PaintMesh},
    scene::update::UpdatePass,
    shape::{self, BoxShaderVertex, PaintRectangle, PaintShape},
    surface::{RenderAttachment, RenderSurface, RenderingContext, SurfaceDependent},
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
    mesh_renderer: mesh::MeshRenderer,

    root: ElementRef<Root>,
}

impl<Root: RootConstructor + 'static> Scene<Root> {
    pub fn new(rendering_context: Arc<RenderingContext>, scale_fac: f64) -> Self {
        let shape_renderer = shape::ShapeRenderer::new(&rendering_context);
        let mesh_renderer = mesh::MeshRenderer::new(&rendering_context);
        let mut font_manager = atlas::FontManager::new(rendering_context);

        {
            let mut font_system = font_manager.get_font_system();
            font_system.db_mut().load_system_fonts();
        }

        let scene_resources = SceneResources {
            font_system: font_manager.get_font_system_ref(),
            scale_factor: scale_fac as f32,
        };

        let root = Root::new(&scene_resources).into();

        Self {
            font_manager,
            shape_renderer,
            mesh_renderer,
            root,
        }
    }

    pub fn render(
        &mut self,
        render_surface: &RenderSurface,
        RenderAttachment {
            window_texture,
            msaa_view,
            ..
        }: RenderAttachment,
        mut input: InputState,
    ) -> (InputState, PlatformOutput) {
        let window_view = window_texture
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
        let SceneContext {
            shapes,
            output: platform_output,
            ..
        } = scene_context;

        let mut batcher = BatchedRenderCollector::new();

        let mut rects = Vec::new();
        let mut text_boxes = Vec::new();

        let mut meshes = Vec::new();
        let mut num_mesh_vertices = 0;
        let mut num_mesh_indices = 0;

        let mut last_clip_rect = None;

        for shape in shapes.into_iter() {
            match shape {
                shape::PaintShape::Rectangle(paint_rect) => {
                    let physical_paint_rect = paint_rect.to_physical(scale_fac);

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
                shape::PaintShape::Mesh(paint_mesh) => {
                    let num_indices = paint_mesh.indices.len();
                    let num_vertices = paint_mesh.vertices.len();

                    batcher.add_mesh_indices(num_indices as u64);

                    num_mesh_indices += num_indices;
                    num_mesh_vertices += num_vertices;

                    meshes.push(paint_mesh);
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

        self.mesh_renderer.prepare_meshes(
            device,
            queue,
            meshes.into_iter().map(|m| m.as_gpu_mesh(scale_fac)),
            num_mesh_vertices as u64,
            num_mesh_indices as u64,
        );

        let mut text_box_iterator = self.font_manager.prepare(text_boxes);

        let batches = batcher.finalize();

        let font_resources = self.font_manager.render_resources();

        {
            let load_op = wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(match &msaa_view {
                    None => wgpu::RenderPassColorAttachment {
                        view: &window_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: true,
                        },
                    },
                    Some(msaa_view) => wgpu::RenderPassColorAttachment {
                        view: msaa_view,
                        resolve_target: Some(&window_view),
                        ops: wgpu::Operations {
                            load: load_op,
                            store: false,
                        },
                    },
                })],
                depth_stencil_attachment: None,
            });

            for x in batches {
                // log::trace!("{:?}", x);

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

                    BatchedRender::MeshIndices(num_indices) => self
                        .mesh_renderer
                        .render_indices(&mut render_pass, num_indices),

                    BatchedRender::ClipRect(Some(rect)) => render_pass.set_scissor_rect(
                        rect.min.x,
                        rect.min.y,
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
        window_texture.present();

        (input, platform_output)
    }

    fn generate_scene_resources(&self, scale_factor: f32) -> SceneResources {
        SceneResources {
            font_system: self.font_manager.get_font_system_ref(),
            scale_factor,
        }
    }

    pub fn get_dependents_mut<'a>(&mut self) -> impl Iterator<Item = &mut dyn SurfaceDependent> {
        [
            &mut self.font_manager as &mut dyn SurfaceDependent,
            &mut self.shape_renderer,
            &mut self.mesh_renderer,
        ]
        .into_iter()
    }
}

#[derive(EnumAsInner, Debug)]
enum BatchedRender {
    Rectangles(u64),
    TextBox,
    ClipRect(Option<PhysicalRect<u32>>),
    MeshIndices(u64),
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

    fn add_mesh_indices(&mut self, indices: u64) {
        let el = self
            .current
            .get_or_insert(BatchedRender::MeshIndices(Default::default()));

        if let Some(num_indices) = el.as_mesh_indices_mut() {
            *num_indices += indices;
        } else {
            self.write_current();
            self.current = Some(BatchedRender::MeshIndices(indices));
        };
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
}

struct BatchRenderer<
    T: Iterator<Item = BatchedRender>,
    K: Iterator<Item = BatchedAtlasRenderBoxesEntry>,
> {
    inner: T,
    text_box_iterator: BatchedAtlasRenderBoxIterator<K>,
}
