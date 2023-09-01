use std::borrow::Cow;

use crate::{
    input::output::{CursorIcon, PlatformOutput},
    math::{CoordinateTransform, Pos, Rect, Size, TransformationList, WindowScaleFactor},
    shape::{ClipRect, PaintShape, ShaderClipRect},
};

pub(super) struct PaintShapeWithContext {
    pub shape: PaintShape,
    pub clip_rect_idx: Option<u32>,
    pub transformation_idx: Option<u32>,
}

pub struct SceneContext {
    pub(super) shapes: Vec<PaintShapeWithContext>,
    pub(super) output: PlatformOutput,

    pub(super) clip_rects: Vec<(ClipRect, usize)>,
    clip_rect_stack: Vec<usize>,

    pub(super) transformations: TransformationList,
    pub(super) active_transformation_idx: Option<usize>,

    scale_factor: WindowScaleFactor,
}

impl SceneContext {
    pub(super) fn new(
        scale_factor: WindowScaleFactor,
        transformations: TransformationList,
    ) -> Self {
        Self {
            shapes: Default::default(),
            clip_rects: Vec::from([Default::default()]),
            clip_rect_stack: Default::default(),
            scale_factor,
            active_transformation_idx: Default::default(),
            transformations,
            output: Default::default(),
        }
    }

    pub fn add_shape(&mut self, shape: impl Into<PaintShape>) {
        let mut shape = shape.into();

        match &mut shape {
            PaintShape::Text(text) => text.clip_rect = self.current_clip_rect(),
            _ => {}
        }

        self.shapes.push(PaintShapeWithContext {
            shape,
            clip_rect_idx: self.current_clip_rect_idx().map(|x| x as u32),
            transformation_idx: self.active_transformation_idx.map(|x| x as u32),
        })
    }

    pub fn push_clip_rect(&mut self, rect: impl Into<ClipRect>) {
        self.clip_rect_stack.push(self.clip_rects.len());
        self.clip_rects.push((
            rect.into(),
            self.active_transformation_idx.unwrap_or_default(),
        ));
    }

    pub fn pop_clip_rect(&mut self) {
        self.clip_rect_stack.pop().expect("Expected clip rect");
    }

    pub fn current_clip_rect(&self) -> Option<ClipRect> {
        self.current_clip_rect_idx().map(|i| self.clip_rects[i].0)
    }

    pub(crate) fn current_clip_rect_idx(&self) -> Option<usize> {
        self.clip_rect_stack.last().copied()
    }

    // pub fn push_transformation(&mut self, transformation: impl Into<CoordinateTransform>) {
    //     let transformation = transformation.into();
    //     let new_idx = self.transformations.len();

    //     self.transformations.push(
    //         self.current_transformation()
    //             .map(|t| t.then(&transformation))
    //             .unwrap_or(transformation),
    //     );
    //     self.transformation_stack.push(new_idx);
    // }

    // pub(crate) fn push_transformation_idx(&mut self, idx: usize) {
    //     self.transformation_stack.push(idx);
    // }

    // // pub fn push_transformation_non_cascading(
    // //     &mut self,
    // //     transformation: impl Into<CoordinateTransform>,
    // // ) {
    // //     self.transformation_stack.push(self.transformations.len());
    // //     self.transformations.push(transformation.into());
    // // }

    // pub fn pop_transformation(&mut self) {
    //     self.transformation_stack
    //         .pop()
    //         .expect("Expected transformation");
    // }

    // pub fn current_transformation(&self) -> Option<CoordinateTransform> {
    //     self.current_transformation_idx()
    //         .map(|i| self.transformations[i])
    // }

    // pub(crate) fn current_transformation_idx(&self) -> Option<usize> {
    //     self.transformation_stack.last().copied()
    // }

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
}
