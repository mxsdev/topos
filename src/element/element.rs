use crate::{paint::ScenePainter, util::Pos2};

use super::{
    boundary::{Boundary, EmptyBoundary},
    ElementEvent,
};

pub trait Element: Send {
    fn update(&mut self, event: &ElementEvent) -> bool;
    fn paint(&mut self, painter: &mut ScenePainter);

    // fn update_hover(&mut self, mouse_pos: &Pos2) -> bool {
    //     false
    // }

    // fn takes_focus(&self) -> bool {
    //     true
    // }

    // fn boundary(&self) -> &impl Boundary {
    //     &EmptyBoundary
    // }

    // fn on_hover_enter(&mut self) {}
    // fn on_hover_exit(&mut self) {}
}
