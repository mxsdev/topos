use topos::{
    accessibility::{AccessNodeBuilder, AccessRole},
    element::{Element, Response, RootConstructor},
    input::input_state::InputState,
    math::{DeviceScaleFactor, Rect},
    scene::{
        ctx::SceneContext,
        layout::{ColumnReverse, FlexBox, LayoutPass, LayoutPassResult, Percent},
        scene::SceneResources,
    },
    shape::ClipRect,
};

pub struct TestRoot {
    scale_factor: DeviceScaleFactor,

    response: Response<Rect>,
}

impl RootConstructor for TestRoot {
    fn new(resources: &mut SceneResources) -> Self {
        Self {
            scale_factor: resources.device_scale_factor(),

            response: Default::default(),
        }
    }
}

impl Element for TestRoot {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
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
