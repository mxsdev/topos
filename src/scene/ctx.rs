use std::{
    cell::{RefCell, RefMut},
    ops::Deref,
    rc::Rc,
};

use euclid::{default, Translation2D};

use crate::{
    element::{Element, ElementRef, SizeConstraint},
    input::input_state::InputState,
    shape::PaintShape,
    util::{Pos2, Size2, Translate2DMut},
};

pub struct SceneContext {
    // internal: Rc<RefCell<SceneContextInternal>>,
    // input: Rc<RefCell<InputState>>,
    shapes: Vec<PaintShape>,
    scale_factor: f32,
}

pub struct ChildUI {
    shapes: Vec<PaintShape>,
    pub size: Size2,
}

impl Clone for SceneContext {
    fn clone(&self) -> Self {
        Self {
            shapes: Default::default(),
            scale_factor: self.scale_factor,
        }
    }
}

impl SceneContext {
    fn new_inner(scale_factor: f32) -> Self {
        Self {
            shapes: Default::default(),
            scale_factor,
        }
    }

    pub(super) fn new(scale_factor: f32) -> Self {
        Self::new_inner(scale_factor)
    }

    pub(super) fn drain(self) -> Vec<PaintShape> {
        self.shapes
    }

    pub fn add_shape(&mut self, shape: impl Into<PaintShape>) {
        self.shapes.push(shape.into())
    }

    // pub fn add_buffer(&mut self, buffer: &cosmic_text::Buffer) {}

    // pub fn input(&mut self) -> RefMut<InputState> {
    //     self.input.borrow_mut()
    // }

    // pub fn render_child(&mut self, element: &mut ElementRef<impl Element>) {
    //     let mut ctx = self.clone();

    //     let scene_layout = self.scene_layout.borrow();
    //     let placement = scene_layout.get(&element.id());

    //     if let Some(pos) = placement {
    //         element.get().ui(&mut ctx, *pos);
    //         self.shapes.extend(ctx.shapes.into_iter());
    //     }
    // }
}
