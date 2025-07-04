use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut, Mul},
    sync::{Arc, Mutex},
};

use cosmic_text::{Edit, PhysicalGlyph, Scroll, Shaping};
use ordered_float::{NotNan, OrderedFloat};
use rustc_hash::FxHashMap;
use shrinkwraprs::Shrinkwrap;

pub use cosmic_text::{
    Affinity, Attrs, BufferLine, CacheKey as GlyphCacheKey, Cursor, LayoutCursor, LayoutLine,
    LayoutRunIter, Metrics, ShapeLine, Wrap,
};

use crate::{
    color::{ColorRgba, FromCosmicTextColor},
    math::{
        PhysicalPos, PhysicalSize, PhysicalVector, Pos, Rect, RoundedRect, ScaleFactor, Size,
        Vector,
    },
    shape::PaintFill,
};

use super::{LogicalUnit, PhysicalUnit};

#[repr(u32)]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum AtlasContentType {
    Color = 0,
    Mask = 1,
}

impl AtlasContentType {
    pub const fn num_channels(&self) -> u32 {
        match self {
            AtlasContentType::Color => 4,
            AtlasContentType::Mask => 1,
        }
    }

    pub const fn bytes_per_channel(&self) -> u32 {
        match self {
            AtlasContentType::Color => 1,
            AtlasContentType::Mask => 1,
        }
    }
}

#[derive(Shrinkwrap, Clone)]
pub struct FontSystemRef(Arc<Mutex<FontSystem>>);

impl FontSystemRef {
    pub fn new() -> Self {
        Self::from(FontSystem::new())
    }
}

impl From<FontSystem> for FontSystemRef {
    #[inline(always)]
    fn from(value: FontSystem) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}

#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct FontSystem {
    #[shrinkwrap(main_field)]
    pub inner: cosmic_text::FontSystem,
}

impl FontSystem {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            inner: cosmic_text::FontSystem::new(),
        }
    }
}

pub struct PlacedGlyph<U = PhysicalUnit> {
    pub glyph: PhysicalGlyph,
    pub depth: f32,
    pub color: PaintFill,
    _unit: PhantomData<U>,
}

impl<U: std::fmt::Debug> PlacedGlyph<U> {
    fn from_layout_glyph<UnitFrom>(
        glyph: &cosmic_text::LayoutGlyph,
        scale_fac: ScaleFactor<UnitFrom, U>,
        text_box_pos: Pos<f32, UnitFrom>,
        default_color: impl Into<PaintFill>,
        line_y: f32,
    ) -> Self
    where
        UnitFrom: std::fmt::Debug,
    {
        Self {
            glyph: glyph.physical(
                // TODO: we should use the text box pos here...
                (Vector::new(0., line_y) * scale_fac.as_float()).into(),
                scale_fac.get().into(),
            ),
            depth: 0.,
            color: glyph
                .color_opt
                .map(ColorRgba::from_cosmic)
                .map(Into::into)
                .unwrap_or_else(|| default_color.into()),
            _unit: PhantomData,
        }
    }

    pub fn to_draw_glyph<TargetUnit>(
        &self,
        pos: Pos<f32, TargetUnit>,
        size: PhysicalSize<u32>,
        mut placement: PhysicalPos<i32>,
        scale_fac: ScaleFactor<PhysicalUnit, TargetUnit>,
    ) -> Rect<f32, TargetUnit> {
        placement.y *= -1;

        let scale_fac = scale_fac.as_float();

        Rect::from_min_size(
            pos + (placement.map(|x| x as f32) * scale_fac).to_vector()
                + (PhysicalVector::new(self.glyph.x as f32, self.glyph.y as f32) * scale_fac),
            size.map(|x| x as f32) * scale_fac,
        )
    }
}

pub struct PlacedTextBox<U = LogicalUnit> {
    pub glyphs: Vec<PlacedGlyph>,
    pub clip_rect: Option<RoundedRect<f32, U>>,
    pub pos: Pos<f32, U>,
    pub color: PaintFill,
    pub scale_fac: ScaleFactor<U, PhysicalUnit>,
    pub bounding_size: Size<f32, U>,
}

impl<U> PlacedTextBox<U> {
    pub fn new(
        glyphs: Vec<PlacedGlyph>,
        pos: Pos<f32, U>,
        color: impl Into<PaintFill>,
        clip_rect: Option<RoundedRect<f32, U>>,
        scale_fac: ScaleFactor<U, PhysicalUnit>,
        bounding_size: Size<f32, U>,
    ) -> Self {
        Self {
            glyphs,
            clip_rect,
            pos,
            color: color.into(),
            scale_fac,
            bounding_size,
        }
    }

