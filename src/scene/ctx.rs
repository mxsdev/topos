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
    util::{Pos2, Rect, Size2, Translate2DMut},
};

pub struct SceneContext {
    // internal: Rc<RefCell<SceneContextInternal>>,
    // input: Rc<RefCell<InputState>>,
    shapes: Vec<PaintShape>,
    clip_rects: Vec<Option<Rect>>,
    scale_factor: f32,
}

impl SceneContext {
    pub(super) fn new(scale_factor: f32) -> Self {
        Self {
            shapes: Default::default(),
            clip_rects: Default::default(),
            scale_factor,
        }
    }

    pub(super) fn drain(self) -> Vec<PaintShape> {
        self.shapes
    }

    pub fn add_shape(&mut self, shape: impl Into<PaintShape>) {
        self.shapes.push(shape.into())
    }

    pub fn push_clip_rect(&mut self, rect: impl Into<Option<Rect>>) {
        let rect: Option<Rect> = rect.into();

        let rect = rect
            .and_then(|x| Some(x.intersection_unchecked(&self.current_clip_rect()?)))
            .or(rect);

        self.add_shape(PaintShape::ClipRect(rect));
        self.clip_rects.push(rect);
    }

    pub fn pop_clip_rect(&mut self) {
        self.clip_rects.pop();

        self.add_shape(PaintShape::ClipRect(self.current_clip_rect()))
    }

    fn current_clip_rect(&self) -> Option<Rect> {
        self.clip_rects.last().copied().flatten()
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
