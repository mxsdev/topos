use cosmic_text::{Attrs, Family, Metrics};
use palette::{rgb::FromHexError, Darken, Desaturate, FromColor, Hsva, IntoColor};
use svg::node::element::Title;

use crate::{
    accessibility::{AccessNode, AccessNodeBuilder, AccessRole},
    color::{ColorRgba, ColorSrgba},
    element::{Element, ElementRef, SizeConstraint},
    input::input_state::InputState,
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
    shape::PaintRectangle,
    util::{FromMinSize, Pos2, Rect, Size2},
};

use super::{TestRect, TextBox};

#[derive(Default)]
pub struct TitleBarGlyph {
    glyph_size: f32,

    size: Size2,
    input_rect: Rect,

    pub(super) color: ColorRgba,

    pub(super) clicked: bool,
    pub(super) hovered: bool,
}

impl TitleBarGlyph {
    pub fn new(glyph_size: f32, color: ColorRgba) -> Self {
        Self {
            glyph_size,
            color,
            ..Default::default()
        }
    }

    pub fn set_size(&mut self, glyph_size: f32) {
        self.glyph_size = glyph_size
    }
}

impl Element for TitleBarGlyph {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2 {
        self.size = Size2::splat(self.glyph_size);
        self.size
    }

    fn ui(&mut self, ctx: &mut SceneContext, pos: Pos2) {
        ctx.add_shape(PaintRectangle {
            rect: self.input_rect.into(),
            fill: self.color.into(),

            ..Default::default()
        })
    }

    fn input(&mut self, input: &mut InputState, pos: Pos2) {
        self.input_rect = Rect::from_min_size(pos, self.size);

        self.hovered = false;

        if let Some(hover_pos) = input.pointer.hover_pos() {
            if self.input_rect.contains(hover_pos) {
                self.hovered = true;
                input.pointer.consume_hover();
            }
        }

        self.clicked = input.pointer.primary_clicked() && self.hovered;
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(AccessRole::ToggleButton)
    }
}
