use keyframe::{functions::BezierCurve, mint::Vector2};
use lyon::{
    lyon_tessellation::StrokeOptions,
    path::{LineCap, LineJoin},
};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole, AsAccessRect},
    color::ColorRgba,
    element::transition::Transition,
    graphics::VertexBuffers,
    input::input_state::InputState,
    lib::Response,
    math::{Pos, Rect, RoundedRect, Size, Vector},
    scene::{
        ctx::SceneContext,
        layout::{LayoutPassResult, Manual},
    },
    shape::{PaintMesh, PaintRectangle},
    util::svg::{svg_path_to_lyon, PosVertexBuffers, PosVertexCtor},
};

use crate::element::Element;

pub struct TestRect {
    size: Size,

    pub response: Response,
    drag: Vector,

    transition: Transition,

    glyph_tris: PosVertexBuffers,
}

impl TestRect {
    pub fn new(pos: Pos) -> Self {
        let curve = BezierCurve::from(Vector2 { x: 0.62, y: 0. }, Vector2 { x: 0.43, y: 0.98 });

        let mut glyph_tris = VertexBuffers::new();

        let path = svg_path_to_lyon(include_str!("../icon/alert-octagon.svg"))
            .expect("failed to parse svg");

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

        Self {
            size: Size::new(180., 180.),

            response: Response::new(RoundedRect::default().with_radius_from(10.))
                .with_clickable(true)
                .with_focusable(true)
                .with_hoverable(true),
            drag: pos.to_vector(),

            transition: Transition::new(0.15).set_ease_func(curve),

            glyph_tris,
        }
    }
}

impl Element for TestRect {
    fn ui(&mut self, ctx: &mut SceneContext, _rect: Rect) {
        use palette::Mix;
        let fill = ColorRgba::mix(
            ColorRgba::new(1., 0., 0., 1.),
            ColorRgba::new(0., 1., 0., 1.),
            self.transition.fac(),
        );

        ctx.add_shape(
            PaintRectangle::from_rect(self.response.boundary)
                .with_fill(fill)
                .with_stroke(ColorRgba::new(0., 0., 0., 0.75), 1.)
                .with_blur(30., ColorRgba::new(0., 0., 0., 0.75)),
        );

        ctx.add_shape(PaintMesh::from_pos_vertex_buffers(
            &self.glyph_tris,
            ColorRgba::new(0., 0., 0., 1.),
            self.response.boundary.min() + Vector::splat(10.),
        ));

        if self.response.focused() {
            ctx.add_shape(
                PaintRectangle::from_rect(self.response.boundary.inflate(1., 1.).with_radius(None))
                    .with_stroke(ColorRgba::new(1., 1., 0., 1.), 1.),
            );
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

    fn layout(&mut self, layout_pass: &mut crate::scene::layout::LayoutPass) -> LayoutPassResult {
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
