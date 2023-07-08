use cosmic_text::{Attrs, Family, Metrics};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    color::{ColorRgba, ColorSrgba},
    element::{Element, ElementRef},
    scene::{
        layout::{FlexBox, LayoutPass, LayoutPassResult, Percent},
        scene::SceneResources,
    },
    shape::PaintRectangle,
    util::{Pos, Rect, RoundedRect},
};

use super::{TestRect, TextBox};

pub struct MainElement {
    rects: Vec<ElementRef<TestRect>>,
    text_box: ElementRef<TextBox>,
}

impl MainElement {
    pub fn new(resources: &mut SceneResources) -> Self {
        let text_box = TextBox::new(
            resources,
            Metrics::new(20., 20.),
            ColorRgba::new(0., 0., 0., 1.),
            "Hello world".into(),
            Attrs::new().family(Family::Name("Test Calibre")),
        );

        Self {
            rects: vec![
                TestRect::new(Pos::new(20., 20.)).into(),
                TestRect::new(Pos::new(40., 40.)).into(),
                TestRect::new(Pos::new(60., 60.)).into(),
            ],
            text_box: text_box.into(),
        }
    }
}

impl Element for MainElement {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        for rect in self.rects.iter_mut() {
            layout_pass.layout_child(rect);
        }

        layout_pass.layout_child(&mut self.text_box);

        layout_pass
            .engine()
            .new_leaf(
                FlexBox::builder()
                    .width(Percent(1.))
                    .flex_grow(1.)
                    .to_taffy(),
            )
            .unwrap()
    }

    fn ui(&mut self, ctx: &mut crate::scene::ctx::SceneContext, rect: Rect) {
        ctx.add_shape(PaintRectangle {
            rect: rect.into(),
            fill: ColorSrgba::new(255, 254, 209, 255).into_linear().into(),
            ..Default::default()
        });

        let mut send_to_back = None::<usize>;

        for (i, rect) in self.rects.iter_mut().enumerate() {
            if rect.get().response.just_focused() {
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
