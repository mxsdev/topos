use std::sync::Arc;

use cosmic_text::{Attrs, Family, Metrics, Style, Weight};

use crate::{
    accessibility::{AccessNode, AccessNodeBuilder, AccessRole},
    color::ColorRgba,
    element::{Element, ElementRef, RootConstructor, SizeConstraint},
    input::input_state::InputState,
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
    util::{FromMinSize, Pos2, Rect, Size2},
};

use super::{MainElement, TestRect, TextBox, TitleBar};

pub struct TestRoot {
    scale_factor: f64,

    main: ElementRef<MainElement>,
    title_bar: ElementRef<TitleBar>,
}

impl RootConstructor for TestRoot {
    fn new(resources: &SceneResources) -> Self {
        Self {
            scale_factor: resources.scale_factor(),

            main: MainElement::new(resources).into(),
            title_bar: TitleBar::new().into(),
        }
    }
}

impl Element for TestRoot {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2 {
        let titlebar_size = Size2::new(constraints.max.width, 27.);
        let main_size = Size2::new(
            constraints.max.width,
            constraints.max.height - titlebar_size.height,
        );

        layout_pass.layout_and_place_child(
            &mut self.main,
            main_size,
            Pos2::new(0., titlebar_size.height),
        );

        layout_pass.layout_and_place_child(&mut self.title_bar, titlebar_size, Pos2::zero());

        constraints.max
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {}

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {}

    fn node(&self) -> AccessNodeBuilder {
        let mut builder = AccessNodeBuilder::new(AccessRole::Window);
        builder.set_transform(accesskit::Affine::scale(self.scale_factor));
        builder
    }
}
