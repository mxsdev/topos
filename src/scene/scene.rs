use std::{
    ops::DerefMut,
    sync::{Arc, RwLock},
};

use bytemuck::Zeroable;
use itertools::Itertools;

use crate::{
    accessibility::AccessNode,
    atlas::{self, FontManager, TextureAtlasManager, TextureAtlasManagerRef},
    element::{Element, ElementId, ElementRef, RootConstructor},
    graphics::{DynamicGPUMeshTriBuffer, PushVertices, VertexBuffers},
    input::{input_state::InputState, output::PlatformOutput},
    math::{
        CompleteScaleFactor, DeviceScaleFactor, PhysicalSize, Pos, Rect, TransformationScaleFactor,
    },
    shape::{
        self, BoxShaderVertex, ClipRect, ComputedPaintShape, PaintMeshVertex, PaintShape,
        ShaderClipRect, ShapeBufferWithContext,
    },
    surface::{RenderAttachment, RenderSurface, RenderingContext},
    texture::TextureManagerRef,
    util::{
        guard::ReadLockable,
        text::{FontSystem, FontSystemRef, HasBuffer, TextBox, TextBoxLike},
    },
};

use super::{
    ctx::{PaintShapeWithContext, SceneContext},
    framepacer::{Framepacer, InstantLike, ManagedFramepacer},
    layout::{ElementTree, LayoutEngine, LayoutPass},
};

pub struct SceneResources<'a> {
    texture_atlas_manager: atlas::TextureAtlasManagerRef,
    texture_manager: TextureManagerRef,
    font_system: FontSystemRef,
    rendering_context: Arc<RenderingContext>,
    layout_engine: &'a mut LayoutEngine,
    font_manager: &'a mut FontManager,
    device_scale_factor: DeviceScaleFactor,

    pub(crate) element_clip_rect: Option<ClipRect>,
    pub(crate) element_transformation_scale_factor: Option<TransformationScaleFactor>,
}

impl<'a> SceneResources<'a> {
    pub fn new(
        texture_atlas_manager: atlas::TextureAtlasManagerRef,
        texture_manager: TextureManagerRef,
        font_system: FontSystemRef,
        rendering_context: Arc<RenderingContext>,
        device_scale_factor: DeviceScaleFactor,
        layout_engine: &'a mut LayoutEngine,
        font_manager: &'a mut FontManager,
    ) -> Self {
        Self {
            texture_atlas_manager,
            texture_manager,
            font_system,
            rendering_context,
            device_scale_factor,
            layout_engine,
            font_manager,

            element_clip_rect: Default::default(),
            element_transformation_scale_factor: Default::default(),
        }
    }

    pub(super) fn set_scale_factor(&mut self, fac: DeviceScaleFactor) {
        self.device_scale_factor = fac;
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

    pub fn device_scale_factor(&self) -> DeviceScaleFactor {
        self.device_scale_factor
    }

    pub fn scale_factor(&self) -> CompleteScaleFactor {
        self.device_scale_factor * self.element_transformation_scale_factor.unwrap_or_default()
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

    pub(crate) fn prepare_text<Buffer: HasBuffer + 'static>(&mut self, text: &TextBox<Buffer>) {
        self.font_manager.process_glyphs(
            &text.calculate_placed_text_box(self.element_clip_rect, self.scale_factor()),
        );
    }
}

pub struct Scene<Root: RootConstructor + 'static> {
    font_manager: atlas::FontManager,
    shape_renderer: shape::ShapeRenderer,
    atlas_manager: atlas::TextureAtlasManagerRef,
    texture_manager: TextureManagerRef,

    root: ElementRef<Root>,

    layout_engine: LayoutEngine,

    layout_result: Option<ElementTree>,
}

