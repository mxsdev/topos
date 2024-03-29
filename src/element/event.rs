use crate::math::{Pos, Vector};

#[derive(Debug)]
pub enum ElementEvent {
    CursorMove { pos: Pos, del: Option<Vector> },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
}

pub type MouseButton = winit::event::MouseButton;
