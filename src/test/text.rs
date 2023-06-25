use std::sync::{Arc, Mutex};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    color::ColorRgba,
    scene::layout::{FlexBox, LayoutPassResult},
};

use cosmic_text::{Attrs, FontSystem, Metrics};
use taffy::style::AvailableSpace;

use crate::{
    atlas::PlacedTextBox,
    element::Element,
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
    util::{IntoTaffy, Rect, Size2},
};

pub struct TextBox {
    buffer: Arc<Mutex<cosmic_text::Buffer>>,

    logical_metrics: Metrics,
    color: ColorRgba,

    text: String,
    attrs: Attrs<'static>,
}

impl TextBox {
    pub fn new(
        scene_resources: &SceneResources,
        metrics: Metrics,
        color: ColorRgba,
        text: String,
        attrs: Attrs<'static>,
    ) -> Self {
        let mut font_system = scene_resources.font_system();

        let mut buffer = cosmic_text::Buffer::new(
            &mut font_system,
            metrics.scale(scene_resources.scale_factor_f32()),
        );

        buffer.set_text(&mut font_system, &text, attrs);

        Self {
            attrs,
            text,
            buffer: Arc::new(buffer.into()),
            color,
            logical_metrics: metrics,
        }
    }
}

struct MeasureTextBox {
    font_system: Arc<Mutex<FontSystem>>,
    buffer: Arc<Mutex<cosmic_text::Buffer>>,
    scale_factor: f32,
}

impl
    FnOnce<(
        taffy::prelude::Size<Option<f32>>,
        taffy::prelude::Size<AvailableSpace>,
    )> for MeasureTextBox
{
    type Output = taffy::prelude::Size<f32>;

    extern "rust-call" fn call_once(
        self,
        args: (
            taffy::prelude::Size<Option<f32>>,
            taffy::prelude::Size<AvailableSpace>,
        ),
    ) -> Self::Output {
        self.call(args)
    }
}

impl
    FnMut<(
        taffy::prelude::Size<Option<f32>>,
        taffy::prelude::Size<AvailableSpace>,
    )> for MeasureTextBox
{
    extern "rust-call" fn call_mut(
        &mut self,
        args: (
            taffy::prelude::Size<Option<f32>>,
            taffy::prelude::Size<AvailableSpace>,
        ),
    ) -> Self::Output {
        self.call(args)
    }
}

// TODO: upstream PR to make this less disgusting
impl
    Fn<(
        taffy::prelude::Size<Option<f32>>,
        taffy::prelude::Size<AvailableSpace>,
    )> for MeasureTextBox
{
    extern "rust-call" fn call(
        &self,
        (
            taffy::geometry::Size { width, height },
            taffy::geometry::Size {
                width: available_width,
                height: available_height,
            },
        ): (
            taffy::prelude::Size<Option<f32>>,
            taffy::prelude::Size<AvailableSpace>,
        ),
    ) -> Self::Output {
        let tbox_width = width.unwrap_or(match available_width {
            taffy::style::AvailableSpace::Definite(max_width) => max_width,
            taffy::style::AvailableSpace::MinContent => 0.,
            taffy::style::AvailableSpace::MaxContent => f32::INFINITY,
        });

        let tbox_height = height.unwrap_or(match available_height {
            taffy::style::AvailableSpace::Definite(max_height) => max_height,
            taffy::style::AvailableSpace::MinContent => 0.,
            taffy::style::AvailableSpace::MaxContent => f32::INFINITY,
        });

        let mut buffer = self.buffer.lock().unwrap();

        buffer.set_size(
            &mut self.font_system.lock().unwrap(),
            tbox_width * self.scale_factor,
            tbox_height * self.scale_factor,
        );

        let lh = buffer.metrics().line_height;

        let size = buffer
            .layout_runs()
            .fold(Size2::new(0.0, 0.0), |mut size, run| {
                let new_width = run.line_w;
                if new_width > size.width {
                    size.width = new_width;
                }

                size.height += lh;

                size
            });

        size.into_taffy()
    }
}

impl Element for TextBox {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        let scale_factor = layout_pass.scale_factor();
        let new_metrics = self.logical_metrics.scale(scale_factor as f32);

        {
            let mut buffer = self.buffer.lock().unwrap();

            let mut font_system = layout_pass.font_system();

            buffer.set_metrics(&mut font_system, new_metrics);

            buffer.shape_until_scroll(&mut font_system);
        }

        let scale_factor = layout_pass.scene_resources().scale_factor_f32();
        let font_system = layout_pass.font_system_ref();

        layout_pass
            .engine()
            .new_leaf_with_measure(
                FlexBox::builder().to_taffy(),
                taffy::node::MeasureFunc::Boxed(Box::new(MeasureTextBox {
                    buffer: self.buffer.clone(),
                    font_system,
                    scale_factor,
                })),
            )
            .unwrap()
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        ctx.add_shape(PlacedTextBox::from_buffer(
            &self.buffer.lock().unwrap(),
            rect.min,
            self.color,
        ))
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(AccessRole::StaticText)
    }
}
