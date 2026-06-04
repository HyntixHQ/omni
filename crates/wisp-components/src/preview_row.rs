use tiny_skia::Color;
use tiny_skia::Pixmap;
use wisp::draw;

use crate::list_view::{ListColors, ListState};
use crate::wm_preview::{draw_preview, PreviewColors, PreviewSpec};

/// A list row with an optional mini-screen preview on the right.
/// Same layout as `ListItem` but with a `preview` slot for a `PreviewSpec`.
/// Matches shadcn CommandItem / GPUI-component ListItem.
pub struct PreviewRow<'a> {
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub preview: Option<PreviewSpec>,
    pub selected: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PreviewRowSize {
    pub preview_w: f32,
    pub preview_h: f32,
    pub preview_gap: f32,
}

impl Default for PreviewRowSize {
    fn default() -> Self {
        Self {
            preview_w: 60.0,
            preview_h: 40.0,
            preview_gap: 10.0,
        }
    }
}

/// Draw a list of `PreviewRow`s. Reuses `ListState` for scroll/hover/select/click,
/// so apps can use `state.index_at(y, row_h)` etc. just like `draw_list`.
pub fn draw_preview_rows(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    mouse_y: f32,
    items: &[PreviewRow],
    state: &mut ListState,
    font_size: f32,
    font_family: &str,
    colors: &ListColors,
    preview_size: PreviewRowSize,
    preview_colors: &PreviewColors,
) {
    let desc_size = font_size - 2.0;
    let padding_x = 8.0;
    let row_h = preview_row_height(font_system, font_size, font_family);

    let (n_ascent, n_descent) = draw::text_metrics(font_system, "Ag", font_size, font_family);
    let (d_ascent, d_descent) = draw::text_metrics(font_system, "Ag", desc_size, font_family);
    let name_h = n_ascent + n_descent;
    let desc_h = d_ascent + d_descent;
    let row_content_h = name_h + desc_h + 2.0;

    state.layout(h, items.len(), row_h);

    if mouse_y >= y && mouse_y <= y + h {
        state.set_hover(mouse_y - y, row_h);
    } else {
        state.hover_index = None;
    }

    let scroll_y = -state.scroll.offset();
    let scroll_top = scroll_y.floor();
    let start_idx = if scroll_top > 0.0 {
        (scroll_top / row_h) as usize
    } else {
        0
    };
    let end_idx = ((scroll_y + h) / row_h).ceil() as usize;
    let end_idx = end_idx.min(items.len());

    let text_max_w = w - preview_size.preview_w - preview_size.preview_gap - padding_x * 2.0;
    let name_y_offset = (row_h - row_content_h) / 2.0;

    for i in start_idx..end_idx {
        let item = &items[i];
        let row_y = (y + (i as f32 * row_h) - scroll_y).round();
        let row_h_actual = row_h - 4.0;
        let row_bot = row_y + row_h_actual;
        if row_bot < y || row_y > y + h {
            continue;
        }

        let is_hovered = state.hover_index == Some(i);
        let is_active = item.selected && colors.active_highlight;

        if item.selected || is_hovered {
            let bg = if item.selected { colors.active_bg } else { colors.hover };
            draw::fill_rect_clipped(pixmap, x, row_y, w, row_h_actual, 4.0, bg, row_y, row_y + row_h_actual);
        }

        if is_active && row_y >= y && row_bot <= y + h {
            draw::stroke_rounded_rect(pixmap, x + 0.5, row_y + 0.5, w - 1.0, row_h_actual - 1.0, 4.0, colors.active_border, 1.0);
        }

        let name_color = if item.selected { colors.selected_fg } else { colors.fg };
        let desc_color = if item.selected { colors.selected_desc_fg } else { colors.desc_fg };
        let name_color = if item.disabled { colors.desc_fg } else { name_color };
        let desc_color = if item.disabled { colors.desc_fg } else { desc_color };
        let preview_color = if item.disabled {
            Color::from_rgba8(100, 100, 100, 120)
        } else if item.selected {
            colors.active_border
        } else {
            preview_colors.accent
        };
        let final_preview_colors = PreviewColors {
            accent: preview_color,
            outline: if item.disabled { Color::from_rgba8(60, 60, 60, 200) } else { preview_colors.outline },
        };

        let name_y = row_y + name_y_offset;
        draw::draw_text_clipped(
            pixmap, font_system, swash_cache,
            x + padding_x, name_y, item.title, font_size, font_family, name_color,
            name_y, name_y + name_h, text_max_w,
        );

        if let Some(desc) = item.subtitle {
            let desc_y = (name_y + name_h + 2.0).round();
            draw::draw_text_clipped(
                pixmap, font_system, swash_cache,
                x + padding_x, desc_y, desc, desc_size, font_family, desc_color,
                desc_y, desc_y + desc_h, text_max_w,
            );
        }

        if let Some(preview) = item.preview {
            let px = (x + w - padding_x - preview_size.preview_w).round();
            let py = (row_y + (row_h_actual - preview_size.preview_h) / 2.0).round();
            draw_preview(pixmap, px, py, preview_size.preview_w, preview_size.preview_h, preview, &final_preview_colors);
        }
    }
}

/// Compute row height for preview rows.
pub fn preview_row_height(font_system: &mut cosmic_text::FontSystem, font_size: f32, font_family: &str) -> f32 {
    let desc_size = font_size - 2.0;
    let (n_ascent, n_descent) = draw::text_metrics(font_system, "Ag", font_size, font_family);
    let (d_ascent, d_descent) = draw::text_metrics(font_system, "Ag", desc_size, font_family);
    let name_h = n_ascent + n_descent;
    let desc_h = d_ascent + d_descent;
    let row_content_h = name_h + desc_h + 2.0;
    row_content_h + 12.0
}
