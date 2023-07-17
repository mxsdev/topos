use cosmic_text::{Attrs, Family, Metrics};
use keyframe::{functions::BezierCurve, mint::Vector2};
use palette::{rgb::FromHexError, Darken, Desaturate, FromColor, Hsva, IntoColor, WithAlpha};

use crate::{
    accessibility::{AccessNode, AccessNodeBuilder, AccessRole},
    color::{ColorRgb, ColorRgba, ColorSrgba},
    element::{transition::Transition, Element, ElementRef, SizeConstraint},
    input::input_state::InputState,
    math::Rect,
    scene::{
        ctx::SceneContext,
        layout::{
            AlignItems, Center, FlexBox, JustifyContent, LayoutPass, LayoutPassResult, LayoutRect,
            Percent,
        },
        scene::SceneResources,
    },
    shape::PaintRectangle,
    util::Lerp,
};

use super::{TestRect, TextBoxElement, TitleBarGlyph};

pub struct TitleBar {
    input_rect: Rect,

    clicked: bool,

    selected_idx: Option<usize>,

    glyphs: Vec<(ElementRef<TitleBarGlyph>, Transition)>,

    layout_node: LayoutPassResult,

    height: f32,
}

const INACTIVE_GLYPH_HEIGHT: f32 = 14.;
const ACTIVE_GLYPH_HEIGHT: f32 = 18.;

const GLYPH_COL: ColorRgb = ColorRgb::new(1., 1., 1.);

const TRANS_TIME: f32 = 0.125;

impl TitleBar {
    pub fn new(resources: &mut SceneResources, height: f32) -> Self {
        let curve = BezierCurve::from(Vector2 { x: 0.62, y: 0. }, Vector2 { x: 0.43, y: 0.98 });

        let layout_node = resources
            .layout_engine()
            .new_leaf(
                FlexBox::builder()
                    .width(Percent(1.))
                    .height(height)
                    .padding_x(5.)
                    .align_items(Center)
                    .justify_content(Center)
                    .gap(8.),
            )
            .unwrap();

        let glyphs = (0..4)
            .map(|_| TitleBarGlyph::new(INACTIVE_GLYPH_HEIGHT, GLYPH_COL.with_alpha(1.)).into())
            .map(|el| (el, Transition::new(TRANS_TIME).set_ease_func(curve)))
            .collect();

        Self {
            glyphs,
            height,
            layout_node,

            clicked: Default::default(),
            input_rect: Default::default(),
            selected_idx: Default::default(),
        }
    }
}

impl Element for TitleBar {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        for (child, _) in self.glyphs.iter_mut() {
            layout_pass.layout_child(child)
        }

        self.layout_node.clone()
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        if self.clicked {
            ctx.start_window_drag()
        }

        let col: ColorRgba = ColorSrgba::<u8>::new(178, 132, 190, 255).into_linear();

        let col2: ColorRgba = Hsva::from_color(col)
            .desaturate(0.3)
            .darken(0.9)
            .into_color();

        ctx.add_shape(PaintRectangle::from_rect(self.input_rect).with_fill(col2));
    }

    fn ui_post(&mut self, ctx: &mut SceneContext, rect: Rect) {
        for child in self.glyphs.iter_mut() {
            let trans = &mut child.1;
            let mut child = child.0.get();

            // TODO: set this only once...
            if child.response.hovered() {
                trans.set_state(true);
            } else {
                trans.set_state(false);
            }

            // TODO: only change when transition changes
            child.set_size(INACTIVE_GLYPH_HEIGHT.lerp(ACTIVE_GLYPH_HEIGHT, trans.fac()));
            child.color = GLYPH_COL.with_alpha(Lerp::lerp(0.5, 1., trans.fac()));
        }
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        for (_, trans) in self.glyphs.iter_mut() {
            trans.update(input)
        }

        self.input_rect = rect;

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
