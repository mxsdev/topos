use topos::{
    accessibility::{AccessNodeBuilder, AccessRole},
    element::{Element, ElementRef, Response, RootConstructor},
    input::input_state::InputState,
    math::{DeviceScaleFactor, Rect, Size},
    scene::{
        ctx::SceneContext,
        layout::{ColumnReverse, FlexBox, LayoutPass, LayoutPassResult, Percent},
        scene::SceneResources,
    },
    shape::{ClipRect, PaintRectangle},
};

use super::{MainElement, TitleBar};

pub struct TestRoot {
    scale_factor: DeviceScaleFactor,

    main: ElementRef<MainElement>,
    title_bar: ElementRef<TitleBar>,

    response: Response<Rect>,
}

impl RootConstructor for TestRoot {
    fn new(resources: &mut SceneResources) -> Self {
        Self {
            scale_factor: resources.device_scale_factor(),

            main: MainElement::new(resources).into(),
            title_bar: TitleBar::new(resources, 27.).into(),

            response: Default::default(),
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

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        self.response.update_rect(input, rect);
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        if self.response.primary_clicked() {
            ctx.start_window_drag();
        }

        // ctx.add_shape(
        //     PaintRectangle::from_rect(rect).with_fill(ColorSrgba::new(255, 254, 209, 255)),
        // );

        // ctx.push_clip_rect(
        //     ClipRect::from(Rect::from_min_size(Pos::zero(), Size::new(500., 312.)))
        //         .with_radius(Some(10.)),
        // );

        // ctx.push_transformation(CoordinateTransform::rotation(Angle::degrees(20.)));

        // const SCALE_FAC: f32 = 3.;
        // ctx.push_transformation(CoordinateTransform::scale(SCALE_FAC, SCALE_FAC));
    }

    fn ui_post(&mut self, ctx: &mut SceneContext, _rect: Rect) {
        // ctx.pop_clip_rect();
        // ctx.pop_transformation();
    }

    fn node(&self) -> AccessNodeBuilder {
        let mut builder = AccessNodeBuilder::new(AccessRole::Window);
        // TODO: make this part of topos
        builder.set_transform(topos::accesskit::Affine::scale(
            self.scale_factor.get().into_inner() as f64,
        ));
        builder
    }

    // fn coordinate_transform(&self) -> Option<CoordinateTransform> {
    //     CoordinateTransform::identity().into()
    // }

    fn clip_rect(&self) -> Option<ClipRect> {
        // ClipRect::from(Rect::from_min_size(Pos::zero(), Size::new(500., 312.)))
        //     .with_radius(Some(10.))
        //     .into()

        None
    }
}
