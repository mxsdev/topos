use cosmic_text::{Attrs, Metrics, Style};
use palette::Srgba;

use crate::{
    element::{Element, ElementRef, RootConstructor, SizeConstraint},
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
    util::{Pos2, Size2},
};

use super::{TestRect, TextBox};

pub struct TestRoot {
    rects: Vec<ElementRef<TestRect>>,
    text_box: ElementRef<TextBox>,
}

impl RootConstructor for TestRoot {
    fn new(resources: &SceneResources) -> Self {
        let attrs = Attrs::new();

        // attrs.family(Family::Name("SF Pro"));
        attrs.style(Style::Italic);

        let text_box = TextBox::new(
            resources,
            Metrics::new(30., 40.),
            Srgba::new(1., 1., 1., 1.),
            "Hello world!".into(),
            attrs,
        );

        Self {
            rects: vec![
                TestRect::new(Pos2::new(20., 20.)).into(),
                TestRect::new(Pos2::new(40., 40.)).into(),
                TestRect::new(Pos2::new(60., 60.)).into(),
            ],
            text_box: text_box.into(),
        }
    }
}

impl Element for TestRoot {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2 {
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

    fn ui(&mut self, _ctx: &mut SceneContext, _pos: Pos2) {
        let mut send_to_back = None::<usize>;

        // ctx.render_child(&mut self.text_box);

        for (i, rect) in self.rects.iter_mut().enumerate() {
            // ctx.render_child(rect);

            if rect.get().clicked {
                send_to_back = Some(i);
            }
        }

        if let Some(idx) = send_to_back {
            let rect = self.rects.remove(idx);
            self.rects.push(rect);
        }
    }
}