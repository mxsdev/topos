use std::{
    ops::DerefMut,
    sync::{Arc, RwLock},
};

use bytemuck::Zeroable;
use itertools::Itertools;

use crate::{
    accessibility::AccessNode,
    atlas::{self, TextureAtlasManager, TextureAtlasManagerRef},
    element::{Element, ElementId, ElementRef, RootConstructor},
    graphics::{DynamicGPUMeshTriBuffer, PushVertices, VertexBuffers},
    input::{input_state::InputState, output::PlatformOutput},
    math::{PhysicalSize, Pos, Rect, WindowScaleFactor},
    shape::{
        self, BoxShaderVertex, ComputedPaintShape, PaintMeshVertex, PaintShape, ShaderClipRect,
        ShapeBufferWithContext,
    },
    surface::{RenderAttachment, RenderSurface, RenderingContext},
    texture::TextureManagerRef,
    util::text::{FontSystem, FontSystemRef},
};

use super::{
    ctx::{PaintShapeWithContext, SceneContext},
    layout::{ElementTree, LayoutEngine, LayoutPass},
};

pub struct SceneResources<'a> {
    texture_atlas_manager: atlas::TextureAtlasManagerRef,
    texture_manager: TextureManagerRef,
    font_system: FontSystemRef,
    rendering_context: Arc<RenderingContext>,
    layout_engine: &'a mut LayoutEngine,
    scale_factor: WindowScaleFactor,
}

