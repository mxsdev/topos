use std::{
    cell::{RefCell, RefMut},
    ops::DerefMut,
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard},
};

use enum_as_inner::EnumAsInner;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use swash::scale;

use crate::{
    accessibility::AccessNode,
    atlas::{self},
    element::{Element, ElementEvent, ElementId, ElementRef, RootConstructor, SizeConstraint},
    graphics::{DynamicGPUMeshTriBuffer, PushVertices, VertexBuffers},
    input::{input_state::InputState, output::PlatformOutput, winit::WinitState},
    math::{PhysicalRect, PhysicalSize, Rect, WindowScaleFactor},
    scene::update::UpdatePass,
    shape::{self, BoxShaderVertex, PaintMeshVertex, PaintRectangle, PaintShape},
    surface::{RenderAttachment, RenderSurface, RenderingContext, SurfaceDependent},
    util::{
        text::{FontSystem, PlacedTextBox},
        PhysicalUnit,
    },
};

use super::{
    ctx::SceneContext,
    framepacer::Framepacer,
    layout::{LayoutEngine, LayoutPass},
    PaintPass,
};

pub struct SceneResources<'a> {
    font_system: Arc<Mutex<FontSystem>>,
    rendering_context: Arc<RenderingContext>,
    layout_engine: &'a mut LayoutEngine,
    scale_factor: WindowScaleFactor,
}

impl<'a> SceneResources<'a> {
    pub fn new(
        font_system: Arc<Mutex<FontSystem>>,
        rendering_context: Arc<RenderingContext>,
        scale_factor: WindowScaleFactor,
        layout_engine: &'a mut LayoutEngine,
    ) -> Self {
        Self {
            font_system,
            rendering_context,
            scale_factor,
            layout_engine,
        }
    }

    pub(super) fn set_scale_factor(&mut self, fac: WindowScaleFactor) {
        self.scale_factor = fac;
    }

    pub fn font_system(&self) -> impl DerefMut<Target = FontSystem> + '_ {
        self.font_system.lock().unwrap()
    }

    pub fn font_system_ref(&self) -> Arc<Mutex<FontSystem>> {
        self.font_system.clone()
    }

    pub fn rendering_context_ref(&self) -> Arc<RenderingContext> {
        self.rendering_context.clone()
    }

    pub fn scale_factor(&self) -> WindowScaleFactor {
        self.scale_factor
    }

    pub fn layout_engine(&mut self) -> &mut LayoutEngine {
        self.layout_engine
    }
}

pub struct Scene<Root: RootConstructor + 'static> {
    font_manager: atlas::FontManager,
    shape_renderer: shape::ShapeRenderer,

    root: ElementRef<Root>,

    layout_engine: LayoutEngine,

    shape_buffer: DynamicGPUMeshTriBuffer<BoxShaderVertex>,
}

impl<Root: RootConstructor + 'static> Scene<Root> {
    pub fn new(rendering_context: Arc<RenderingContext>, scale_fac: f64) -> Self {
        let shape_renderer = shape::ShapeRenderer::new(&rendering_context);
        let mut font_manager = atlas::FontManager::new(rendering_context.clone());

        {
            let mut font_system = font_manager.get_font_system();

            font_system.db_mut().load_system_fonts();

            // font_system
            //     .db_mut()
            //     .load_font_data(include_bytes!("../../assets/TestCalibre-Regular.otf").to_vec());
        }

        let mut layout_engine = LayoutEngine::default();
        layout_engine.disable_rounding();

        let mut scene_resources = SceneResources::new(
            font_manager.get_font_system_ref(),
            rendering_context,
            WindowScaleFactor::new(scale_fac as f32),
            &mut layout_engine,
        );

        let root = Root::new(&mut scene_resources).into();

        let shape_buffer = DynamicGPUMeshTriBuffer::new(&scene_resources.rendering_context.device);

        Self {
            font_manager,
            shape_renderer,
            root,
            layout_engine,
            shape_buffer,
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

        let render_ctx = render_surface.rendering_context();

        let RenderingContext { device, queue, .. } = render_ctx;

        let scale_fac = render_surface.scale_factor();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let physical_screen_size: PhysicalSize<u32> = render_surface.get_size().into();

        let screen_size = physical_screen_size.map(|x| x as f32) * scale_fac.inverse();

        // layout pass
        let mut scene_resources = SceneResources::new(
            self.font_manager.get_font_system_ref(),
            render_surface.clone_rendering_context(),
            scale_fac,
            &mut self.layout_engine,
        );

        let layout_pass = LayoutPass::new(&mut self.root, &mut scene_resources);

        let mut scene_layout = layout_pass.do_layout_pass(screen_size, &mut self.root);

        scene_layout.do_input_pass(&mut input);

        let mut scene_context = SceneContext::new(scale_fac);
        scene_layout.do_ui_pass(&mut scene_context);

        scene_context.output.accesskit_update().tree =
            Some(accesskit::Tree::new(self.root.id().as_access_id()));

        // render pass
        let SceneContext {
            shapes,
            output: platform_output,
            ..
        } = scene_context;

        let mut shape_buffer_local = VertexBuffers::default();

        for shape in shapes {
            match shape {
                PaintShape::Rectangle(paint_rect) => {
                    shape_buffer_local
                        .push_quads(BoxShaderVertex::from_paint_rect(paint_rect * scale_fac).0);
                }

                PaintShape::Text(text_box) => {
                    self.font_manager
                        .prepare(text_box.apply_scale_fac(scale_fac), &mut shape_buffer_local);
                }

                PaintShape::Mesh(mesh) => shape_buffer_local.push_vertices(
                    mesh.vertices
                        .into_iter()
                        .map(|PaintMeshVertex { color, pos }| {
                            BoxShaderVertex::mesh_tri(pos * scale_fac, color)
                        }),
                    mesh.indices,
                ),

                PaintShape::ClipRect(_) => {}
            }
        }

        self.shape_buffer
            .write_all(queue, device, &shape_buffer_local);

        {
            let load_op = wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
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

            render_pass.set_pipeline(&render_ctx.shape_render_pipeline);

            render_pass.set_bind_group(0, &render_ctx.shape_bind_group, &[]);

            self.shape_buffer.render(&mut render_pass, 0..1);
        }

        // TODO: for multiple render passes, submit multiple encoders as
        // iterator (??? might work, test performance)
        queue.submit(std::iter::once(encoder.finish()));
        window_texture.present();

        (input, platform_output)
    }

    pub fn get_dependents_mut<'a>(&mut self) -> impl Iterator<Item = &mut dyn SurfaceDependent> {
        [
            &mut self.font_manager as &mut dyn SurfaceDependent,
            &mut self.shape_renderer,
        ]
        .into_iter()
    }

    pub fn root_id(&self) -> ElementId {
        self.root.id()
    }

    pub fn root_access_node(&mut self) -> AccessNode {
        self.root.get().node().build()
    }
}
