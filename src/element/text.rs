use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    atlas::AtlasAllocation,
    color::ColorRgba,
    math::{PhysicalSize, Pos},
    scene::layout::{measure_func_boxed, AvailableSpace, FlexBox, LayoutPassResult, Measurable},
    shape::PaintFill,
    util::{
        guard::ReadLockable,
        text::{AtlasContentType, FontSystemRef, TextBox},
    },
};

use crate::util::text::{Attrs, Metrics};
use ordered_float::OrderedFloat;
use rustc_hash::FxHashMap;

use crate::{
    element::Element,
    math::{Rect, Size},
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
};

struct CacheBuffer {
    buffer: TextBox,

    invalidate_cache: bool,

    // TODO: make this a LRU cache to ease memory consumption...
    cache: FxHashMap<MeasureTextBoxCacheKey, Size>,
}

impl From<TextBox> for CacheBuffer {
    fn from(buffer: TextBox) -> Self {
        Self {
            buffer,
            invalidate_cache: false,

            cache: Default::default(),
        }
    }
}

pub struct TextBoxElement {
    buffer: Arc<Mutex<CacheBuffer>>,

    layout_node: LayoutPassResult,

    logical_metrics: Metrics,

    text: String,
    attrs: Attrs<'static>,

    image: AtlasAllocation,
}

struct TextBoxMeasureFunc {
    font_system: FontSystemRef,
    buffer: Arc<Mutex<CacheBuffer>>,
}

impl Measurable for TextBoxMeasureFunc {
    fn measure(
        &self,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
    ) -> Size<f32> {
        let Size {
            width: available_width,
            height: available_height,
            ..
        } = available_space;

        let Size { width, height, .. } = known_dimensions;

        let tbox_width = width.unwrap_or(match available_width {
            AvailableSpace::Definite(max_width) => max_width,
            AvailableSpace::MinContent => 0.,
            AvailableSpace::MaxContent => f32::INFINITY,
        });

        let tbox_height = height.unwrap_or(match available_height {
            AvailableSpace::Definite(max_height) => max_height,
            AvailableSpace::MinContent => 0.,
            AvailableSpace::MaxContent => f32::INFINITY,
        });

        let cache_key = MeasureTextBoxCacheKey::from_measure_fn(tbox_width, tbox_height);

        let mut buffer = self.buffer.lock().unwrap();

        if buffer.invalidate_cache {
            buffer.cache.clear();
            buffer.invalidate_cache = false;
        }

        // TODO: would be nice to use `or_insert_with` here instead...
        let result = match buffer.cache.get(&cache_key).cloned() {
            Some(res) => res,
            None => {
                buffer.set_size(
                    &mut self.font_system.lock().unwrap(),
                    tbox_width,
                    tbox_height,
                );

                let size = buffer.buffer.computed_size();

                buffer.cache.insert(cache_key, size);

                size
            }
        }
        .into();

        result
    }
}

impl TextBoxElement {
    pub fn new(
        scene_resources: &mut SceneResources,
        metrics: Metrics,
        color: impl Into<PaintFill>,
        text: String,
        attrs: Attrs<'static>,
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

            buffer.set_text(&mut font_system, &text, attrs);

            buffer.shape_until_scroll(&mut font_system);

            buffer
        };

        let buffer = Arc::new(Mutex::new(CacheBuffer::from(buffer)));

        let text_box_measure_func = TextBoxMeasureFunc {
            font_system: scene_resources.font_system_ref(),
            buffer: buffer.clone(),
        };

        let layout_node = scene_resources
            .layout_engine()
            .new_leaf_with_measure(
                FlexBox::builder(),
                measure_func_boxed(text_box_measure_func),
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

impl Element for TextBoxElement {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        self.layout_node.clone()
    }

    fn layout_post(&mut self, resources: &mut SceneResources, rect: Rect) {
        self.buffer.lock().unwrap().set_size(
            &mut resources.font_system(),
            rect.width(),
            rect.height(),
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
        AccessNodeBuilder::new(AccessRole::StaticText)
    }
}

// TODO: do this with 1/3 subpixel binning...
type CachedFloat = OrderedFloat<f32>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct MeasureTextBoxCacheKey {
    width: Option<CachedFloat>,
    height: Option<CachedFloat>,
}

impl MeasureTextBoxCacheKey {
    fn from_measure_fn(tbox_width: f32, tbox_height: f32) -> Self {
        Self {
            width: Some(tbox_width.into()),
            height: Some(tbox_height.into()),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum AvailableSpaceCache {
    Definite(CachedFloat),
    MinContent,
    MaxContent,
}

impl From<AvailableSpace> for AvailableSpaceCache {
    fn from(value: AvailableSpace) -> Self {
        match value {
            AvailableSpace::Definite(v) => Self::Definite(v.into()),
            AvailableSpace::MinContent => Self::MinContent,
            AvailableSpace::MaxContent => Self::MaxContent,
        }
    }
}

impl Deref for CacheBuffer {
    type Target = cosmic_text::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for CacheBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
