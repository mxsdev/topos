use std::borrow::Cow;

use crate::{
    atlas::TextureAtlasManagerRef,
    input::output::{CursorIcon, PlatformOutput},
    math::{CoordinateTransform, Pos, Rect, Size, TransformationList, WindowScaleFactor},
    shape::{ClipRect, ClipRectList, ComputedPaintShape, PaintShape, ShaderClipRect},
};

use super::scene::SceneResources;

pub(super) struct PaintShapeWithContext {
    pub shape: ComputedPaintShape,
    pub clip_rect_idx: Option<u32>,
    pub transformation_idx: Option<u32>,
}

pub struct SceneContext<'a> {
    pub(super) shapes: Vec<PaintShapeWithContext>,
    pub(super) output: PlatformOutput,

    pub(super) transformations: TransformationList,
    pub(super) active_transformation_idx: Option<usize>,

    pub(super) clip_rects: ClipRectList,
    pub(super) active_clip_rect_idx: Option<usize>,

    scene_resources: SceneResources<'a>,

    scale_factor: WindowScaleFactor,
}

impl<'a> SceneContext<'a> {
    pub(super) fn new(
        scale_factor: WindowScaleFactor,
        transformations: TransformationList,
        clip_rects: ClipRectList,
        scene_resources: SceneResources<'a>,
    ) -> Self {
        Self {
            shapes: Default::default(),
            scale_factor,
            active_transformation_idx: Default::default(),
            transformations,
            output: Default::default(),
            clip_rects,
            active_clip_rect_idx: Default::default(),
            scene_resources,
        }
    }

    pub fn add_shape<'b, T: Into<PaintShape<'b>>>(&mut self, shape: T) {
        let scale_fac = self
            .active_transformation_idx
            // TODO: cache computation
            .map(|idx| {
                let (sx, sy) = self.transformations.get(idx).scale_factor();
                sx.max(sy)
            })
            .unwrap_or(1.)
            * self.scale_factor.get();

        let mut shape = Into::<PaintShape<'b>>::into(shape)
            .compute_paint_shape(self.current_clip_rect(), WindowScaleFactor::new(scale_fac));

        self.shapes.push(PaintShapeWithContext {
            shape,
            clip_rect_idx: self.active_clip_rect_idx.map(|x| x as u32),
            transformation_idx: self.active_transformation_idx.map(|x| x as u32),
        })
    }

    // pub fn push_clip_rect(&mut self, rect: impl Into<ClipRect>) {
    //     self.clip_rect_stack.push(self.clip_rects.len());
    //     self.clip_rects.push((
    //         rect.into(),
    //         self.active_transformation_idx.unwrap_or_default(),
    //     ));
    // }

    // pub fn pop_clip_rect(&mut self) {
    //     self.clip_rect_stack.pop().expect("Expected clip rect");
    // }

    pub fn current_clip_rect(&mut self) -> Option<ClipRect> {
        self.active_clip_rect_idx.map(|i| self.clip_rects.get(i).0)
    }

    // pub(crate) fn current_clip_rect_idx(&self) -> Option<usize> {
    //     self.clip_rect_stack.last().copied()
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

    pub fn resources(&mut self) -> &mut SceneResources<'a> {
        &mut self.scene_resources
    }
}