    pub fn glyph_cache_keys(&self) -> impl Iterator<Item = GlyphCacheKey> + '_ {
        self.glyphs.iter().map(|glyph| glyph.glyph.cache_key)
    }

    #[inline]
    pub fn with_clip_rect(self, clip_rect: impl Into<Option<RoundedRect<f32, U>>>) -> Self {
        Self {
            clip_rect: clip_rect.into(),
            ..self
        }
    }
}

pub trait HasBuffer {
    fn buffer(&self) -> &cosmic_text::Buffer;
    fn buffer_mut(&mut self) -> &mut cosmic_text::Buffer;

    fn new(font_system: &mut FontSystem, font_size: f32, line_height: f32) -> Self where Self: Sized;

    fn editor(&self) -> Option<&cosmic_text::Editor<'static>> where Self: Sized { None }
    fn editor_mut(&mut self) -> Option<&mut cosmic_text::Editor<'static>> where Self: Sized { None }
}

impl HasBuffer for cosmic_text::Buffer {
    #[inline(always)]
    fn buffer(&self) -> &cosmic_text::Buffer {
        self
    }

    #[inline(always)]
    fn buffer_mut(&mut self) -> &mut cosmic_text::Buffer {
        self
    }

    #[inline(always)]
    fn new(font_system: &mut FontSystem, font_size: f32, line_height: f32) -> Self {
        cosmic_text::Buffer::new(font_system, cosmic_text::Metrics::new(font_size, line_height))
    }
}

impl HasBuffer for cosmic_text::Editor<'static> {
    #[inline(always)]
    fn buffer(&self) -> &cosmic_text::Buffer {
        match self.buffer_ref() {
            cosmic_text::BufferRef::Owned(buffer) => buffer,
            cosmic_text::BufferRef::Borrowed(buffer) => buffer,
            cosmic_text::BufferRef::Arc(buffer) => buffer,
        }
    }

    #[inline(always)]
    fn buffer_mut(&mut self) -> &mut cosmic_text::Buffer {
        match self.buffer_ref_mut() {
            cosmic_text::BufferRef::Owned(buffer) => buffer,
            cosmic_text::BufferRef::Borrowed(buffer) => buffer,
            cosmic_text::BufferRef::Arc(_) => {
                panic!("Unsupported mutable reference to buffer")
            }
        }
    }

    #[inline(always)]
    fn new(font_system: &mut FontSystem, font_size: f32, line_height: f32) -> Self {
        cosmic_text::Editor::new(cosmic_text::Buffer::new(
            font_system,
            cosmic_text::Metrics::new(font_size, line_height),
        ))
    }

    #[inline(always)]
    fn editor(&self) -> Option<&cosmic_text::Editor<'static>> {
        Some(self)
    }

    #[inline(always)]
    fn editor_mut(&mut self) -> Option<&mut cosmic_text::Editor<'static>> {
        Some(self)
    }
}

pub struct TextBox<Buffer: HasBuffer = cosmic_text::Buffer, U = LogicalUnit> {
    pub buffer: Buffer,
    pub color: PaintFill,
    pub pos: Pos<f32, U>,
    _unit: PhantomData<U>,
}

impl<U, Buffer: HasBuffer> Deref for TextBox<Buffer, U> {
    type Target = cosmic_text::Buffer;

    fn deref(&self) -> &Self::Target {
        self.buffer.buffer()
    }
}

impl<U, Buffer: HasBuffer> DerefMut for TextBox<Buffer, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer.buffer_mut()
    }
}

pub(crate) trait TextBoxLike<U: std::fmt::Debug = LogicalUnit> {
    fn calculate_placed_text_box(
        &self,
        clip_rect: Option<RoundedRect<f32, U>>,
        scale_factor: ScaleFactor<U, PhysicalUnit>,
    ) -> PlacedTextBox<U>;
}

