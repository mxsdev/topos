use std::{
    cell::{RefCell, RefMut},
    ops::Deref,
    rc::Rc,
};

use euclid::{default, Translation2D};

use crate::{
    element::{Element, ElementRef, SizeConstraint},
    input::{
        input_state::InputState,
        output::{CursorIcon, PlatformOutput},
    },
    shape::PaintShape,
    util::{Rect, WindowScaleFactor},
};

pub struct SceneContext {
    pub(super) shapes: Vec<PaintShape>,
    pub(super) output: PlatformOutput,

    clip_rects: Vec<Option<Rect>>,

    scale_factor: WindowScaleFactor,
}

impl SceneContext {
    pub(super) fn new(scale_factor: WindowScaleFactor) -> Self {
        Self {
            shapes: Default::default(),
            clip_rects: Default::default(),
            scale_factor,
            output: Default::default(),
        }
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

    pub fn output(&mut self) -> &mut PlatformOutput {
        &mut self.output
    }

    pub fn set_cursor(&mut self, cursor_icon: CursorIcon) {
        self.output.set_cursor(cursor_icon)
    }

    pub fn start_window_drag(&mut self) {
        self.output.start_window_drag()
    }

    /// Open the given url in a web browser.
    /// If egui is running in a browser, the same tab will be reused.
    pub fn open_url(&mut self, url: impl ToString) {
        self.output.open_url(url)
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
