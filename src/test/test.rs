use keyframe::{functions::BezierCurve, mint::Vector2};
use num_traits::Signed;

use crate::{
    color::ColorRgba,
    element::transition::Transition,
    input::{input_state::InputState, output::CursorIcon, PointerButton},
    scene::{ctx::SceneContext, update::UpdatePass, PaintPass},
    shape::{PaintBlur, PaintRectangle},
    util::{FromMinSize, Pos2, Rect, RoundedRect, Size2, Translate2D, Translate2DMut, Vec2},
};

use crate::element::{boundary::Boundary, Element, ElementEvent, MouseButton, SizeConstraint};

pub struct TestRect {
    rect: RoundedRect,
    hovered: bool,
    dragging: bool,
    pub clicked: bool,

    transition: Transition,
}

impl TestRect {
    pub fn new(pos: Pos2) -> Self {
        // keyframe::ease(function, from, to, time)
        // keyframe::functions::BezierCurve::from([])

        let curve = BezierCurve::from(Vector2 { x: 0.62, y: 0. }, Vector2 { x: 0.43, y: 0.98 });
        //  BezierCurve::from([.62,0.].into(),[.43,.98].into())

        Self {
            rect: RoundedRect::new(
                // Rect::new(Pos2::new(20., 20.), Pos2::new(200., 100.)),
                Rect::from_min_size(pos, Size2::new(180., 180.)),
                Some(10.),
            ),
            hovered: false,
            dragging: false,
            clicked: false,

            // ease_func: Box::new(keyframe::functions::Linear),
            transition: Transition::new(0.15).set_ease_func(curve),
        }
    }
}

impl Element for TestRect {
    fn ui(&mut self, ctx: &mut SceneContext, pos: Pos2) {
        use palette::Mix;
        let fill = ColorRgba::mix(
            ColorRgba::new(1., 0., 0., 1.),
            ColorRgba::new(0., 1., 0., 1.),
            self.transition.fac(),
        );

        // let fill = match self.hovered {
        //     true => ColorRgba::new(1., 0., 0., 1.),
        //     false => ColorRgba::new(0., 1., 0., 1.),
        // };

        ctx.add_shape(PaintRectangle {
            rect: self.rect.translate_vec(Vec2::new(0., 0.)),
            fill: Some(fill),
            stroke_color: Some(ColorRgba::new(0., 0., 0., 1.)),
            stroke_width: Some(1.),
            blur: Some(PaintBlur::new(10., ColorRgba::new(0., 0., 0., 0.5))),
        });
    }

    fn input(&mut self, input: &mut InputState, pos: Pos2) {
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
                self.hovered = self.rect.sdf(&hover).is_positive()
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
        constraints: SizeConstraint,
        layout_pass: &mut crate::scene::layout::LayoutPass,
    ) -> Size2 {
        Size2::zero()
    }
}
