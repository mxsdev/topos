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

use super::{TestRect, TextBox};

pub struct TestRoot {
    rects: Vec<ElementRef<TestRect>>,
    text_box: ElementRef<TextBox>,

    clicked: bool,

    access_node: Arc<AccessNode>,

    scale_factor: f64,
}

impl RootConstructor for TestRoot {
    fn new(resources: &SceneResources) -> Self {
        let text_box = TextBox::new(
            resources,
            Metrics::new(20., 20.),
            ColorRgba::new(1., 1., 1., 1.),
            "Hello world".into(),
            Attrs::new().family(Family::Name("Test Calibre")),
        );

        Self {
            rects: vec![
                TestRect::new(Pos2::new(20., 20.)).into(),
                TestRect::new(Pos2::new(40., 40.)).into(),
                TestRect::new(Pos2::new(60., 60.)).into(),
            ],
            text_box: text_box.into(),

            clicked: false,

            access_node: AccessNodeBuilder::new(AccessRole::default()).build().into(),

            scale_factor: resources.scale_factor(),
        }
    }
}

impl Element for TestRoot {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2 {
        self.scale_factor = layout_pass.scale_factor();

        for rect in self.rects.iter_mut() {
            layout_pass.layout_and_place_child(rect, constraints, Pos2::zero());
        }

        layout_pass.layout_and_place_child(
            &mut self.text_box,
            SizeConstraint {
                min: Size2::zero(),
                max: Size2::new(100., 500.),
            },
            Pos2::zero(),
        );

        constraints.max
    }

    fn input(&mut self, input: &mut InputState, pos: Pos2) {
        self.clicked = input.pointer.primary_clicked();
    }

    fn ui(&mut self, ctx: &mut SceneContext, _pos: Pos2) {
        let mut send_to_back = None::<usize>;

        for (i, rect) in self.rects.iter_mut().enumerate() {
            if rect.get().clicked {
                send_to_back = Some(i);
            }
        }

        if let Some(idx) = send_to_back {
            let rect = self.rects.remove(idx);
            self.rects.push(rect);
        }
    }

    fn node(&self) -> AccessNodeBuilder {
        let mut builder = AccessNodeBuilder::new(AccessRole::Window);
        builder.set_transform(accesskit::Affine::scale(self.scale_factor));
        builder
    }
}
