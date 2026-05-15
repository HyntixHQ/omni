use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Paint, Pixmap};
use crate::render::{draw_pixmap_within, fill_rect_within, get_text_metrics, hex_color, render_text_within, sk_paint};
use crate::config::Config;

pub struct ListItem<'a> {
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub prefix_icon: Option<&'a Pixmap>,
    pub suffix_icon: Option<&'a Pixmap>,
    pub selected: bool,
}

#[derive(Default, Debug, Clone)]
pub struct ListViewState {
    pub scroll_offset: f32,
    pub hover_y: Option<f32>,
}

impl ListViewState {
    pub fn ensure_visible(&mut self, selected_index: usize, row_height: f32, visible_height: f32, max_items: usize) {
        let max_scroll = (max_items as f32 * row_height - visible_height).max(0.0);
        let current_row = (self.scroll_offset / row_height).round() as isize;
        let visible_rows = (visible_height / row_height).floor() as isize;
        let last_visible_row = current_row + visible_rows - 1;
        let sel = selected_index as isize;

        if sel < current_row {
            self.scroll_offset = (sel as f32 * row_height).round().min(max_scroll);
        } else if sel > last_visible_row {
            let new_current = sel - visible_rows + 1;
            self.scroll_offset = (new_current as f32 * row_height).round().max(0.0).min(max_scroll);
        }
    }

    pub fn scroll_by(&mut self, delta: f32, max_items: usize, row_height: f32, visible_height: f32) {
        let max_scroll = (max_items as f32 * row_height - visible_height).max(0.0);
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_scroll).round();
    }
}

pub fn draw_list_view(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    items: &[ListItem],
    state: &mut ListViewState,
    cfg: &Config,
) {
    let font_size = cfg.font.size as f32;
    let desc_size = font_size - 2.0;
    let family = &cfg.font.family;
    let icon_size = 28.0;

    // 1. Calculate Intrinsic Row Height
    let (n_ascent, n_descent) = get_text_metrics(font_system, "Ag", font_size, family);
    let (d_ascent, d_descent) = get_text_metrics(font_system, "Ag", desc_size, family);
    
    let name_h = n_ascent + n_descent;
    let desc_h = d_ascent + d_descent;
    
    let row_content_h = name_h + desc_h + 2.0;
    let row_height = row_content_h + 12.0; // 6px padding top/bottom

    let max_scroll = (items.len() as f32 * row_height - h).max(0.0);
    state.scroll_offset = state.scroll_offset.clamp(0.0, max_scroll);

    let start_idx = (state.scroll_offset / row_height).floor() as usize;
    let end_idx = ((state.scroll_offset + h) / row_height).ceil() as usize;
    let end_idx = end_idx.min(items.len());

    for i in start_idx..end_idx {
        let item = &items[i];
        let row_y = (y + (i as f32 * row_height) - state.scroll_offset).round();
        let row_h = row_height - 4.0;

        if row_y + row_h < y || row_y > y + h {
            continue;
        }

        let is_hovered = if let Some(hy) = state.hover_y {
            hy >= row_y && hy <= row_y + row_h
        } else {
            false
        };

        if item.selected || is_hovered {
            let bg_color = if item.selected {
                sk_paint(&cfg.theme.selected_bg)
            } else {
                let [r, g, b, _] = hex_color(&cfg.theme.selected_bg);
                let mut p = Paint::default();
                p.set_color(tiny_skia::Color::from_rgba8(r, g, b, 50));
                p.anti_alias = true;
                p
            };

            fill_rect_within(pixmap, x, row_y, w, row_h, 6.0, &bg_color, y, y + h);
        }

        let name_fg = if item.selected {
            hex_color(&cfg.theme.selected_fg)
        } else {
            hex_color(&cfg.theme.fg)
        };

        let icon_y = row_y + ((row_h - icon_size) / 2.0).round();
        let icon_x = x + 6.0;

        if let Some(icon_pixmap) = item.prefix_icon {
            draw_pixmap_within(pixmap, icon_pixmap, icon_x, icon_y, y, y + h);
        }

        let text_x = x + 6.0 + icon_size + 10.0;
        let max_text_w = w - (text_x - x) - 6.0;
        let text_y = (row_y + (row_h - row_content_h) / 2.0).round();

        render_text_within(
            pixmap, font_system, swash_cache,
            text_x, text_y, item.title, font_size, family, name_fg,
            y, y + h, max_text_w,
        );

        if let Some(desc) = item.subtitle {
            let desc_y = (text_y + name_h + 2.0).round();
            let desc_fg = if item.selected {
                hex_color(&cfg.theme.selected_fg)
            } else {
                hex_color(&cfg.theme.desc_fg)
            };
            render_text_within(
                pixmap, font_system, swash_cache,
                text_x, desc_y, desc, desc_size, family, desc_fg,
                y, y + h, max_text_w,
            );
        }

        if let Some(suffix_icon) = item.suffix_icon {
            let sx = (x + w - 6.0 - icon_size).round();
            draw_pixmap_within(pixmap, suffix_icon, sx, icon_y, y, y + h);
        }
    }
}
