use tiny_skia::Pixmap;
use wisp::draw;

#[derive(Debug, Clone)]
pub struct ScrollBarColors {
    pub track: tiny_skia::Color,
    pub thumb: tiny_skia::Color,
}

impl Default for ScrollBarColors {
    fn default() -> Self {
        Self {
            track: tiny_skia::Color::from_rgba8(49, 50, 68, 0),
            thumb: tiny_skia::Color::from_rgba8(69, 71, 90, 255),
        }
    }
}

/// Draw a scrollbar thumb. Caller provides the visible ratio and offset.
/// `visible_ratio` = viewport_height / total_content_height (0.0 to 1.0)
/// `scroll_offset` = current scroll offset in pixels
/// `content_height` = total content height in pixels
pub fn scrollbar(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    viewport_h: f32,
    visible_ratio: f32,
    scroll_offset: f32,
    content_height: f32,
    thumb_width: f32,
    colors: &ScrollBarColors,
) {
    if visible_ratio >= 1.0 || content_height <= 0.0 {
        return;
    }

    let thumb_h = (viewport_h * visible_ratio).max(8.0);
    let max_scroll = content_height - viewport_h * visible_ratio;
    let scroll_ratio = if max_scroll > 0.0 {
        (scroll_offset / max_scroll).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let thumb_y = y + scroll_ratio * (viewport_h - thumb_h);
    let thumb_x = x + (thumb_width - 4.0) / 2.0;

    draw::fill_rounded_rect(pixmap, thumb_x, thumb_y, 4.0, thumb_h, [2.0; 4], colors.thumb);
}
