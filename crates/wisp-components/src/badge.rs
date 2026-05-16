use tiny_skia::Pixmap;
use wisp::draw;

/// 6 variants matching shadcn exactly.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

/// shadcn badge variant colors for dark mode.
/// Matches shadcn's default dark theme exactly.
pub fn badge_variant_colors(variant: BadgeVariant) -> BadgeColors {
    match variant {
        BadgeVariant::Default => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(59, 130, 246, 255),    // --primary (#3b82f6)
            fg: tiny_skia::Color::from_rgba8(250, 250, 250, 255),    // --primary-foreground
            border: None,
        },
        BadgeVariant::Secondary => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(39, 39, 42, 255),       // --secondary (#27272a)
            fg: tiny_skia::Color::from_rgba8(250, 250, 250, 255),    // --secondary-foreground
            border: None,
        },
        BadgeVariant::Destructive => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(69, 26, 26, 26),        // --destructive/10 = rgba(239,68,68,0.1)
            fg: tiny_skia::Color::from_rgba8(239, 68, 68, 255),     // --destructive (#ef4444)
            border: None,
        },
        BadgeVariant::Outline => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(0, 0, 0, 0),            // transparent
            fg: tiny_skia::Color::from_rgba8(250, 250, 250, 255),    // --foreground
            border: Some(tiny_skia::Color::from_rgba8(39, 39, 42, 255)), // --border
        },
        BadgeVariant::Ghost => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(0, 0, 0, 0),            // transparent
            fg: tiny_skia::Color::from_rgba8(250, 250, 250, 255),    // --foreground
            border: None,
        },
        BadgeVariant::Link => BadgeColors {
            bg: tiny_skia::Color::from_rgba8(0, 0, 0, 0),            // transparent
            fg: tiny_skia::Color::from_rgba8(59, 130, 246, 255),    // --primary (#3b82f6)
            border: None,
        },
    }
}

#[derive(Debug, Clone)]
pub struct BadgeColors {
    pub bg: tiny_skia::Color,
    pub fg: tiny_skia::Color,
    pub border: Option<tiny_skia::Color>,
}

/// Props matching shadcn badge spec exactly.
/// Default values match shadcn: h-5(20px), px-2(8px), py-0.5(2px), text-xs(12px), pill shape.
#[derive(Debug, Clone, Default)]
pub struct BadgeProps<'a> {
    pub text: &'a str,
    pub variant: BadgeVariant,
    pub focused: bool,
    pub colors: Option<BadgeColors>,
    /// Optional icon pixmap (12x12 preferred, matches shadcn [&>svg]:size-3)
    pub icon: Option<&'a Pixmap>,
}

/// Compute the width of a badge without drawing (for layout).
pub fn badge_size(text: &str, font_system: &mut cosmic_text::FontSystem, font_family: &str) -> f32 {
    let font_size = 12.0;
    let padding_x = 8.0;
    let text_w = draw::text_width(font_system, text, font_size, font_family);
    text_w + padding_x * 2.0
}

/// Draw a badge matching shadcn spec exactly.
///
/// Spec: h-5(20px) w-fit px-2(8px) py-0.5(2px) text-xs(12px) font-medium
///       gap-1(4px) rounded-4xl(pill) border-transparent
///       [&>svg]:size-3!(12px)
/// Returns (width, height).
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
    let font_size = 12.0;     // shadcn: text-xs = 12px
    let padding_x = 8.0;      // shadcn: px-2 = 8px
    let gap = 4.0;            // shadcn: gap-1 = 4px
    let icon_size = 12.0;     // shadcn: [&>svg]:size-3! = 12px
    let height = 20.0;        // shadcn: h-5 = 20px

    let text_w = draw::text_width(font_system, props.text, font_size, font_family);
    let icon_w = if props.icon.is_some() { icon_size + gap } else { 0.0 };
    let content_w = icon_w + text_w;
    let bw = content_w + padding_x * 2.0;
    let bh = height;           // fixed height, not computed
    let corner_r = bh / 2.0;   // shadcn: rounded-4xl = pill (50%)

    // Background
    if colors.bg.alpha() > 0.0 {
        draw::fill_rounded_rect(pixmap, x, y, bw, bh, [corner_r; 4], colors.bg);
    }

    // Border (always 1px transparent by default, visible on outline)
    if let Some(border_color) = colors.border {
        draw::stroke_rounded_rect(pixmap, x + 0.5, y + 0.5, bw - 1.0, bh - 1.0, corner_r, border_color, 1.0);
    }

    // Focus ring (shadcn: focus-visible:ring-[3px] ring-ring/50)
    if props.focused {
        let ring = tiny_skia::Color::from_rgba8(
            250, 250, 250, 127, // ring-ring/50 (50% opacity of ring color)
        );
        draw::stroke_rounded_rect(pixmap, x - 1.5, y - 1.5, bw + 3.0, bh + 3.0, corner_r + 1.5, ring, 3.0);
    }

    // Content (centered: items-center justify-center gap-1)
    let content_x = x + (bw - content_w) / 2.0;
    let content_y = y + (bh - font_size) / 2.0;

    // Icon (shadcn: [&>svg]:size-3! = 12px)
    if let Some(icon) = props.icon {
        let icon_y = y + (bh - icon_size) / 2.0;
        draw::draw_pixmap_at(pixmap, icon, content_x, icon_y);
    }

    // Text (shadcn: text-xs font-medium whitespace-nowrap)
    let text_x = content_x + icon_w;
    let text_y = content_y;
    draw::draw_text(pixmap, font_system, swash_cache, props.text, text_x, text_y, font_size, font_family, colors.fg);

    (bw, bh)
}
