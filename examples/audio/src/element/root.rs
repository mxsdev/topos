use topos::{
    accessibility::{AccessNodeBuilder, AccessRole},
    element::{Element, ElementRef, Response, RootConstructor},
    input::input_state::InputState,
    math::{DeviceScaleFactor, Rect},
    scene::{
        ctx::SceneContext,
        layout::{ColumnReverse, FlexBox, LayoutPass, LayoutPassResult, Percent},
        scene::SceneResources,
    },
    shape::ClipRect,
    util::layout::Center,
};

use super::wave::Wave;

pub struct TestRoot {
    scale_factor: DeviceScaleFactor,
    response: Response<Rect>,

    wave: ElementRef<Wave>,
}

impl RootConstructor for TestRoot {
    fn new(resources: &mut SceneResources) -> Self {
        Self {
            scale_factor: resources.device_scale_factor(),

            response: Default::default(),
            wave: super::wave::Wave::new().into(),
        }
    }
}

impl Element for TestRoot {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        layout_pass.layout_child(&mut self.wave);

        let result = layout_pass
            .engine()
            .new_leaf(
                FlexBox::builder()
                    .width(Percent(1.))
                    .height(Percent(1.))
                    .justify_content(Center)
                    .align_items(Center),
            )
            .unwrap();

        result
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        self.response.update_rect(input, rect);
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        if self.response.primary_clicked() {
            ctx.start_window_drag();
        }
    }

    fn ui_post(&mut self, _: &mut SceneContext, _rect: Rect) {}

    fn node(&self) -> AccessNodeBuilder {
        let mut builder = AccessNodeBuilder::new(AccessRole::Window);
        // TODO: make this part of topos
        builder.set_transform(topos::accesskit::Affine::scale(
            self.scale_factor.get().into_inner() as f64,
        ));
        builder
    }

    fn clip_rect(&self) -> Option<ClipRect> {
        None
    }
}
