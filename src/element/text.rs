use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    atlas::AtlasAllocation,
    color::ColorRgba,
    math::{PhysicalSize, Pos},
    scene::layout::{AvailableSpace, FlexBox, LayoutPassResult},
    shape::PaintFill,
    util::{
        guard::ReadLockable, layout::{LayoutStyle, TaffyNodeContext}, text::{AtlasContentType, CachedFloat, FontSystemRef, TextBox, TextCacheBuffer}
    },
};

use crate::util::text::{Attrs, Metrics};

use crate::{
    element::Element,
    math::{Rect, Size},
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
};

pub struct TextBoxElement {
    buffer: Arc<Mutex<TextCacheBuffer>>,

    layout_node: LayoutPassResult,

    logical_metrics: Metrics,

    text: String,
    attrs: Attrs<'static>,

    image: AtlasAllocation,
}

impl TextBoxElement {
    pub fn new(
        scene_resources: &mut SceneResources,
        metrics: Metrics,
        color: impl Into<PaintFill>,
        text: String,
        attrs: Attrs<'static>,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let image_allocation = {
            let mut atlas_manager = scene_resources.texture_atlas_manager().write().unwrap();

            let s = PhysicalSize::new(2, 1);

            let image_allocation = atlas_manager
                .allocate(
                    scene_resources.texture_manager(),
                    AtlasContentType::Color,
                    s,
                )
                .unwrap();

            atlas_manager.get_atlas(&image_allocation).write_texture(
                &scene_resources.rendering_context_ref(),
                &image_allocation,
                &[0xFF, 0xEC, 0xD2, 0xFF, 0xFC, 0xB6, 0x9F, 0xFF],
                // &[0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            );

            image_allocation
        };

        let buffer = {
            let mut font_system = scene_resources.font_system();

            let mut buffer = TextBox::new(
                &mut font_system,
                metrics.font_size,
                metrics.line_height,
                color,
                // PaintFill::from_atlas_allocation_uv(
                //     &image_allocation,
                //     Rect::new(Pos::new(0.5, 0.5), Pos::new(1.5, 0.5)),
                // ),
                Pos::default(),
            );

            buffer.set_text(&mut font_system, &text, &attrs);

            buffer.shape_until_scroll(&mut font_system);

            buffer
        };

        let buffer = Arc::new(Mutex::new(TextCacheBuffer::from(buffer)));

        let layout_node = scene_resources
            .layout_engine()
            .new_leaf_with_context(
                layout,
                TaffyNodeContext::Text(buffer.clone()),
            )
            .unwrap();

        Self {
            attrs,
            text,
            buffer,
            logical_metrics: metrics,
            layout_node,
            image: image_allocation,
        }
    }
}

impl TextBoxElement {
    pub fn set_layout(&mut self, layout: impl Into<LayoutStyle>) {
        self.layout_node.set_style(layout);
    }
}

impl Element for TextBoxElement {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        self.layout_node.clone()
    }

    fn layout_post(&mut self, resources: &mut SceneResources, rect: Rect) {
        self.buffer.lock().unwrap().set_size(
            &mut resources.font_system(),
            Some(rect.width()),
            Some(rect.height()),
        );

        resources.prepare_text(&self.buffer.lock().unwrap().buffer);
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        ctx.add_shape(&self.buffer.lock().unwrap().buffer)
    }

    fn ui_post(&mut self, ctx: &mut SceneContext, _rect: Rect) {
        // ctx.pop_clip_rect();
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(AccessRole::TextRun)
    }
}


// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// enum AvailableSpaceCache {
//     Definite(CachedFloat),
//     MinContent,
//     MaxContent,
// }

// impl From<AvailableSpace> for AvailableSpaceCache {
//     fn from(value: AvailableSpace) -> Self {
//         match value {
//             AvailableSpace::Definite(v) => Self::Definite(v.into()),
//             AvailableSpace::MinContent => Self::MinContent,
//             AvailableSpace::MaxContent => Self::MaxContent,
//         }
//     }
// }
