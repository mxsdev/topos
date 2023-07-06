use crate::util::{Pos2, Vec2};
use winit::event::*;

#[derive(Debug)]
pub enum ElementEvent {
    CursorMove { pos: Pos2, del: Option<Vec2> },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
}

pub type MouseButton = winit::event::MouseButton;
