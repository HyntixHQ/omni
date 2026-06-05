use tiny_skia::Pixmap;
use wisp::draw;

/// shadcn Label — renders a `<label>` equivalent.
///
/// Spec: text-sm(14px) font-medium(500) leading-none
///       peer-disabled:cursor-not-allowed peer-disabled:opacity-70
#[derive(Debug, Clone)]
pub struct LabelProps<'a> {
    pub text: &'a str,
    pub font_size: f32,
    pub color: tiny_skia::Color,
    pub disabled: bool,
}

impl<'a> Default for LabelProps<'a> {
    fn default() -> Self {
        Self {
            text: "",
            font_size: 14.0,
            color: tiny_skia::Color::from_rgba8(250, 250, 250, 255),
            disabled: false,
        }
    }
}

/// Draw a shadcn-style label.
/// Returns the bottom y position.
pub fn label(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    x: f32,
    y: f32,
    props: &LabelProps,
    font_family: &str,
) -> f32 {
    let opacity = if props.disabled { 0.7 } else { 1.0 };
    let mut c = props.color;
    let a = (c.alpha() * opacity * 255.0) as u8;
    c = tiny_skia::Color::from_rgba8(
        (c.red() * 255.0) as u8,
        (c.green() * 255.0) as u8,
        (c.blue() * 255.0) as u8,
        a,
    );
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        props.text,
        x,
        y,
        props.font_size,
        font_family,
        c,
    );
    y + props.font_size
}
