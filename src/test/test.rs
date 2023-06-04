use num_traits::Signed;
use palette::Srgba;

use crate::{
    input::PointerButton,
    scene::{ctx::SceneContext, update::UpdatePass, PaintPass},
    shape::{PaintBlur, PaintRectangle},
    util::{Pos2, Rect, RoundedRect, Size2, Translate2DMut},
};

use crate::element::{boundary::Boundary, Element, ElementEvent, MouseButton, SizeConstraint};

pub struct TestElement {
    rect: RoundedRect,
    hovered: bool,
    dragging: bool,
}

impl TestElement {
    pub fn new() -> Self {
        Self {
            rect: RoundedRect::new(
                Rect::new(Pos2::new(20., 20.), Pos2::new(200., 100.)),
                Some(10.),
            ),
            hovered: false,
            dragging: false,
        }
    }
}

impl Element for TestElement {
    fn ui(&mut self, ctx: &mut SceneContext, pos: Pos2) {
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
            };
        }

        let fill = match self.hovered {
            true => Srgba::new(1., 0., 0., 1.),
            false => Srgba::new(0., 1., 0., 1.),
        };

        if self.hovered {
            ctx.input().pointer.consume_hover();
        }

        ctx.add_shape(PaintRectangle {
            rect: self.rect,
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
