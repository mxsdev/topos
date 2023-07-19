use lyon::path::builder::WithSvg;

use crate::math::Pos;

pub type SVGParser<'a> = svg::Parser<'a>;
pub type SVGAttributes = svg::node::Attributes;

pub type TessellationPath = lyon::path::Path;

pub trait SVGParsable<'a> {
    fn parse_as_svg(self) -> svg::Parser<'a>;
}

impl<'a> SVGParsable<'a> for SVGParser<'a> {
    fn parse_as_svg(self) -> svg::Parser<'a> {
        self
    }
}

impl<'a> SVGParsable<'a> for &'a str {
    fn parse_as_svg(self) -> svg::Parser<'a> {
        svg::Parser::new(self)
    }
}

pub fn build_lyon_from_svg_path_attributes(
    attributes: SVGAttributes,
    path: &mut WithSvg<impl lyon::path::builder::PathBuilder>,
) {
    use lyon::path::traits::SvgPathBuilder;
    use svg::node::element::path::Data;

    let data = attributes.get("d").unwrap();
    let data = Data::parse(data).unwrap();

    use lyon::math::{point, vector};
    use svg::node::element::path::{Command, Position};

    for command in data.into_iter() {
        match command {
            Command::Move(pos, args) => {
                for pt in args.chunks_exact(2) {
                    match pos {
                        Position::Absolute => {
                            path.move_to(point(pt[0], pt[1]));
                        }
                        Position::Relative => {
                            path.relative_move_to(vector(pt[0], pt[1]));
                        }
                    }
                }
            }

            Command::Line(pos, args) => {
                for pt in args.chunks_exact(2) {
                    match pos {
                        Position::Absolute => {
                            path.line_to(point(pt[0], pt[1]));
                        }
                        Position::Relative => {
                            path.relative_line_to(vector(pt[0], pt[1]));
                        }
                    }
                }
            }

            Command::HorizontalLine(pos, args) => {
                for pt in args.iter() {
                    match pos {
                        Position::Absolute => {
                            path.horizontal_line_to(*pt);
                        }
                        Position::Relative => {
                            path.relative_horizontal_line_to(*pt);
                        }
                    }
                }
            }

            Command::VerticalLine(pos, args) => {
                for pt in args.iter() {
                    match pos {
                        Position::Absolute => {
                            path.vertical_line_to(*pt);
                        }
                        Position::Relative => {
                            path.relative_vertical_line_to(*pt);
                        }
                    }
                }
            }

            Command::QuadraticCurve(pos, args) => {
                for pt in args.chunks_exact(4) {
                    match pos {
                        Position::Absolute => {
                            path.quadratic_bezier_to(point(pt[0], pt[1]), point(pt[2], pt[3]));
                        }
                        Position::Relative => {
                            path.relative_quadratic_bezier_to(
                                vector(pt[0], pt[1]),
                                vector(pt[2], pt[3]),
                            );
                        }
                    }
                }
            }

            Command::CubicCurve(pos, args) => {
                for pt in args.chunks_exact(6) {
                    match pos {
                        Position::Absolute => {
                            path.cubic_bezier_to(
                                point(pt[0], pt[1]),
                                point(pt[2], pt[3]),
                                point(pt[4], pt[5]),
                            );
                        }
                        Position::Relative => {
                            path.relative_cubic_bezier_to(
                                vector(pt[0], pt[1]),
                                vector(pt[2], pt[3]),
                                vector(pt[4], pt[5]),
                            );
                        }
                    }
                }
            }

            Command::SmoothQuadraticCurve(pos, args) => {
                for pt in args.chunks_exact(2) {
                    match pos {
                        Position::Absolute => {
                            path.smooth_quadratic_bezier_to(point(pt[0], pt[1]));
                        }
                        Position::Relative => {
                            path.smooth_relative_quadratic_bezier_to(vector(pt[0], pt[1]));
                        }
                    }
                }
            }

            Command::SmoothCubicCurve(pos, args) => {
                for pt in args.chunks_exact(4) {
                    match pos {
                        Position::Absolute => {
                            path.smooth_cubic_bezier_to(point(pt[0], pt[1]), point(pt[2], pt[3]));
                        }
                        Position::Relative => {
                            path.smooth_relative_cubic_bezier_to(
                                vector(pt[0], pt[1]),
                                vector(pt[2], pt[3]),
                            );
                        }
                    }
                }
            }

            Command::EllipticalArc(pos, args) => {
                for pt in args.chunks_exact(7) {
                    let [rx, ry, angle, large_arc_flag, sweep_flag, x, y] =
                        [pt[0], pt[1], pt[2], pt[3], pt[4], pt[5], pt[6]];

                    let radius = [rx, ry].into();
                    let angle = lyon::math::Angle::degrees(angle);
                    let arc_flags = lyon::path::ArcFlags {
                        large_arc: large_arc_flag != 0.,
                        sweep: sweep_flag != 0.,
                    };
                    let pos_to = [x, y];

                    match pos {
                        Position::Absolute => {
                            path.arc_to(radius, angle, arc_flags, pos_to.into());
                        }
                        Position::Relative => {
                            path.relative_arc_to(radius, angle, arc_flags, pos_to.into());
                        }
                    }
                }
            }

            Command::Close => {
                path.close();
            }
        }
    }
}

pub fn svg_path_attributes_to_lyon(attributes: SVGAttributes) -> TessellationPath {
    let mut path = TessellationPath::builder().with_svg();

    build_lyon_from_svg_path_attributes(attributes, &mut path);

    path.build()
}

/// Convert single SVG Path to tessellation path.
///
/// Colors, stroke width/style, and other metadata is ignored.
pub fn svg_path_to_lyon<'a>(svg: impl SVGParsable<'a>) -> Option<TessellationPath> {
    use svg::{node::element::tag, parser::Event};

    for event in svg.parse_as_svg() {
        match event {
            Event::Tag(tag::Path, _, attributes) => {
                return Some(svg_path_attributes_to_lyon(attributes))
            }
            _ => {}
        }
    }

    None
}

/// Converts multiple SVG Paths to tessellation paths.
///
/// Colors, stroke width/style, and other metadata is ignored.
pub fn svg_to_lyon<'a>(svg: impl SVGParsable<'a>) -> impl Iterator<Item = TessellationPath> + 'a {
    use svg::{node::element::tag, parser::Event};

    svg.parse_as_svg().flat_map(|event| match event {
        Event::Tag(tag::Path, _, attributes) => Some(svg_path_attributes_to_lyon(attributes)),
        _ => None,
    })
}

pub type PosVertexInfo = Pos;
pub type PosVertexBuffers = crate::graphics::VertexBuffers<PosVertexInfo>;
pub struct PosVertexCtor;

impl lyon::tessellation::FillVertexConstructor<PosVertexInfo> for PosVertexCtor {
    fn new_vertex(&mut self, vertex: lyon::lyon_tessellation::FillVertex) -> PosVertexInfo {
        let pos = vertex.position();
        Pos::new(pos.x, pos.y)
    }
}

impl lyon::tessellation::StrokeVertexConstructor<PosVertexInfo> for PosVertexCtor {
    fn new_vertex(&mut self, vertex: lyon::lyon_tessellation::StrokeVertex) -> PosVertexInfo {
        let pos = vertex.position();
        Pos::new(pos.x, pos.y)
    }
}
