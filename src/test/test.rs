use keyframe::{functions::BezierCurve, mint::Vector2};
use num_traits::Signed;
use palette::{rgb::Rgb, Srgba};

use crate::{
    element::transition::Transition,
    input::PointerButton,
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
        self.clicked = self.hovered && ctx.input().pointer.primary_clicked();

        if self.hovered {
            if ctx.input().pointer.primary_pressed() {
                self.dragging = true;
            }
        }

        if self.dragging {
            let del = ctx.input().pointer.delta();
            self.rect.translate_mut(del.x, del.y);

            if ctx.input().pointer.primary_released() {
                self.dragging = false;
            }
        } else {
            if let Some(hover) = ctx.input().pointer.hover_pos() {
                self.hovered = self.rect.sdf(&hover).is_positive()
            } else {
                self.hovered = false;
            };
        }

        if self.hovered || self.dragging {
            ctx.input().pointer.consume_hover();
        }

        self.transition.set_state(self.hovered);
        self.transition.update(ctx);

        use palette::Mix;
        let fill = Srgba::mix(
            Srgba::new(1., 0., 0., 1.),
            Srgba::new(0., 1., 0., 1.),
            self.transition.fac(),
        );

        // let fill = match self.hovered {
        //     true => Srgba::new(1., 0., 0., 1.),
        //     false => Srgba::new(0., 1., 0., 1.),
        // };

        ctx.add_shape(PaintRectangle {
            rect: self.rect.translate_vec(Vec2::new(0., 0.)),
            fill: Some(fill),
            stroke_color: Some(Srgba::new(0., 0., 0., 1.)),
            stroke_width: Some(1.),
            blur: Some(PaintBlur::new(10., Srgba::new(0., 0., 0., 0.5))),
        });

        Default::default()
    }

    fn layout(
        &mut self,
        constraints: SizeConstraint,
        layout_pass: &mut crate::scene::layout::LayoutPass,
    ) -> Size2 {
        Size2::zero()
    }

    // fn update(&mut self, event: &ElementEvent, update: &mut UpdatePass) -> bool {
    //     match event {
    //         ElementEvent::CursorMove {
    //             pos: mouse_pos,
    //             del,
    //         } => {
    //             if let Some(del) = del {
    //                 if self.dragging {
    //                     self.rect.rect = self.rect.rect.translate(*del)
    //                 }
    //             }

    //             let inside = self.rect.sdf(&mouse_pos).is_positive();

    //             if self.hovered != inside {
    //                 log::trace!("hover changed: {:?}", inside)
    //             }

    //             if inside {
    //                 update.consume_hover();
    //             }

    //             self.hovered = inside;

    //             return inside;
    //         }

    //         ElementEvent::MouseDown {
    //             button: MouseButton::Left,
    //         } => {
    //             if self.hovered {
    //                 self.dragging = true;
    //             }
    //         }

    //         ElementEvent::MouseUp {
    //             button: MouseButton::Left,
    //         } => {
    //             self.dragging = false;
    //         }

    //         _ => {}
    //     }

    //     false
    // }

    // fn paint(&mut self, painter: &mut PaintPass) {
    //     let fill = match self.hovered {
    //         true => Srgba::new(1., 0., 0., 1.),
    //         false => Srgba::new(0., 1., 0., 1.),
    //     };

    //     painter.add(PaintRectangle {
    //         rect: self.rect,
    //         fill: Some(fill),
    //         stroke_color: Some(Srgba::new(0., 0., 0., 1.)),
    //         stroke_width: Some(1.),
    //         blur: Some(PaintBlur::new(10., Srgba::new(0., 0., 0., 0.5))),
    //     })
    // }
}
