use cosmic_text::{Attrs, Family, Metrics};
use palette::{rgb::FromHexError, Darken, Desaturate, FromColor, Hsva, IntoColor};
use svg::node::element::Title;

use crate::{
    accessibility::{AccessNode, AccessNodeBuilder, AccessRole},
    color::{ColorRgba, ColorSrgba},
    element::{Element, ElementRef, SizeConstraint},
    input::input_state::InputState,
    lib::Response,
    scene::{
        ctx::SceneContext,
        layout::{FlexBox, LayoutPass, LayoutPassResult, Manual},
        scene::SceneResources,
    },
    shape::PaintRectangle,
    util::{Rect, Size},
};

use super::{TestRect, TextBox};

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
            .new_leaf(
                FlexBox::builder()
                    .size(Size::splat(self.glyph_size))
                    .to_taffy(),
            )
            .unwrap()
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        ctx.add_shape(PaintRectangle {
            rect: self.response.boundary.into(),
            fill: self.color.into(),

            ..Default::default()
        })
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        self.response.update_rect(input, rect)
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(AccessRole::ToggleButton)
    }
}
