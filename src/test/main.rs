use cosmic_text::{Attrs, Family, Metrics};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    color::{ColorRgba, ColorSrgba, FromNSColor},
    element::{Element, ElementRef},
    input::input_state::InputState,
    lib::Response,
    math::{Angle, CoordinateTransform, Pos, Rect, Vector},
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

    scale_fac: f32,
    delta: Vector,
}

impl MainElement {
    pub fn new(resources: &mut SceneResources) -> Self {
        let mut color = ColorRgba::new(1., 1., 1., 1.);

        #[cfg(target_os = "macos")]
        {
            use icrate::AppKit::NSColor;
            color = ColorSrgba::from_ns_color(unsafe { NSColor::textColor() }.as_ref()).into();
        }

        color.alpha = 0.3;

        let text_box = TextBoxElement::new(
            resources,
            Metrics::new(20., 20.),
            color,
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
            scale_fac: 1.,
            delta: Vector::zero(),
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

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        if let Some(pos) = input.pointer.latest_pos() {
            let scroll_del = (input.scroll_delta.y * 0.01).exp();

            let old_scale_fac = self.scale_fac;
            self.scale_fac *= scroll_del;

            let f = old_scale_fac - self.scale_fac;

            self.delta += Vector::new(pos.x * f, pos.y * f);
        }
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(AccessRole::GenericContainer)
    }

    fn coordinate_transform(&self) -> Option<CoordinateTransform> {
        CoordinateTransform::scale(self.scale_fac, self.scale_fac)
            .then_translate(self.delta)
            .into()
    }
}
