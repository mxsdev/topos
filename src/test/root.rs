use std::sync::Arc;

use cosmic_text::{Attrs, Family, Metrics, Style, Weight};

use crate::{
    accessibility::{AccessNode, AccessNodeBuilder, AccessRole},
    color::ColorRgba,
    element::{Element, ElementRef, RootConstructor, SizeConstraint},
    input::input_state::InputState,
    scene::{
        ctx::SceneContext,
        layout::{Column, FlexBox, FlexDirection, LayoutPass, LayoutPassResult, Percent},
        scene::SceneResources,
    },
    util::{FromMinSize, Pos2, Rect, Size2},
};

use super::{MainElement, TestRect, TextBox, TitleBar};

pub struct TestRoot {
    scale_factor: f64,

    main: ElementRef<MainElement>,
    title_bar: ElementRef<TitleBar>,
}

impl RootConstructor for TestRoot {
    fn new(resources: &mut SceneResources) -> Self {
        Self {
            scale_factor: resources.scale_factor(),

            main: MainElement::new(resources).into(),
            title_bar: TitleBar::new(27.).into(),
        }
    }
}

impl Element for TestRoot {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        layout_pass.layout_child(&mut self.title_bar);
        layout_pass.layout_child(&mut self.main);

        layout_pass
            .engine()
            .new_leaf(
                FlexBox::builder()
                    .direction(Column)
                    .width(Percent(1.))
                    .height(Percent(1.))
                    .to_taffy(),
            )
            .unwrap()
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {}

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {}

    fn node(&self) -> AccessNodeBuilder {
        let mut builder = AccessNodeBuilder::new(AccessRole::Window);
        builder.set_transform(accesskit::Affine::scale(self.scale_factor));
        builder
    }
}
