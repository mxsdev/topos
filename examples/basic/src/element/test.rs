use topos::keyframe::{functions::BezierCurve, mint::Vector2};
use topos::lyon::geom::euclid::{Size2D, Vector2D};
use topos::lyon::geom::{Box2D, euclid::Point2D};
use topos::lyon::path::builder::BorderRadii;
use topos::lyon::path::traits::SvgPathBuilder;
use topos::lyon::path::{ArcFlags, Winding};
use topos::lyon::{
    self,
    lyon_tessellation::StrokeOptions,
    path::{LineCap, LineJoin},
};

use topos::util::svg::TessellationPath;
use topos::{
    accessibility::{AccessNodeBuilder, AccessRole, AsAccessRect},
    atlas::AtlasAllocation,
    color::ColorRgba,
    element::Element,
    element::{transition::Transition, Response},
    graphics::VertexBuffers,
    input::input_state::InputState,
    math::{PhysicalSize, Pos, Rect, RoundedRect, Size, Vector},
    scene::{
        ctx::SceneContext,
        layout::{LayoutPass, LayoutPassResult, Manual},
        scene::SceneResources,
    },
    shape::{PaintFill, PaintMesh, PaintRectangle},
    util::{
        svg::{svg_path_to_lyon, PosVertexBuffers, PosVertexCtor},
        text::AtlasContentType,
    },
};

pub struct TestRect {
    size: Size,

    pub response: Response,
    drag: Vector,

    transition: Transition,

    glyph_tris: PosVertexBuffers,

    image_allocation: AtlasAllocation,
}

