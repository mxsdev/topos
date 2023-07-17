use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    color::ColorRgba,
    element::Element,
    input::input_state::InputState,
    lib::Response,
    math::{Rect, Size},
    scene::{
        ctx::SceneContext,
        layout::{FlexBox, LayoutPass, LayoutPassResult},
    },
    shape::PaintRectangle,
};

pub struct TitleBarGlyph {
    glyph_size: f32,
    pub(super) response: Response<Rect>,
    pub(super) color: ColorRgba,
}

impl TitleBarGlyph {
    pub fn new(glyph_size: f32, color: ColorRgba) -> Self {
        Self {
            glyph_size,
            color,
            response: Response::new(Rect::zero()),
        }
    }

    pub fn set_size(&mut self, glyph_size: f32) {
        self.glyph_size = glyph_size
    }
}

impl Element for TitleBarGlyph {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        layout_pass
            .engine()
            .new_leaf(FlexBox::builder().size(Size::splat(self.glyph_size)))
            .unwrap()
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        ctx.add_shape(PaintRectangle::from_rect(rect).with_fill(self.color));
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        self.response.update_rect(input, rect)
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(AccessRole::ToggleButton)
    }
}
