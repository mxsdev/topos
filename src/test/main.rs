use cosmic_text::{Attrs, Family, Metrics};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    color::{ColorRgba, ColorSrgba},
    element::{Element, ElementRef, SizeConstraint},
    input::input_state::InputState,
    scene::{ctx::SceneContext, scene::SceneResources},
    shape::PaintRectangle,
    util::{FromMinSize, Pos2, Rect, Size2},
};

use super::{TestRect, TextBox};

pub struct MainElement {
    rects: Vec<ElementRef<TestRect>>,
    text_box: ElementRef<TextBox>,
    size: Size2,
}

impl MainElement {
    pub fn new(resources: &SceneResources) -> Self {
        let text_box = TextBox::new(
            resources,
            Metrics::new(20., 10.),
            ColorRgba::new(0., 0., 0., 1.),
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
            size: Default::default(),
        }
    }
}

impl Element for MainElement {
    fn layout(
        &mut self,
        constraints: crate::element::SizeConstraint,
        layout_pass: &mut crate::scene::layout::LayoutPass,
    ) -> crate::util::Size2 {
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

        self.size = constraints.max;
        self.size
    }

    fn ui(&mut self, ctx: &mut crate::scene::ctx::SceneContext, pos: Pos2) {
        ctx.add_shape(PaintRectangle {
            rect: Rect::from_min_size(pos, self.size).into(),
            fill: ColorSrgba::new(255, 254, 209, 255).into_linear().into(),
            ..Default::default()
        });

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
        AccessNodeBuilder::new(AccessRole::GenericContainer)
    }
}
