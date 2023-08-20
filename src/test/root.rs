use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    element::{Element, ElementRef, RootConstructor},
    input::input_state::InputState,
    math::{Pos, Rect, Size, WindowScaleFactor},
    scene::{
        ctx::SceneContext,
        layout::{ColumnReverse, FlexBox, LayoutPass, LayoutPassResult, Percent},
        scene::SceneResources,
    },
    shape::ClipRect,
};

use super::{MainElement, TitleBar};

pub struct TestRoot {
    scale_factor: WindowScaleFactor,

    main: ElementRef<MainElement>,
    title_bar: ElementRef<TitleBar>,
}

impl RootConstructor for TestRoot {
    fn new(resources: &mut SceneResources) -> Self {
        Self {
            scale_factor: resources.scale_factor(),

            main: MainElement::new(resources).into(),
            title_bar: TitleBar::new(resources, 27.).into(),
        }
    }
}

impl Element for TestRoot {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        layout_pass.layout_child(&mut self.main);
        layout_pass.layout_child(&mut self.title_bar);

        let result = layout_pass
            .engine()
            .new_leaf(
                FlexBox::builder()
                    .direction(ColumnReverse)
                    .width(Percent(1.))
                    .height(Percent(1.)),
            )
            .unwrap();

        result
    }

    fn input(&mut self, _: &mut InputState, _: Rect) {}

    fn ui(&mut self, ctx: &mut SceneContext, _: Rect) {
        ctx.push_clip_rect(
            ClipRect::from(Rect::from_min_size(Pos::zero(), Size::new(500., 312.)))
                .with_radius(Some(10.)),
        );
    }

    fn ui_post(&mut self, ctx: &mut SceneContext, _rect: Rect) {
        ctx.pop_clip_rect()
    }

    fn node(&self) -> AccessNodeBuilder {
        let mut builder = AccessNodeBuilder::new(AccessRole::Window);
        builder.set_transform(accesskit::Affine::scale(self.scale_factor.get() as f64));
        builder
    }
}