impl<Root: RootConstructor + 'static> Scene<Root> {
    pub fn new(
        rendering_context: Arc<RenderingContext>,
        render_surface: &RenderSurface,
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

        let mut layout_engine = LayoutEngine::new(font_manager.get_font_system_ref());
        layout_engine.disable_rounding();

        let atlas_manager = font_manager.atlas_manager_ref();
        let texture_manager = texture_manager.clone();

        let mut scene_resources = Self::get_scene_resources(
            &atlas_manager,
            &texture_manager,
            &mut font_manager,
            render_surface,
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
            layout_result: None,
        }
    }

    fn get_scene_resources<'a>(
        atlas_manager: &TextureAtlasManagerRef,
        texture_manager: &TextureManagerRef,
        font_manager: &'a mut FontManager,
        render_surface: &RenderSurface,
        layout_engine: &'a mut LayoutEngine,
    ) -> SceneResources<'a> {
        SceneResources::new(
            atlas_manager.clone(),
            texture_manager.clone(),
            font_manager.get_font_system_ref(),
            render_surface.clone_rendering_context(),
            render_surface.device_scale_factor(),
            layout_engine,
            font_manager,
        )
    }

    pub fn do_layout(&mut self, render_surface: &RenderSurface) -> ElementTree {
        let scale_fac = render_surface.device_scale_factor();

        let physical_screen_size: PhysicalSize<u32> = render_surface.get_size().into();

        let screen_size =
            physical_screen_size.cast_unit().map(|x| x as f32) * scale_fac.inverse().as_float();

        let mut scene_resources = Self::get_scene_resources(
            &self.atlas_manager,
            &self.texture_manager,
            &mut self.font_manager,
            render_surface,
            &mut self.layout_engine,
        );

        let layout_pass = LayoutPass::new(&mut self.root, &mut scene_resources);

        layout_pass.do_layout_pass(screen_size, &mut self.root)
    }

    pub fn render<I: InstantLike + Copy + std::fmt::Debug>(
        &mut self,
        render_surface: &RenderSurface,
        RenderAttachment {
            window_texture,
            msaa_view,
            ..
        }: RenderAttachment,
        ElementTree {
            root: mut scene_layout,
            transformations,
            mut clip_rects,
        }: ElementTree,
        mut input: InputState,
        start_time: I,
        fp: &mut (impl Framepacer<I> + ?Sized),
        time_context: &I::Context,
    ) -> (InputState, PlatformOutput, std::time::Duration, I) {
        let render_ctx = render_surface.rendering_context();

        let RenderingContext {
            device,
            queue,
            adapter,
            ..
        } = render_ctx;

        let window_view = window_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let scale_fac = render_surface.device_scale_factor();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // layout pass
        let mut scene_resources = Self::get_scene_resources(
            &self.atlas_manager,
            &self.texture_manager,
            &mut self.font_manager,
            render_surface,
            &mut self.layout_engine,
        );

        input.insert_transformations(transformations);
        scene_layout.do_input_pass(&mut input, None, &mut clip_rects, None, &mut scene_resources);
        let transformations = input.take_transformations().unwrap();

        let mut scene_context =
            SceneContext::new(scale_fac, transformations, clip_rects, scene_resources);
        scene_layout.do_ui_pass(&mut scene_context, None, None);

        {
            let root_id = self.root.id().as_access_id();
            scene_context.output.accesskit_update().focus = root_id;
            scene_context.output.accesskit_update().tree =
                Some(accesskit::Tree::new(root_id));
        }

        // render pass
        let SceneContext {
            shapes,
            clip_rects: scene_clip_rects,
            transformations: scene_transformations,
            output: platform_output,
            ..
        } = scene_context;

        let mut shape_buffer_local = ShapeBufferWithContext::new();

        let clip_rects = scene_clip_rects.finish().collect_vec();

        self.shape_renderer
            .write_all_clip_rects(render_ctx, &clip_rects);

        self.shape_renderer.write_all_transformations(
            render_ctx,
            &scene_transformations.transformations,
            &scene_transformations.transformation_inverses,
        );

        let mut texture_manager_lock = self.texture_manager.write().unwrap();

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
            .write_all_shapes(queue, device, shape_buffer_local.vertex_buffers);

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
                            store: wgpu::StoreOp::Store,
                        },
                    },
                    Some(msaa_view) => wgpu::RenderPassColorAttachment {
                        view: msaa_view,
                        resolve_target: Some(&window_view),
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Discard,
                        },
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: Default::default(),
                occlusion_query_set: Default::default(),
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

        fp.check_missed_deadline(
            I::now(time_context),
            start_time.elapsed(time_context).into(),
        );

        // window_texture.present(&wgpu::PresentationDescriptor {
        //     presentation_delay: wgpu::PresentationDelay::ScheduleTime(
        //         fp.get_deadline().expect("Deadline has not been set!"),
        //     ),
        // });

        let approx_present_time = I::now(&time_context);

        match (fp.desired_frame_time(), fp.desired_frame_instant()) {
            (Some(desired_frame_time), _) => {
                // window_texture.present(&wgpu::PresentationDescriptor {
                //     presentation_delay: wgpu::PresentationDelay::ScheduleMinimumDuration(
                //         desired_frame_time,
                //     ),
                // });
                window_texture.present();
            }

            (_, Some(desired_instant)) => {
                // window_texture.present(&wgpu::PresentationDescriptor {
                //     presentation_delay: wgpu::PresentationDelay::ScheduleTime(desired_instant),
                // });
                window_texture.present();
            }

            _ => {
                // window_texture.present(&Default::default());
                window_texture.present();
            }
        }

        self.font_manager.collect_garbage();

        let render_time = start_time.elapsed(time_context);

        (input, platform_output, render_time, approx_present_time)
    }

    pub fn root_id(&self) -> ElementId {
        self.root.id()
    }

    pub fn root_access_node(&mut self) -> AccessNode {
        self.root.get().node().build()
    }
}
