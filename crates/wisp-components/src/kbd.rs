use tiny_skia::Pixmap;
use wisp::draw;

#[derive(Debug, Clone)]
pub struct KbdProps<'a> {
    pub keys: &'a str,
    pub font_size: f32,
    pub padding_x: f32,
}

impl<'a> Default for KbdProps<'a> {
    fn default() -> Self {
        Self {
            keys: "",
            font_size: 11.0,
            padding_x: 4.0,
        }
    }
}

/// Draw a shadcn-style keyboard shortcut indicator.
/// Returns (width, height).
pub fn kbd(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    x: f32,
    y: f32,
    props: &KbdProps,
    font_family: &str,
) -> (f32, f32) {
    let text_w = draw::text_width(font_system, props.keys, props.font_size, font_family);
    let text_h = props.font_size;
    let bw = text_w + props.padding_x * 2.0;
    let bh = text_h + 4.0;
    let corner_r = 4.0;

    let bg = tiny_skia::Color::from_rgba8(30, 30, 46, 255);
    let border = tiny_skia::Color::from_rgba8(69, 71, 90, 200);
    let fg = tiny_skia::Color::from_rgba8(166, 173, 200, 255);

    draw::fill_rounded_rect(pixmap, x, y, bw, bh, [corner_r; 4], bg);
    draw::stroke_rect(pixmap, x + 0.5, y + 0.5, bw - 1.0, bh - 1.0, border, 1.0);

    let text_x = x + (bw - text_w) / 2.0;
    let text_y = y + (bh - text_h) / 2.0;
    draw::draw_text(pixmap, font_system, swash_cache, props.keys, text_x, text_y, props.font_size, font_family, fg);

    (bw, bh)
}

/// Measure a kbd chip width without drawing.
pub fn measure_kbd(
    font_system: &mut cosmic_text::FontSystem,
    text: &str,
    font_size: f32,
    padding_x: f32,
    font_family: &str,
) -> (f32, f32) {
    let text_w = draw::text_width(font_system, text, font_size, font_family);
    (text_w + padding_x * 2.0, font_size + 4.0)
}
