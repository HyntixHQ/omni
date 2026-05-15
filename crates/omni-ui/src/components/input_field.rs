use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{FillRule, Pixmap, Transform};

use crate::config::Config;
use crate::render::{get_text_metrics, hex_color, measure_text_width, render_text, rounded_rect_path, sk_paint};
use crate::state::OmniApp;

const INPUT_PADDING_X: f32 = 10.0;
const CURSOR_WIDTH: f32 = 2.0;

pub fn draw_input_field(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    app: &OmniApp,
    cfg: &Config,
    cursor_visible: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> f32 {
    let input_bottom = y + h;
    let border_r = 8.0;

    let query = &app.query;
    let cursor_at = app.cursor_at;

    // Restore background box for the search bar
    let bg_path = rounded_rect_path(x, y, w, h, border_r);
    pixmap.fill_path(
        &bg_path,
        &sk_paint(&cfg.theme.entry_bg),
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    // Text area
    let text_x = x + INPUT_PADDING_X;
    let font_size = cfg.font.entry_size as f32;
    let family = &cfg.font.family;
    
    // Get actual font metrics for precise centering
    let (ascent, descent) = get_text_metrics(font_system, if query.is_empty() { "A" } else { query }, font_size, family);
    let text_h = ascent + descent;
    
    // text_y is the top of the typographic box (where 'y' in render_text starts)
    let text_y = y + (h - text_h) / 2.0;

    let max_w = w - INPUT_PADDING_X * 2.0;

    if query.is_empty() {
        render_text(
            pixmap,
            font_system,
            swash_cache,
            text_x,
            text_y,
            "Search apps...",
            font_size,
            family,
            hex_color(&cfg.theme.placeholder_fg),
            max_w,
        );
        
        // Render cursor even when empty if focused (Omni is always focused)
        if cursor_visible {
            let cx = text_x;
            draw_cursor(pixmap, cx, y, h, text_h, &cfg.theme.caret);
        }
    } else {
        render_text(
            pixmap,
            font_system,
            swash_cache,
            text_x,
            text_y,
            query,
            font_size,
            family,
            hex_color(&cfg.theme.fg),
            max_w,
        );

        // Calculate cursor position based on characters before it
        if cursor_visible {
            let before_cursor: String = query.chars().take(cursor_at).collect();
            let cursor_offset = measure_text_width(font_system, &before_cursor, font_size, family, max_w, h);
            let cx = text_x + cursor_offset;
            draw_cursor(pixmap, cx, y, h, text_h, &cfg.theme.caret);
        }
    }

    input_bottom
}

fn draw_cursor(pixmap: &mut Pixmap, x: f32, y: f32, container_h: f32, cursor_height: f32, color: &str) {
    let cy = y + (container_h - cursor_height) / 2.0;
    let cursor_rect = tiny_skia::Rect::from_xywh(x, cy, CURSOR_WIDTH, cursor_height)
        .unwrap_or(tiny_skia::Rect::from_xywh(x, cy, 1.0, cursor_height).unwrap());
    let cursor_path = tiny_skia::PathBuilder::from_rect(cursor_rect);
    pixmap.fill_path(
        &cursor_path,
        &sk_paint(color),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}
