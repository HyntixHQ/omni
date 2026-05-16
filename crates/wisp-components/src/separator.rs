use tiny_skia::Pixmap;
use wisp::draw;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeparatorOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct SeparatorProps {
    pub orientation: SeparatorOrientation,
    pub thickness: f32,
    pub color: tiny_skia::Color,
    pub alpha: u8,
    /// shadcn: decorative controls whether the separator is a semantic
    /// `role="separator"` or purely decorative. Does not affect rendering.
    pub decorative: bool,
}

impl Default for SeparatorProps {
    fn default() -> Self {
        Self {
            orientation: SeparatorOrientation::Horizontal,
            thickness: 1.0,
            color: tiny_skia::Color::from_rgba8(69, 71, 90, 255),
            alpha: 255,
            decorative: true,
        }
    }
}

/// GPUI-style convenience: horizontal separator with defaults.
pub fn h_separator() -> SeparatorProps {
    SeparatorProps {
        orientation: SeparatorOrientation::Horizontal,
        ..Default::default()
    }
}

/// GPUI-style convenience: vertical separator with defaults.
pub fn v_separator() -> SeparatorProps {
    SeparatorProps {
        orientation: SeparatorOrientation::Vertical,
        ..Default::default()
    }
}

/// Draws a shadcn/GPUI-style separator (divider).
pub fn separator(pixmap: &mut Pixmap, x: f32, y: f32, length: f32, props: &SeparatorProps) {
    let mut c = props.color;
    let r = (c.red() * 255.0) as u8;
    let g = (c.green() * 255.0) as u8;
    let b = (c.blue() * 255.0) as u8;
    c = tiny_skia::Color::from_rgba8(r, g, b, props.alpha);

    match props.orientation {
        SeparatorOrientation::Horizontal => {
            draw::fill_rect(pixmap, x, y, length, props.thickness, c);
        }
        SeparatorOrientation::Vertical => {
            draw::fill_rect(pixmap, x, y, props.thickness, length, c);
        }
    }
}
