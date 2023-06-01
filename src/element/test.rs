use num_traits::Signed;
use palette::Srgba;

use crate::{
    shape::PaintRectangle,
    util::{Pos2, Rect, RoundedRect},
};

use super::{boundary::Boundary, Element, ElementEvent, MouseButton};

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
    fn update(&mut self, event: &ElementEvent) -> bool {
        match event {
            ElementEvent::CursorMove {
                pos: mouse_pos,
                del,
            } => {
                if let Some(del) = del {
                    if self.dragging {
                        self.rect.rect = self.rect.rect.translate(*del)
                    }
                }

                let inside = self.rect.sdf(&mouse_pos).is_positive();

                if self.hovered != inside {
                    log::trace!("hover changed: {:?}", inside)
                }

                self.hovered = inside;

                return inside;
            }

            ElementEvent::MouseDown {
                button: MouseButton::Left,
            } => {
                if self.hovered {
                    self.dragging = true;
                }
            }

            ElementEvent::MouseUp {
                button: MouseButton::Left,
            } => {
                self.dragging = false;
            }

            _ => {}
        }

        false
    }

    fn paint(&mut self, painter: &mut crate::paint::ScenePainter) {
        let fill = match self.hovered {
            true => Srgba::new(1., 0., 0., 1.),
            false => Srgba::new(0., 1., 0., 1.),
        };

        painter.add(PaintRectangle {
            rect: self.rect,
            fill: Some(fill),
            stroke_color: Some(Srgba::new(0., 0., 0., 1.)),
            stroke_width: Some(1.5),
        })
    }
}