impl<'a> SceneResources<'a> {
    pub fn new(
        texture_atlas_manager: atlas::TextureAtlasManagerRef,
        texture_manager: TextureManagerRef,
        font_system: FontSystemRef,
        rendering_context: Arc<RenderingContext>,
        scale_factor: WindowScaleFactor,
        layout_engine: &'a mut LayoutEngine,
    ) -> Self {
        Self {
            texture_atlas_manager,
            texture_manager,
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

    pub fn font_system_ref(&self) -> FontSystemRef {
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

    pub fn texture_atlas_manager(&self) -> &TextureAtlasManagerRef {
        &self.texture_atlas_manager
    }

    pub fn texture_manager(&self) -> &TextureManagerRef {
        &self.texture_manager
    }
}

pub struct Scene<Root: RootConstructor + 'static> {
    font_manager: atlas::FontManager,
    shape_renderer: shape::ShapeRenderer,
    atlas_manager: atlas::TextureAtlasManagerRef,
    texture_manager: TextureManagerRef,

    root: ElementRef<Root>,

    layout_engine: LayoutEngine,
}

impl<Root: RootConstructor + 'static> Scene<Root> {
    pub fn new(
        rendering_context: Arc<RenderingContext>,
        texture_manager: &TextureManagerRef,
        scale_fac: f64,
    ) -> Self {
        let shape_renderer = shape::ShapeRenderer::new(&rendering_context, texture_manager);
        let mut font_manager =
            atlas::FontManager::new(rendering_context.clone(), texture_manager.clone());

        {
            let mut font_system = font_manager.get_font_system().lock().unwrap();

            font_system.db_mut().load_system_fonts();

            // font_system
            //     .db_mut()
            //     .load_font_data(include_bytes!("../../assets/TestCalibre-Regular.otf").to_vec());
        }

        let mut layout_engine = LayoutEngine::default();
        layout_engine.disable_rounding();

        let atlas_manager = font_manager.atlas_manager_ref();
        let texture_manager = texture_manager.clone();

        let mut scene_resources = SceneResources::new(
            atlas_manager.clone(),
            texture_manager.clone(),
            font_manager.get_font_system_ref(),
            rendering_context,
            WindowScaleFactor::new(scale_fac as f32),
            &mut layout_engine,
        );

        let root = Root::new(&mut scene_resources).into();

        Self {
            font_manager,
            shape_renderer,
            atlas_manager,
            root,
            layout_engine,
            texture_manager,
        }
    }

    pub fn render(
        &mut self,
        render_surface: &RenderSurface,
        texture_manager: &TextureManagerRef,
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
            self.atlas_manager.clone(),
            self.texture_manager.clone(),
            self.font_manager.get_font_system_ref(),
            render_surface.clone_rendering_context(),
            scale_fac,
            &mut self.layout_engine,
        );

        let layout_pass = LayoutPass::new(&mut self.root, &mut scene_resources);

        let ElementTree {
            root: mut scene_layout,
            transformations,
            mut clip_rects,
        } = layout_pass.do_layout_pass(screen_size, scale_fac, &mut self.root);

        input.insert_transformations(transformations);
        scene_layout.do_input_pass(&mut input, None, &mut clip_rects, None);
        let transformations = input.take_transformations().unwrap();

        let mut scene_context =
            SceneContext::new(scale_fac, transformations, clip_rects, scene_resources);
        scene_layout.do_ui_pass(&mut scene_context, None, None);

        scene_context.output.accesskit_update().tree =
            Some(accesskit::Tree::new(self.root.id().as_access_id()));

        // render pass
        let SceneContext {
            shapes,
            clip_rects: scene_clip_rects,
            transformations: scene_transformations,
            output: platform_output,
            ..
        } = scene_context;

        let mut shape_buffer_local = ShapeBufferWithContext::new();

        let clip_rects = scene_clip_rects.finish(scale_fac).collect_vec();

        self.shape_renderer
            .write_all_clip_rects(render_ctx, &clip_rects);

        self.shape_renderer.write_all_transformations(
            render_ctx,
            &scene_transformations.transformations,
            &scene_transformations.transformation_inverses,
        );

        let mut texture_manager_lock = texture_manager.write().unwrap();

        let (texture_bind_group, sampler_bind_group) = {
            (
                // TODO: store these things
                texture_manager_lock.generate_texture_bind_group(device),
                texture_manager_lock.generate_sampler_bind_group(device),
            )
        };

        for PaintShapeWithContext {
            shape,
            clip_rect_idx,
            transformation_idx,
        } in shapes
        {
            shape_buffer_local.clip_rect_idx = clip_rect_idx.unwrap_or_default();
            shape_buffer_local.transformation_idx = transformation_idx.unwrap_or_default();

            match shape {
                ComputedPaintShape::Rectangle(paint_rect) => {
                    shape_buffer_local.push_quads(
                        BoxShaderVertex::from_paint_rect(&self.atlas_manager, paint_rect).0,
                    );
                }

                ComputedPaintShape::Text(text_box) => {
                    self.font_manager.prepare(text_box, &mut shape_buffer_local);
                }

                ComputedPaintShape::Mesh(mesh) => shape_buffer_local.push_vertices(
                    mesh.vertices
                        .into_iter()
                        .map(|PaintMeshVertex { color, pos }| {
                            BoxShaderVertex::mesh_tri(pos, color)
                        }),
                    mesh.indices,
                ),
            }
        }

        self.shape_renderer
            .write_all_shapes(queue, device, &shape_buffer_local.vertex_buffers);

        {
            let load_op = wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
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

            render_pass.set_pipeline(&self.shape_renderer.shape_render_pipeline);

            render_pass.set_bind_group(0, &self.shape_renderer.shape_bind_group, &[]);
            render_pass.set_bind_group(1, &texture_bind_group, &[]);
            render_pass.set_bind_group(2, &sampler_bind_group, &[]);

            self.shape_renderer.render(&mut render_pass, 0..1);
        }

        drop(texture_manager_lock);

        // TODO: for multiple render passes, submit multiple encoders as
        // iterator (??? might work, test performance)
        queue.submit(std::iter::once(encoder.finish()));
        window_texture.present();

        self.font_manager.collect_garbage();

        (input, platform_output)
    }

    pub fn root_id(&self) -> ElementId {
        self.root.id()
    }

    pub fn root_access_node(&mut self) -> AccessNode {
        self.root.get().node().build()
    }
}
