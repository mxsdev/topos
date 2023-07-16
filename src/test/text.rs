use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    color::ColorRgba,
    scene::{
        self,
        layout::{
            self, measure_func_boxed, AvailableSpace, FlexBox, LayoutPassResult, Measurable,
            MeasureFunc,
        },
    },
    surface::RenderingContext,
    util::layout::LayoutNode,
};

use cosmic_text::{Attrs, FontSystem, Metrics};
use ordered_float::OrderedFloat;
use rustc_hash::FxHashMap;

use crate::{
    atlas::PlacedTextBox,
    element::Element,
    math::{Rect, Size},
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
};

struct CacheBuffer {
    buffer: cosmic_text::Buffer,

    invalidate_cache: bool,

    // TODO: make this a LRU cache to ease memory consumption...
    cache: FxHashMap<MeasureTextBoxCacheKey, Size>,
}

impl From<cosmic_text::Buffer> for CacheBuffer {
    fn from(buffer: cosmic_text::Buffer) -> Self {
        Self {
            buffer,
            invalidate_cache: false,

            // invalid_key: Default::default(),
            cache: Default::default(),
        }
    }
}

pub struct TextBox {
    buffer: Arc<Mutex<CacheBuffer>>,

    layout_node: LayoutPassResult,

    logical_metrics: Metrics,
    color: ColorRgba,

    text: String,
    attrs: Attrs<'static>,
}

struct TextBoxMeasureFunc {
    font_system: Arc<Mutex<FontSystem>>,
    buffer: Arc<Mutex<CacheBuffer>>,
    rendering_context: Arc<RenderingContext>,
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
                let scale_factor = self.rendering_context.texture_info.get_scale_factor().get();

                buffer.set_size(
                    &mut self.font_system.lock().unwrap(),
                    tbox_width * scale_factor,
                    tbox_height * scale_factor,
                );

                let lh = buffer.metrics().line_height;

                let size = buffer
                    .layout_runs()
                    .fold(Size::new(0.0, 0.0), |mut size, run| {
                        let new_width = run.line_w;
                        if new_width > size.width {
                            size.width = new_width;
                        }

                        size.height += lh;

                        size
                    });

                buffer.cache.insert(cache_key, size);

                size
            }
        }
        .into();

        result
    }
}

impl TextBox {
    pub fn new(
        scene_resources: &mut SceneResources,
        metrics: Metrics,
        color: ColorRgba,
        text: String,
        attrs: Attrs<'static>,
    ) -> Self {
        let buffer = {
            let mut font_system = scene_resources.font_system();

            let mut buffer = cosmic_text::Buffer::new(
                &mut font_system,
                metrics.scale(scene_resources.scale_factor().get()),
            );

            buffer.set_text(&mut font_system, &text, attrs);

            buffer.shape_until_scroll(&mut font_system);

            buffer
        };

        let buffer = Arc::new(Mutex::new(CacheBuffer::from(buffer)));

        let text_box_measure_func = TextBoxMeasureFunc {
            font_system: scene_resources.font_system_ref(),
            buffer: buffer.clone(),
            rendering_context: scene_resources.rendering_context_ref(),
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
            color,
            logical_metrics: metrics,
            layout_node,
        }
    }
}

impl Element for TextBox {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        self.layout_node.clone()
    }

    fn layout_post(&mut self, resources: &mut SceneResources, rect: Rect) {
        self.buffer.lock().unwrap().set_size(
            &mut resources.font_system(),
            rect.width() * resources.scale_factor().get(),
            rect.height() * resources.scale_factor().get(),
        );
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
