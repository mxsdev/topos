use itertools::Itertools;
use keyframe::{functions::BezierCurve, mint::Vector2};
use lyon::{lyon_tessellation::{FillOptions, StrokeOptions}, path::{traits::SvgPathBuilder, LineCap, LineJoin}};
use num_traits::Signed;
use svg::node::element::{tag, path::Data};

use crate::{
    color::ColorRgba,
    element::transition::Transition,
    graphics::VertexBuffers,
    input::{input_state::InputState, output::CursorIcon, PointerButton},
    mesh::{MeshVertex, PaintMesh, PaintMeshVertex},
    scene::{ctx::SceneContext, update::UpdatePass, PaintPass, layout::{LayoutPassResult, Manual, FlexBox}},
    shape::{PaintBlur, PaintRectangle, PaintShape},
    util::{FromMinSize, Pos2, Rect, RoundedRect, Size2, Translate2D, Translate2DMut, Vec2}, accessibility::{AccessNodeBuilder, AccessRole, AsAccessRect},
};

use crate::element::{boundary::Boundary, Element, ElementEvent, MouseButton, SizeConstraint};

pub struct TestRect {
    rect: RoundedRect,
    input_rect: RoundedRect,

    hovered: bool,
    dragging: bool,
    pub clicked: bool,

    transition: Transition,

    glyph_tris: VertexBuffers<Pos2>,
}

struct VertexCtor;
impl lyon::tessellation::FillVertexConstructor<Pos2> for VertexCtor {
    fn new_vertex(&mut self, vertex: lyon::lyon_tessellation::FillVertex) -> Pos2 {
        let pos = vertex.position();
        Pos2::new(pos.x, pos.y)
    }
}

impl lyon::tessellation::StrokeVertexConstructor<Pos2> for VertexCtor {
    fn new_vertex(&mut self, vertex: lyon::lyon_tessellation::StrokeVertex) -> Pos2 {
        let pos = vertex.position();
        Pos2::new(pos.x, pos.y)
    }
}

impl TestRect {
    pub fn new(pos: Pos2) -> Self {
        // keyframe::ease(function, from, to, time)
        // keyframe::functions::BezierCurve::from([])

        let curve = BezierCurve::from(Vector2 { x: 0.62, y: 0. }, Vector2 { x: 0.43, y: 0.98 });
        //  BezierCurve::from([.62,0.].into(),[.43,.98].into())

        let mut glyph_tris = VertexBuffers::new();
        let mut path = lyon::path::Path::builder().with_svg();

        let svg_data = include_str!("../icon/alert-octagon.svg");

        use svg::{Parser, parser::Event};
        
        for event in Parser::new(svg_data) {
            match event {
                Event::Tag(tag::Path, _, attributes) => {
                    let data = attributes.get("d").unwrap();
                    let data = Data::parse(data).unwrap();

                    use svg::node::element::path::{Position, Command};
                    use lyon::math::{point, vector};

                    for command in data.into_iter() {
                        match command {
                            Command::Move(pos, args) => {
                                for pt in args.chunks_exact(2) {
                                    match pos {
                                        Position::Absolute => { path.move_to(point(pt[0], pt[1])); },
                                        Position::Relative => { path.relative_move_to(vector(pt[0], pt[1])); },
                                    }
                                }
                            },

                            Command::Line(pos, args) => {
                                for pt in args.chunks_exact(2) {
                                    match pos {
                                        Position::Absolute => { path.line_to(point(pt[0], pt[1])); },
                                        Position::Relative => { path.relative_line_to(vector(pt[0], pt[1])); },
                                    }
                                }
                            },

                            Command::HorizontalLine(pos, args) => {
                                for pt in args.iter() {
                                    match pos {
                                        Position::Absolute => { path.horizontal_line_to(*pt); },
                                        Position::Relative => { path.relative_horizontal_line_to(*pt); },
                                    }
                                }
                            },

                            Command::VerticalLine(pos, args) => {
                                for pt in args.iter() {
                                    match pos {
                                        Position::Absolute => { path.vertical_line_to(*pt); },
                                        Position::Relative => { path.relative_vertical_line_to(*pt); },
                                    }
                                }
                            },

                            Command::QuadraticCurve(pos, args) => {
                                for pt in args.chunks_exact(4) {
                                    match pos {
                                        Position::Absolute => { path.quadratic_bezier_to(point(pt[0], pt[1]), point(pt[2], pt[3])); },
                                        Position::Relative => { path.relative_quadratic_bezier_to(vector(pt[0], pt[1]), vector(pt[2], pt[3])); },
                                    }
                                }
                            },

                            Command::CubicCurve(pos, args) => {
                                for pt in args.chunks_exact(6) {
                                    match pos {
                                        Position::Absolute => { path.cubic_bezier_to(point(pt[0], pt[1]), point(pt[2], pt[3]), point(pt[4], pt[5])); },
                                        Position::Relative => { path.relative_cubic_bezier_to(vector(pt[0], pt[1]), vector(pt[2], pt[3]), vector(pt[4], pt[5])); },
                                    }
                                }
                            },

                            Command::SmoothQuadraticCurve(pos, args) => {
                                for pt in args.chunks_exact(2) {
                                    match pos {
                                        Position::Absolute => { path.smooth_quadratic_bezier_to(point(pt[0], pt[1])); },
                                        Position::Relative => { path.smooth_relative_quadratic_bezier_to(vector(pt[0], pt[1])); },
                                    }
                                }
                            },

                            Command::SmoothCubicCurve(pos, args) => {
                                for pt in args.chunks_exact(4) {
                                    match pos {
                                        Position::Absolute => { path.smooth_cubic_bezier_to(point(pt[0], pt[1]), point(pt[2], pt[3])); },
                                        Position::Relative => { path.smooth_relative_cubic_bezier_to(vector(pt[0], pt[1]), vector(pt[2], pt[3])); },
                                    }
                                }
                            },

                            Command::EllipticalArc(pos, args) => {
                                for pt in args.chunks_exact(7) {
                                    let [rx, ry, angle, large_arc_flag, sweep_flag, x, y] = [pt[0], pt[1], pt[2], pt[3], pt[4], pt[5], pt[6]];

                                    let radius = [rx, ry].into();
                                    let angle = lyon::math::Angle::degrees(angle);
                                    let arc_flags = lyon::path::ArcFlags {
                                        large_arc: large_arc_flag != 0.,
                                        sweep: sweep_flag != 0.,
                                    };
                                    let pos_to = [x, y];

                                    match pos {
                                        Position::Absolute => { path.arc_to(radius, angle, arc_flags, pos_to.into()); },
                                        Position::Relative => { path.relative_arc_to(radius, angle, arc_flags, pos_to.into()); },
                                    }

                                }
                                // path.arc_to(radii, x_rotation, flags, to)
                            },

                            Command::Close => {
                                path.close();
                            },
                        }
                    }
                    
                    // attributes.get("")
                }

                Event::Tag(tag::SVG, _, attributes) => {
                    
                }

                _ => panic!()
                // svg::parser::Event::Error(_) => todo!(),
                // svg::parser::Event::Text(_) => todo!(),
                // svg::parser::Event::Comment(_) => todo!(),
                // svg::parser::Event::Declaration(_) => todo!(),
                // svg::parser::Event::Instruction(_) => todo!(),
            }
        }

        // lyon::extra::rust_logo::build_logo_path(&mut path);

        let path = path.build();

        let mut buffers = lyon::tessellation::BuffersBuilder::new(&mut glyph_tris, VertexCtor);

        // lyon::tessellation::FillTessellator::new()
        //     .tessellate_path(&path, &FillOptions::default(), &mut buffers)
        //     .unwrap();

        lyon::tessellation::StrokeTessellator::new()
            .tessellate_path(&path, &StrokeOptions::default().with_line_cap(LineCap::Round).with_line_join(LineJoin::Round).with_line_width(2.).with_tolerance(StrokeOptions::DEFAULT_TOLERANCE * 0.5 / 4.), &mut buffers).unwrap();

        Self {
            rect: RoundedRect::new(
                // Rect::new(Pos2::new(20., 20.), Pos2::new(200., 100.)),
                Rect::from_min_size(pos, Size2::new(180., 180.)),
                Some(10.),
                // None,
            ),
            hovered: false,
            dragging: false,
            clicked: false,

            input_rect: Rect::zero().into(),

            // ease_func: Box::new(keyframe::functions::Linear),
            transition: Transition::new(0.15).set_ease_func(curve),

            glyph_tris,
        }
    }
}

