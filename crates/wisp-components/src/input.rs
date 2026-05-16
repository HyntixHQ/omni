use tiny_skia::Pixmap;
use unicode_segmentation::UnicodeSegmentation;
use wisp::draw;
use wisp::style::Size;

#[derive(Debug, Clone)]
pub struct InputProps<'a> {
    pub value: &'a str,
    pub cursor_at: usize,
    pub placeholder: &'a str,
    pub cursor_visible: bool,
    pub font_size: f32,
    pub font_family: &'a str,
    pub bg: tiny_skia::Color,
    pub fg: tiny_skia::Color,
    pub placeholder_fg: tiny_skia::Color,
    pub caret_color: tiny_skia::Color,
    pub border_color: tiny_skia::Color,
    pub border_radius: f32,
    pub focused: bool,
    pub ring_color: tiny_skia::Color,
    pub ring_width: f32,
    pub size: Size,
    pub prefix_icon: Option<&'a Pixmap>,
    pub suffix_icon: Option<&'a Pixmap>,
    pub cleanable: bool,
    pub loading: bool,
    pub disabled: bool,
    pub shadow: bool,
    pub selection: Option<(usize, usize)>,
    pub selection_bg: tiny_skia::Color,
    pub gap: f32,
}

impl<'a> Default for InputProps<'a> {
    fn default() -> Self {
        Self {
            value: "",
            cursor_at: 0,
            placeholder: "",
            cursor_visible: true,
            font_size: 14.0,
            font_family: "sans-serif",
            bg: tiny_skia::Color::from_rgba8(0, 0, 0, 0),
            fg: tiny_skia::Color::from_rgba8(250, 250, 250, 255),
            placeholder_fg: tiny_skia::Color::from_rgba8(163, 163, 163, 255),
            caret_color: tiny_skia::Color::from_rgba8(59, 130, 246, 255),
            border_color: tiny_skia::Color::from_rgba8(47, 47, 47, 255),
            border_radius: 6.0,
            focused: true,
            ring_color: tiny_skia::Color::from_rgba8(212, 212, 212, 255),
            ring_width: 3.0,
            size: Size::Medium,
            prefix_icon: None,
            suffix_icon: None,
            cleanable: false,
            loading: false,
            disabled: false,
            shadow: false,
            selection: None,
            selection_bg: tiny_skia::Color::from_rgba8(59, 130, 246, 80),
            gap: 6.0,
        }
    }
}

