use tiny_skia::Pixmap;
use wisp::draw;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BadgeVariant {
    Default,
    Secondary,
    Destructive,
    Outline,
}

#[derive(Debug, Clone)]
pub struct BadgeColors {
    pub bg: tiny_skia::Color,
    pub fg: tiny_skia::Color,
    pub border: Option<tiny_skia::Color>,
}

/// Returns shadcn-style colors for each badge variant.
/// These match shadcn's default theme colors.
pub fn badge_variant_colors(variant: BadgeVariant) -> BadgeColors {
    match variant {
        BadgeVariant::Default => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(49, 50, 68, 255),
            fg: tiny_skia::Color::from_rgba8(205, 214, 244, 255),
            border: None,
        },
        BadgeVariant::Secondary => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(69, 71, 90, 255),
            fg: tiny_skia::Color::from_rgba8(205, 214, 244, 255),
            border: None,
        },
        BadgeVariant::Destructive => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(243, 139, 168, 255),
            fg: tiny_skia::Color::from_rgba8(30, 30, 46, 255),
            border: None,
        },
        BadgeVariant::Outline => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(0, 0, 0, 0),
            fg: tiny_skia::Color::from_rgba8(205, 214, 244, 255),
            border: Some(tiny_skia::Color::from_rgba8(88, 91, 112, 255)),
        },
    }
}

#[derive(Debug, Clone)]
pub struct BadgeProps<'a> {
    pub text: &'a str,
    pub variant: BadgeVariant,
    pub font_size: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub colors: Option<BadgeColors>,
}

impl<'a> Default for BadgeProps<'a> {
    fn default() -> Self {
        Self {
            text: "",
            variant: BadgeVariant::Secondary,
            font_size: 11.0,
            padding_x: 6.0,
            padding_y: 2.0,
            colors: None,
        }
    }
}

/// Draw a shadcn-style badge. Returns (width, height).
pub fn badge(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    x: f32,
    y: f32,
    props: &BadgeProps,
    font_family: &str,
) -> (f32, f32) {
    let colors = props.colors.clone().unwrap_or_else(|| badge_variant_colors(props.variant));
    let text = props.text;

    let text_w = draw::text_width(font_system, text, props.font_size, font_family);
    let text_h = props.font_size;
    let content_w = text_w;
    let content_h = text_h;
    let bw = content_w + props.padding_x * 2.0;
    let bh = (content_h + props.padding_y * 2.0).round();
    let corner_r = (bh / 3.0).max(3.0);

    // Background
    if colors.bg.alpha() > 0.0 {
        draw::fill_rounded_rect(pixmap, x, y, bw, bh, [corner_r; 4], colors.bg);
    }

    // Border (outline variant)
    if let Some(border_color) = colors.border {
        draw::stroke_rect(pixmap, x + 0.5, y + 0.5, bw - 1.0, bh - 1.0, border_color, 1.0);
    }

    // Text
    let text_x = x + (bw - text_w) / 2.0;
    let text_y = y + (bh - text_h) / 2.0;
    draw::draw_text(pixmap, font_system, swash_cache, text, text_x, text_y, props.font_size, font_family, colors.fg);

    (bw, bh)
}