impl Element for TestRect {
    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        use palette::Mix;
        let fill = ColorRgba::mix(
            ColorRgba::new(1., 0., 0., 1.),
            ColorRgba::new(0., 1., 0., 1.),
            self.transition.fac(),
        );

        ctx.add_shape(PaintRectangle {
            rect: self.input_rect,
            fill: Some(fill),
            stroke_color: Some(ColorRgba::new(0., 0., 0., 1.)),
            stroke_width: Some(1.),
            blur: Some(PaintBlur::new(30., ColorRgba::new(0., 0., 0., 0.75))),
        });

        ctx.add_shape(PaintMesh {
            indices: self.glyph_tris.indices.clone(),
            vertices: self
                .glyph_tris
                .vertices
                .iter()
                .map(|p| PaintMeshVertex {
                    pos: (*p * 1.) + self.input_rect.min.to_vector() + Vec2::splat(10.),
                    color: ColorRgba::new(0., 0., 0., 1.).into(),
                })
                .collect(),
        })
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        self.input_rect = self.rect.translate_vec(rect.min.to_vector());
        
        self.clicked = self.hovered && input.pointer.primary_clicked();

        if self.hovered {
            if input.pointer.primary_pressed() {
                self.dragging = true;
            }
        }

        if self.dragging {
            let del = input.pointer.delta();
            self.rect.translate_mut(del.x, del.y);

            if input.pointer.primary_released() {
                self.dragging = false;
            }
        } else {
            if let Some(hover) = input.pointer.hover_pos() {
                self.hovered = self.input_rect.sdf(&hover).is_positive()
            } else {
                self.hovered = false;
            };
        }

        if self.hovered || self.dragging {
            input.pointer.consume_hover();
        }

        self.transition.set_state(self.hovered);
        self.transition.update(input);
    }

    fn layout(
        &mut self,
        layout_pass: &mut crate::scene::layout::LayoutPass,
    ) -> LayoutPassResult {
        layout_pass.engine().new_leaf(Manual::builder().to_taffy()).unwrap()
    }

    fn node(&self) -> AccessNodeBuilder {
        let mut builder = AccessNodeBuilder::new(AccessRole::GenericContainer);
        builder.set_bounds(self.rect.rect.as_access_rect());
        builder
    }
}