impl TestRect {
    pub fn new(resources: &mut SceneResources, pos: Pos) -> Self {
        let curve = BezierCurve::from(Vector2 { x: 0.62, y: 0. }, Vector2 { x: 0.43, y: 0.98 });

        
        let path = svg_path_to_lyon(include_str!("../icon/alert-octagon.svg"))
            .expect("failed to parse svg");
    
        let mut glyph_tris = VertexBuffers::new();

        let mut buffers = lyon::tessellation::BuffersBuilder::new(&mut glyph_tris, PosVertexCtor);

        lyon::tessellation::StrokeTessellator::new()
            .tessellate_path(
                &path,
                &StrokeOptions::default()
                    .with_line_cap(LineCap::Round)
                    .with_line_join(LineJoin::Round)
                    .with_line_width(2.)
                    .with_tolerance(StrokeOptions::DEFAULT_TOLERANCE * 0.5 / 4.),
                &mut buffers,
            )
            .unwrap();

        let image_allocation = {
            let mut atlas_manager = resources.texture_atlas_manager().write().unwrap();

            let s = PhysicalSize::new(2, 1);

            let image_allocation = atlas_manager
                .allocate(resources.texture_manager(), AtlasContentType::Color, s)
                .unwrap();

            atlas_manager.get_atlas(&image_allocation).write_texture(
                &resources.rendering_context_ref(),
                &image_allocation,
                &[0xFF, 0xEC, 0xD2, 0xFF, 0xFC, 0xB6, 0x9F, 0xFF],
                // &[0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            );

            image_allocation
        };

        Self {
            size: Size::new(180., 180.),

            response: Response::new(RoundedRect::default().with_radius(Some(10.)))
                .with_clickable(true)
                .with_focusable(true)
                .with_hoverable(true)
                .with_focus_on_mouse_down(true),
            drag: pos.to_vector(),

            transition: Transition::new(0.15).set_ease_func(curve),

            glyph_tris,

            image_allocation,
        }
    }
}

impl Element for TestRect {
    fn ui(&mut self, ctx: &mut SceneContext, _rect: Rect) {
        ctx.add_shape(
            PaintRectangle::from_rect(self.response.boundary)
                .with_fill(PaintFill::from_atlas_allocation_uv(
                    &self.image_allocation,
                    // Rect::new(Pos::new(2. - 0.5, 1.), Pos::new(3. - 0.5, 2.)),
                    Rect::new(Pos::new(0.5, 0.5), Pos::new(1.5, 0.5)),
                ))
                // .with_stroke(ColorRgba::new(1., 1., 1., 1.), 1.)
                .with_blur(30., ColorRgba::new(0., 0., 0., 0.75)),
        );

        ctx.add_shape(PaintMesh::from_pos_vertex_buffers(
            &self.glyph_tris,
            ColorRgba::new(0., 0., 0., 1.),
            self.response.boundary.min() + Vector::splat(10.),
        ));

        if self.response.focused() || self.response.hovered() {
            let rect = self.response.boundary.inner;
            let size = rect.size();

            let rounding = self.response.boundary.radius.unwrap_or_default();

            let mut builder = TessellationPath::builder().with_svg();

            let radius = 5. + self.transition.fac() * 5.;

            ctx.add_shape(PaintRectangle::from_rect(
                RoundedRect::from_rect(Rect::from_origin_size(
                    rect.min + Vector::new(0., size.width / 2.),
                    Size::splat(radius * 2.),
                )).with_radius(Some(radius))
            ).with_stroke(ColorRgba::new(1., 1., 0., 1.), 2.));

            // builder.move_to(
            //     Point2D::new(0., rounding),
            // );

            // builder.line_to(
            //     Point2D::new(0., size.width / 2. - 10.),
            // );

            builder.move_to(
                Point2D::new(0., size.width / 2. + radius),
            );

            builder.line_to(
                Point2D::new(0., size.height - rounding),
            );

            builder.arc_to(
                Vector2D::splat(rounding),
                lyon::path::math::Angle::degrees(90.),
                ArcFlags::default(),
                Point2D::new(rounding, size.height),
            );

            builder.line_to(
                Point2D::new(size.width - rounding, size.height),
            );

            builder.arc_to(
                Vector2D::splat(rounding),
                lyon::path::math::Angle::degrees(90.),
                ArcFlags::default(),
                Point2D::new(size.width, size.height - rounding),
            );

            builder.line_to(
                Point2D::new(size.width, rounding),
            );

            builder.arc_to(
                Vector2D::splat(rounding),
                lyon::path::math::Angle::degrees(90.),
                ArcFlags::default(),
                Point2D::new(size.width - rounding, 0.),
            );

            builder.line_to(
                Point2D::new(rounding, 0.),
            );

            builder.arc_to(
                Vector2D::splat(rounding),
                lyon::path::math::Angle::degrees(90.),
                ArcFlags::default(),
                Point2D::new(0., rounding),
            );

            builder.line_to(
                Point2D::new(0., size.width / 2. - radius),
            );

            // builder.arc_to(
            //     Vector2D::splat(rounding),
            //     lyon::path::math::Angle::degrees(90.),
            //     ArcFlags::default(),
            //     Point2D::new(rounding, size.height - rounding),
            // );

            // builder.arc_to(
            //     Vector2D::splat(rounding),
            //     lyon::path::math::Angle::radians(0.),
            //     ArcFlags::default(),
            //     Point2D::new(rect.max.x, rect.min.y),
            // );

            // builder.close();


            // builder.add_rounded_rectangle(
            //     &Box2D::from_size(Size2D::new(self.response.boundary.inner.width(), self.response.boundary.inner.height())),
            //     &BorderRadii::new(rounding), Winding::Positive
            // );

            let path = builder.build();

            let mut glyph_tris = VertexBuffers::new();

            let mut buffers = lyon::tessellation::BuffersBuilder::new(&mut glyph_tris, PosVertexCtor);

            lyon::tessellation::StrokeTessellator::new()
                .tessellate_path(
                    &path,
                    &StrokeOptions::default()
                        .with_line_cap(LineCap::Square)
                        .with_line_join(LineJoin::Round)
                        .with_line_width(2.)
                        .with_tolerance(StrokeOptions::DEFAULT_TOLERANCE * 0.5 / 4.),
                    &mut buffers,
                )
                .unwrap();

            ctx.add_shape(PaintMesh::from_pos_vertex_buffers(
                &glyph_tris,
                ColorRgba::new(1., 1., 0., 1.),
                self.response.boundary.min(),
            ));
            
            // ctx.add_shape(
            //     PaintRectangle::from_rect(self.response.boundary.inflate(1., 1.).with_radius(None))
            //         .with_stroke(ColorRgba::new(1., 1., 0., 1.), 1.),
            // );
        }
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        self.response
            .update_rect(input, Rect::from_min_size(rect.min + self.drag, self.size));

        if self.response.primary_button_down_on() {
            let del = input.pointer.delta();
            self.drag += del;
        }

        self.transition
            .set_state(self.response.hovered_or_primary_down_on());
        self.transition.update(input);
    }

    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        layout_pass
            .engine()
            .new_leaf(Manual::builder())
            .unwrap()
            .into()
    }

    fn node(&self) -> AccessNodeBuilder {
        let mut builder = AccessNodeBuilder::new(AccessRole::GenericContainer);
        builder.set_bounds(self.response.boundary.as_access_rect());
        builder
    }
}
