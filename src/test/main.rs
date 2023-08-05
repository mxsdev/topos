use cosmic_text::{Attrs, Family, Metrics};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    color::{ColorRgba, ColorSrgba},
    element::{Element, ElementRef},
    math::{Pos, Rect, Size},
    scene::{
        layout::{FlexBox, LayoutPass, LayoutPassResult, Percent},
        scene::SceneResources,
    },
    shape::PaintRectangle,
};

use super::{TestRect, TextBoxElement};

pub struct MainElement {
    rects: Vec<ElementRef<TestRect>>,
    text_box: ElementRef<TextBoxElement>,
}

impl MainElement {
    pub fn new(resources: &mut SceneResources) -> Self {
        let text_box = TextBoxElement::new(
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
            .new_leaf(FlexBox::builder().width(Percent(1.)).flex_grow(1.))
            .unwrap()
    }

    fn ui(&mut self, ctx: &mut crate::scene::ctx::SceneContext, rect: Rect) {
        ctx.add_shape(
            PaintRectangle::from_rect(rect).with_fill(ColorSrgba::new(255, 254, 209, 255)),
        );

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