/// Draw a shadcn-style input field matching GPUI-component's Input.
/// Returns the bottom y position.
pub fn input(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    props: &InputProps,
) -> f32 {
    let opacity = if props.disabled { 0.5 } else { 1.0 };

    fn apply_opacity(c: tiny_skia::Color, opacity: f32) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(
            (c.red() * 255.0) as u8,
            (c.green() * 255.0) as u8,
            (c.blue() * 255.0) as u8,
            ((c.alpha() * opacity) * 255.0) as u8,
        )
    }

    // Shadow (drawn before everything, behind the background)
    if props.shadow && !props.disabled {
        draw::fill_rounded_rect(
            pixmap, x + 1.0, y + 2.0, w, h,
            [props.border_radius; 4],
            tiny_skia::Color::from_rgba8(0, 0, 0, 30),
        );
    }

    // Background — shadcn uses bg-transparent with dark:bg-input/30
    // We use a semi-transparent fill based on border_color for dark mode.
    let is_dark = props.bg.alpha() == 0.0;
    let bg_fill = if is_dark {
        apply_opacity(props.border_color, 0.3 * opacity)
    } else {
        apply_opacity(props.bg, opacity)
    };
    if bg_fill.alpha() > 0.0 {
        draw::fill_rounded_rect(pixmap, x, y, w, h, [props.border_radius; 4], bg_fill);
    }

    // Border / Focus ring — when focused, the 3px ring REPLACES the 1px border
    // (shadcn: focus-visible:border-ring focus-visible:ring-[3px] ring-ring/50)
    if props.focused && !props.disabled {
        let r = tiny_skia::Color::from_rgba8(
            (props.ring_color.red() * 255.0) as u8,
            (props.ring_color.green() * 255.0) as u8,
            (props.ring_color.blue() * 255.0) as u8,
            (props.ring_color.alpha() * 127.0) as u8,
        );
        draw::stroke_rounded_rect(
            pixmap, x + 0.5, y + 0.5, w - 1.0, h - 1.0,
            props.border_radius, r, props.ring_width,
        );
    } else {
        let border = apply_opacity(props.border_color, opacity);
        draw::stroke_rounded_rect(pixmap, x + 0.5, y + 0.5, w - 1.0, h - 1.0, props.border_radius, border, 1.0);
    }

    // Text metrics
    let padding_x = props.size.padding_x();
    let text_line_h = props.size.line_height();
    let cursor_h = text_line_h * 0.85;
    let cursor_w = 2.0;
    let text_y = y + (h - props.font_size) / 2.0;
    let cursor_y = y + (h - cursor_h) / 2.0;
    let fg = apply_opacity(props.fg, opacity);
    let placeholder_fg = apply_opacity(props.placeholder_fg, opacity);

    // Prefix icon
    let mut text_x = x + padding_x;
    let icon_size = (props.font_size * 1.2).round();
    if let Some(icon) = props.prefix_icon {
        let icon_y = y + (h - icon_size) / 2.0;
        draw::draw_pixmap_clipped(pixmap, icon, text_x, icon_y, y, y + h);
        text_x += icon_size + props.gap;
    }

    let max_text_w = w - (text_x - x) - padding_x
        - if props.suffix_icon.is_some() || props.cleanable || props.loading {
            icon_size + props.gap + padding_x
        } else {
            0.0
        };

    // Selection highlight
    if let Some((sel_start, sel_end)) = props.selection {
        if sel_start < sel_end {
            let before_sel = &props.value[..sel_start];
            let sel_text = &props.value[sel_start..sel_end];
            let sel_x = text_x + draw::text_width(font_system, before_sel, props.font_size, props.font_family);
            let sel_w = draw::text_width(font_system, sel_text, props.font_size, props.font_family);
            draw::fill_rect(pixmap, sel_x, y + 2.0, sel_w, h - 4.0, props.selection_bg);
        }
    }

    // Text or placeholder
    if props.value.is_empty() {
        draw::draw_text_clipped(
            pixmap, font_system, swash_cache,
            text_x, text_y, props.placeholder, props.font_size, props.font_family,
            placeholder_fg, y, y + h, max_text_w,
        );
        if props.cursor_visible && !props.disabled {
            draw::fill_rect(pixmap, text_x, cursor_y, cursor_w, cursor_h, props.caret_color);
        }
    } else {
        draw::draw_text_clipped(
            pixmap, font_system, swash_cache,
            text_x, text_y, props.value, props.font_size, props.font_family,
            fg, y, y + h, max_text_w,
        );
        if props.cursor_visible && !props.disabled {
            let before: String = props.value
                .grapheme_indices(true)
                .take(props.cursor_at)
                .map(|(_, s)| s)
                .collect();
            let cursor_offset = draw::text_width(font_system, &before, props.font_size, props.font_family);
            let mut cx = text_x + cursor_offset;
            // Clamp cursor to not go beyond right edge of input (matches GPUI)
            let right_limit = x + w - padding_x - cursor_w;
            if cx > right_limit {
                cx = right_limit;
            }
            draw::fill_rect(pixmap, cx, cursor_y, cursor_w, cursor_h, props.caret_color);
        }
    }

    // Suffix area (clean button, loading spinner, suffix icon)
    let mut suffix_x = x + w - padding_x;
    let suffix_items = props.suffix_icon.is_some() as u8
        + props.cleanable as u8
        + props.loading as u8;
    if suffix_items > 0 {
        // Draw from right to left
        // Suffix icon (rightmost)
        if let Some(icon) = props.suffix_icon {
            suffix_x -= icon_size;
            let icon_y = y + (h - icon_size) / 2.0;
            draw::draw_pixmap_clipped(pixmap, icon, suffix_x, icon_y, y, y + h);
            suffix_x -= props.gap;
        }

        // Clean button
        if props.cleanable && !props.value.is_empty() && !props.disabled {
            suffix_x -= icon_size;
            let icon_y = y + (h - icon_size) / 2.0;
            // Draw a small circle X for clean
            let cx = suffix_x + icon_size / 2.0;
            let cy = icon_y + icon_size / 2.0;
            let r = icon_size / 2.0 - 1.0;
            draw::fill_rounded_rect(
                pixmap, cx - r, cy - r, r * 2.0, r * 2.0,
                [r.max(1.0); 4],
                tiny_skia::Color::from_rgba8(163, 163, 163, 200),
            );
            // X lines
            let cross_offset = r * 0.4;
            let cross_color = tiny_skia::Color::from_rgba8(10, 10, 10, 255);
            draw::fill_rect(pixmap, cx - cross_offset, cy - cross_offset, 2.0, 2.0, cross_color);
            suffix_x -= props.gap;
        }

        // Loading spinner (dots)
        if props.loading {
            suffix_x -= icon_size;
            let dot_y = y + h / 2.0;
            let dot_r = 2.0;
            let dot_color = tiny_skia::Color::from_rgba8(163, 163, 163, 200);
            for i in 0..3 {
                let dx = suffix_x + icon_size / 2.0 + (i as f32 - 1.0) * 6.0;
                draw::fill_rounded_rect(pixmap, dx - dot_r, dot_y - dot_r, dot_r * 2.0, dot_r * 2.0, [dot_r; 4], dot_color);
            }
        }

    }

    y + h
}
