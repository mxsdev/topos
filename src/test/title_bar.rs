use cosmic_text::{Attrs, Family, Metrics};
use palette::{rgb::FromHexError, Darken, Desaturate, FromColor, Hsva, IntoColor};

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
pub struct TitleBar {
    size: Size2,
    input_rect: Rect,

    clicked: bool,

    selected_idx: Option<usize>,
}

impl TitleBar {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Element for TitleBar {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2 {
        const INACTIVE_GLYPH_HEIGHT: f32 = 14.;
        const ACTIVE_GLYPH_HEIGHT: f32 = 18.;

        // let header_inner = layout_pass
        //     .engine()
        //     .new_leaf(taffy::style::Style {
        //         ..Default::default()
        //     })
        //     .unwrap();

        self.size = constraints.max;
        self.size
    }

    fn ui(&mut self, ctx: &mut SceneContext, pos: Pos2) {
        if self.clicked {
            ctx.start_window_drag()
        }

        let col: ColorRgba = ColorSrgba::<u8>::new(178, 132, 190, 255).into_linear();

        let col2: ColorRgba = Hsva::from_color(col)
            .desaturate(0.3)
            .darken(0.9)
            .into_color();

        ctx.add_shape(PaintRectangle {
            rect: self.input_rect.into(),
            fill: col2.into(),
            ..Default::default()
        });
    }

    fn input(&mut self, input: &mut InputState, pos: Pos2) {
        self.input_rect = Rect::from_min_size(pos, self.size);

        if let Some(hover_pos) = input.pointer.hover_pos() {
            if self.input_rect.contains(hover_pos) {
                input.pointer.consume_hover();
                self.clicked = input.pointer.primary_clicked();
            }
        }
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(AccessRole::TitleBar)
    }
}