impl<Buffer: HasBuffer, U: std::fmt::Debug> TextBoxLike<U> for TextBox<Buffer, U> {
    fn calculate_placed_text_box(
        &self,
        clip_rect: Option<RoundedRect<f32, U>>,
        scale_factor: ScaleFactor<U, PhysicalUnit>,
    ) -> PlacedTextBox<U> {
        let bounding_size = self.computed_size();

        let glyphs = self
            .buffer
            .buffer()
            .layout_runs()
            .flat_map(|r| {
                r.glyphs.iter().map(move |g| {
                    PlacedGlyph::from_layout_glyph(g, scale_factor, self.pos, self.color, r.line_y)
                })
            })
            .collect();

        PlacedTextBox {
            glyphs: glyphs,
            clip_rect: clip_rect.into(),
            pos: self.pos,
            color: self.color,
            scale_fac: scale_factor,
            bounding_size,
        }
    }
}

impl<Buffer: HasBuffer, U: std::fmt::Debug> TextBox<Buffer, U> {
    pub fn new(
        font_system: &mut FontSystem,
        font_size: f32,
        line_height: f32,
        color: impl Into<PaintFill>,
        pos: Pos<f32, U>,
    ) -> Self {
        Self::new_with_buffer(
            Buffer::new(font_system, font_size, line_height),
            color,
            pos,
        )
    }
    
    pub fn new_with_buffer(buffer: Buffer, color: impl Into<PaintFill>, pos: Pos<f32, U>) -> Self {
        Self {
            buffer,
            color: color.into(),
            pos,
            _unit: PhantomData,
        }
    }

    #[inline(always)]
    pub fn lines(&self) -> &Vec<BufferLine> {
        &self.buffer.buffer().lines
    }

    #[inline(always)]
    pub fn lines_mut(&mut self) -> &mut Vec<BufferLine> {
        &mut self.buffer.buffer_mut().lines
    }

    #[inline]
    pub fn set_font_size(&mut self, font_system: &mut FontSystem, font_size: f32) {
        let mut metrics = self.buffer.buffer().metrics();
        metrics.font_size = font_size;
        self.buffer.buffer_mut().set_metrics(font_system, metrics);
    }

    #[inline]
    pub fn set_line_height(&mut self, font_system: &mut FontSystem, line_height: f32) {
        let mut metrics = self.buffer.buffer().metrics();
        metrics.line_height = line_height;
        self.buffer.buffer_mut().set_metrics(font_system, metrics);
    }

    // /// Pre-shape lines in the buffer, up to `lines`, return actual number of layout lines
    // #[inline(always)]
    // pub fn shape_until(&mut self, font_system: &mut FontSystem, lines: i32) -> i32 {
    //     self.buffer.shape_until(font_system, lines)
    // }

    /// Shape lines until cursor, also scrolling to include cursor in view
    #[inline(always)]
    pub fn shape_until_cursor(&mut self, font_system: &mut FontSystem, cursor: Cursor) {
        self.buffer
            .buffer_mut()
            .shape_until_cursor(font_system, cursor, false)
    }

    /// Shape lines until scroll
    #[inline(always)]
    pub fn shape_until_scroll(&mut self, font_system: &mut FontSystem) {
        self.buffer.buffer_mut().shape_until_scroll(font_system, false)
    }

    #[inline(always)]
    pub fn layout_cursor(
        &mut self,
        font_system: &mut FontSystem,
        cursor: Cursor,
    ) -> Option<LayoutCursor> {
        self.buffer.buffer_mut().layout_cursor(font_system, cursor)
    }

    /// Shape the provided line index and return the result
    #[inline(always)]
    pub fn line_shape(
        &mut self,
        font_system: &mut FontSystem,
        line_i: usize,
    ) -> Option<&ShapeLine> {
        self.buffer.buffer_mut().line_shape(font_system, line_i)
    }

    /// Lay out the provided line index and return the result
    #[inline(always)]
    pub fn line_layout(
        &mut self,
        font_system: &mut FontSystem,
        line_i: usize,
    ) -> Option<&[LayoutLine]> {
        self.buffer.buffer_mut().line_layout(font_system, line_i)
    }

    /// Get the current [`Metrics`]
    #[inline(always)]
    pub fn metrics(&self) -> Metrics {
        self.buffer.buffer().metrics()
    }

    /// Set the current [`Metrics`]
    ///
    /// # Panics
    ///
    /// Will panic if `metrics.font_size` is zero.
    #[inline(always)]
    pub fn set_metrics(&mut self, font_system: &mut FontSystem, metrics: Metrics) {
        self.buffer.buffer_mut().set_metrics(font_system, metrics)
    }

    /// Get the current [`Wrap`]
    #[inline(always)]
    pub fn wrap(&self) -> Wrap {
        self.buffer.buffer().wrap()
    }

