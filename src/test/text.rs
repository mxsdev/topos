use crate::color::ColorRgba;

use cosmic_text::{Attrs, FontSystem, Metrics};

use crate::{
    atlas::PlacedTextBox,
    element::{Element, SizeConstraint},
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
    util::{FromMinSize, LogicalToPhysical, Pos2, Rect, Size2, Vec2},
};

pub struct TextBox {
    buffer: cosmic_text::Buffer,

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
            metrics.scale(scene_resources.scale_factor()),
        );

        buffer.set_text(&mut font_system, &text, attrs);

        Self {
            attrs,
            text,
            buffer,
            color,
            logical_metrics: metrics,
        }
    }
}

impl Element for TextBox {
    fn layout(&mut self, constraints: SizeConstraint, layout_pass: &mut LayoutPass) -> Size2 {
        let size = constraints.max;

        let scale_factor = layout_pass.scale_factor();
        let new_metrics = self.logical_metrics.scale(scale_factor);

        let mut font_system = layout_pass.font_system();

        self.buffer.set_metrics(&mut font_system, new_metrics);

        {
            let physical_size = size.to_physical(scale_factor);

            self.buffer
                .set_size(&mut font_system, physical_size.width, physical_size.height)
        }

        self.buffer.shape_until_scroll(&mut font_system);

        size
    }

    fn ui(&mut self, ctx: &mut SceneContext, pos: Pos2) {
        ctx.add_shape(PlacedTextBox::from_buffer(&self.buffer, pos, self.color))
    }
}
