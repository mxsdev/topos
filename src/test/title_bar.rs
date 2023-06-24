use cosmic_text::{Attrs, Family, Metrics};
use keyframe::{functions::BezierCurve, mint::Vector2};
use palette::{rgb::FromHexError, Darken, Desaturate, FromColor, Hsva, IntoColor, WithAlpha};

use crate::{
    accessibility::{AccessNode, AccessNodeBuilder, AccessRole},
    color::{ColorRgb, ColorRgba, ColorSrgba},
    element::{transition::Transition, Element, ElementRef, GenericElement, SizeConstraint},
    input::input_state::InputState,
    scene::{
        ctx::SceneContext,
        layout::{AlignItems, CSSLayout, FlexBox, JustifyContent, LayoutPass, LayoutRect},
        scene::SceneResources,
    },
    shape::PaintRectangle,
    util::{AsRect, FromMinSize, IntoTaffy, Lerp, Pos2, Rect, Size2},
};

use super::{TestRect, TextBox, TitleBarGlyph};

#[derive(Default)]
pub struct TitleBar {
    size: Size2,
    input_rect: Rect,

    clicked: bool,

    selected_idx: Option<usize>,

    glyphs: Vec<(ElementRef<TitleBarGlyph>, Transition)>,
}

const INACTIVE_GLYPH_HEIGHT: f32 = 14.;
const ACTIVE_GLYPH_HEIGHT: f32 = 18.;

const GLYPH_COL: ColorRgb = ColorRgb::new(1., 1., 1.);

const TRANS_TIME: f32 = 0.125;

impl TitleBar {
    pub fn new() -> Self {
        let curve = BezierCurve::from(Vector2 { x: 0.62, y: 0. }, Vector2 { x: 0.43, y: 0.98 });

        let glyphs = (0..4)
            .map(|_| TitleBarGlyph::new(INACTIVE_GLYPH_HEIGHT, GLYPH_COL.with_alpha(1.)).into())
            .map(|el| (el, Transition::new(TRANS_TIME).set_ease_func(curve)))
            .collect();

        Self {
            glyphs,
            ..Default::default()
        }
    }
}

impl Element for TitleBar {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2 {
        let main = FlexBox::default()
            .size(constraints.max)
            .padding(LayoutRect::x(5.))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .gap(8.)
            .place_children(
                constraints,
                layout_pass,
                self.glyphs.iter_mut().map(|x| &mut x.0),
            );

        self.size = main;
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

    fn ui_post(&mut self, ctx: &mut SceneContext, pos: Pos2) {
        for child in self.glyphs.iter_mut() {
            let trans = &mut child.1;
            let mut child = child.0.get();

            // TODO: set this only once...
            if child.hovered {
                trans.set_state(true);
            } else {
                trans.set_state(false);
            }

            // TODO: only change when transition changes
            child.set_size(INACTIVE_GLYPH_HEIGHT.lerp(ACTIVE_GLYPH_HEIGHT, trans.fac()));
            child.color = GLYPH_COL.with_alpha(Lerp::lerp(0.5, 1., trans.fac()));
        }
    }

    fn input(&mut self, input: &mut InputState, pos: Pos2) {
        for (_, trans) in self.glyphs.iter_mut() {
            trans.update(input)
        }

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