    /// Set the current [`Wrap`]
    #[inline(always)]
    pub fn set_wrap(&mut self, font_system: &mut FontSystem, wrap: Wrap) {
        self.buffer.buffer_mut().set_wrap(font_system, wrap)
    }

    /// Get the current buffer dimensions (width, height)
    #[inline(always)]
    pub fn size(&self) -> (Option<f32>, Option<f32>) {
        self.buffer.buffer().size()
    }

    /// Get the computed size of the buffer; runs a layout pass
    pub fn computed_size(&self) -> Size<f32, U> {
        let lh = self.metrics().line_height;

        self.layout_runs()
            .fold(Size::new(0.0, 0.0), |mut size, run| {
                let new_width = run.line_w;
                if new_width > size.width {
                    size.width = new_width;
                }

                size.height += lh;

                size
            })
    }

    /// Get the current scroll location
    #[inline(always)]
    pub fn scroll(&self) -> Scroll {
        self.buffer.buffer().scroll()
    }

    /// Set the current scroll location
    #[inline(always)]
    pub fn set_scroll(&mut self, scroll: Scroll) {
        self.buffer.buffer_mut().set_scroll(scroll)
    }

    // /// Get the number of lines that can be viewed in the buffer
    // #[inline(always)]
    // pub fn visible_lines(&self) -> Scroll {
    //     self.buffer.visible_lines()
    // }

    /// Set text of buffer, using provided attributes for each line by default
    #[inline(always)]
    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str, attrs: &Attrs) {
        self.buffer
            .buffer_mut()
            .set_text(font_system, text, attrs, Shaping::Basic)
    }

    /// True if a redraw is needed
    #[inline(always)]
    pub fn redraw(&self) -> bool {
        self.buffer.buffer().redraw()
    }

    /// Set redraw needed flag
    #[inline(always)]
    pub fn set_redraw(&mut self, redraw: bool) {
        self.buffer.buffer_mut().set_redraw(redraw)
    }

    /// Get the visible layout runs for rendering and other tasks
    #[inline(always)]
    pub fn layout_runs(&self) -> LayoutRunIter {
        self.buffer.buffer().layout_runs()
    }

    /// Convert x, y position to Cursor (hit detection)
    #[inline(always)]
    pub fn hit(&self, x: f32, y: f32) -> Option<Cursor> {
        self.buffer.buffer().hit(x, y)
    }
}

// Text render caching primitives

// TODO: do this with 1/3 subpixel binning...
pub type CachedFloat = OrderedFloat<f32>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextBoxSizeCacheKey {
    width: Option<CachedFloat>,
    height: Option<CachedFloat>,
}

pub struct TextCacheBuffer<Buffer: HasBuffer = cosmic_text::Buffer> {
    pub buffer: TextBox<Buffer>,
    pub computed_size: Option<Size<f32>>,

    pub invalidate_cache: bool,

    // TODO: make this a LRU cache to ease memory consumption...
    pub cache: FxHashMap<TextBoxSizeCacheKey, Size>,
}

impl Deref for TextCacheBuffer {
    type Target = cosmic_text::Buffer;

    fn deref(&self) -> &Self::Target {
        &*self.buffer
    }
}

impl DerefMut for TextCacheBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.buffer
    }
}

impl<Buffer: HasBuffer> From<TextBox<Buffer>> for TextCacheBuffer<Buffer> {
    fn from(buffer: TextBox<Buffer>) -> Self {
        Self {
            buffer,
            invalidate_cache: false,
            computed_size: None,

            cache: Default::default(),
        }
    }
}

pub trait TextCacheBufferLike {
    fn compute_size(&mut self) -> Size<f32>;

    /// Set the current buffer dimensions
    fn set_size(
        &mut self,
        font_system: &mut FontSystem,
        width: Option<f32>,
        height: Option<f32>,
    );
}

impl<Buffer: HasBuffer> TextCacheBufferLike for TextCacheBuffer<Buffer> {
    fn compute_size(&mut self) -> Size<f32> {
        let computed_size = self.buffer.computed_size();
        self.computed_size = Some(computed_size);
        computed_size
    }

    /// Set the current buffer dimensions
    fn set_size(
        &mut self,
        font_system: &mut FontSystem,
        width: Option<f32>,
        height: Option<f32>,
    ) {
        self.buffer.buffer_mut().set_size(font_system, width, height)
    }
}
