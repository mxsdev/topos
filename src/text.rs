use accesskit::Rect;

#[repr(u32)]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub(crate) enum GlyphContentType {
    Color = 0,
    Mask = 1,
}

impl GlyphContentType {
    pub fn num_channels(&self) -> u32 {
        match self {
            GlyphContentType::Color => 4,
            GlyphContentType::Mask => 1,
        }
    }
}

// pub struct TextBox {
//     buffer: cosmic_text::Buffer,
//     clip_rect: Option<Rect>,
// }

// pub struct TextView {}

pub fn render_text() {
    use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, SwashCache};

    // A FontSystem provides access to detected system fonts, create one per application
    let mut font_system = FontSystem::new();
    // font_system.
    // FontSystem::new_with_fonts(fonts)

    // A SwashCache stores rasterized glyphs, create one per application
    let mut swash_cache = SwashCache::new();

    // Text metrics indicate the font size and line height of a buffer
    let metrics = Metrics::new(14.0, 20.0);

    // A Buffer provides shaping and layout for a UTF-8 string, create one per text widget
    let mut buffer = Buffer::new(&mut font_system, metrics);

    // Borrow buffer together with the font system for more convenient method calls
    // let mut buffer = buffer.borrow_with(&mut font_system);

    // Set a size for the text buffer, in pixels
    buffer.set_size(&mut font_system, 20.0, 25.0);

    // Attributes indicate what font to choose
    let attrs = Attrs::new();

    // attrs.family(cosmic_text::Family::Name(()))
    // attrs.family(Family)

    // Add some text!
    buffer.set_text(&mut font_system, "Hello, Rust! 🦀\n", attrs);

    // Perform shaping as desired
    buffer.shape_until_scroll(&mut font_system);

    // Inspect the output runs
    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            // glyph.cache_key
            // glyph.cache_key.font_size_bits
            swash_cache.get_image_uncached(&mut font_system, glyph.cache_key);
            // glyph.x_offset
            // glyph.cache_key
            // println!("{:#?}", glyph);
        }
    }

    // Create a default text color
    let _text_color = Color::rgb(0xFF, 0xFF, 0xFF);

    // swash_cache.image_cache

    // Draw the buffer (for performance, instead use SwashCache directly)
    // buffer.draw(&mut swash_cache, text_color, |x, y, w, h, color| {
    //     // Fill in your code here for drawing rectangles
    // });
}
