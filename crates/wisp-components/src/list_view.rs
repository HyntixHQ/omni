use tiny_skia::{Color, Pixmap};
use wisp::draw;
use wisp::scroll::ScrollHandle;

/// A list item — matches shadcn CommandItem / GPUI-component ListItem.
pub struct ListItem<'a> {
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub prefix_icon: Option<&'a Pixmap>,
    pub suffix_icon: Option<&'a Pixmap>,
    pub selected: bool,
    pub disabled: bool,
}

/// Scroll state + selection + hover for a list. Wraps `wisp::scroll::ScrollHandle`.
pub struct ListState {
    pub scroll: ScrollHandle,
    pub selected_index: usize,
    pub hover_index: Option<usize>,
}

impl ListState {
    pub fn new() -> Self {
        Self {
            scroll: ScrollHandle::new(),
            selected_index: 0,
            hover_index: None,
        }
    }

    pub fn layout(&mut self, viewport_height: f32, total_items: usize, row_height: f32) {
        let content = total_items as f32 * row_height;
        self.scroll.set_viewport(viewport_height);
        self.scroll.set_content(content);
        self.scroll.set_line_height(row_height);
    }

    pub fn ensure_selected_visible(&mut self, row_height: f32) {
        let item_top = self.selected_index as f32 * row_height;
        let item_bot = item_top + row_height;
        self.scroll.ensure_visible(item_top, item_bot);
    }

    pub fn select_next(&mut self, max: usize) {
        if max == 0 { return; }
        self.selected_index = (self.selected_index + 1).min(max - 1);
    }

    pub fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn set_hover(&mut self, y: f32, row_height: f32) {
        let scroll_y = -self.scroll.offset();
        let rel_y = y + scroll_y;
        self.hover_index = if rel_y >= 0.0 { Some((rel_y / row_height) as usize) } else { None };
    }

    pub fn clear_hover(&mut self) {
        self.hover_index = None;
    }

    pub fn index_at(&self, y: f32, row_height: f32) -> Option<usize> {
        let scroll_y = -self.scroll.offset();
        let rel_y = y + scroll_y;
        if rel_y >= 0.0 { Some((rel_y / row_height) as usize) } else { None }
    }
}

impl Default for ListState {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme colors for the list.
#[derive(Debug, Clone)]
pub struct ListColors {
    pub bg: Color,
    pub hover: Color,
    pub active_bg: Color,
    pub active_border: Color,
    pub active_highlight: bool,
    pub fg: Color,
    pub selected_fg: Color,
    pub desc_fg: Color,
    pub selected_desc_fg: Color,
}

/// Compute row height from font metrics.
pub fn row_height(font_system: &mut cosmic_text::FontSystem, font_size: f32, font_family: &str) -> f32 {
    let desc_size = font_size - 2.0;
    let (n_ascent, n_descent) = draw::text_metrics(font_system, "Ag", font_size, font_family);
    let (d_ascent, d_descent) = draw::text_metrics(font_system, "Ag", desc_size, font_family);
    n_ascent + n_descent + d_ascent + d_descent + 2.0 + 12.0
}

/// Compute compact row height for items without subtitle.
pub fn row_height_compact(font_system: &mut cosmic_text::FontSystem, font_size: f32, font_family: &str) -> f32 {
    let (n_ascent, n_descent) = draw::text_metrics(font_system, "Ag", font_size, font_family);
    let name_h = n_ascent + n_descent;
    name_h + 12.0
}

/// Draw a list — matches shadcn CommandItem / GPUI-component List spec.
/// Takes `mouse_y` for hover tracking (pass -1 to disable hover).
pub fn draw_list(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    mouse_y: f32,
    items: &[ListItem],
    state: &mut ListState,
    font_size: f32,
    font_family: &str,
    colors: &ListColors,
) {
    let desc_size = font_size - 2.0;
    let icon_size = 28.0;  // matches IconCache default render size
    let padding_x = 8.0;   // shadcn: px-2
    let gap = 8.0;         // shadcn: gap-2

    let (n_ascent, n_descent) = draw::text_metrics(font_system, "Ag", font_size, font_family);
    let (d_ascent, d_descent) = draw::text_metrics(font_system, "Ag", desc_size, font_family);

    let name_h = n_ascent + n_descent;
    let desc_h = d_ascent + d_descent;
    let row_content_h = name_h + desc_h + 2.0;
    let row_h = row_content_h + 12.0;

    state.layout(h, items.len(), row_h);

    // Only track hover if mouse is within list bounds
    if mouse_y >= y && mouse_y <= y + h {
        state.set_hover(mouse_y - y, row_h);
    } else {
        state.hover_index = None;
    }

    let scroll_y = -state.scroll.offset();
    let scroll_top = scroll_y.floor();

    // Include partially visible items at both edges (ceil at bottom, floor at top)
    let start_idx = if scroll_top > 0.0 {
        (scroll_top / row_h) as usize
    } else {
        0
    };
    let end_idx = ((scroll_y + h) / row_h).ceil() as usize;
    let end_idx = end_idx.min(items.len());

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

        // Items clipped to per-row bounds — background, text, icons all clip individually
        if item.selected || is_hovered {
            let bg = if item.selected { colors.active_bg } else { colors.hover };
            draw::fill_rect_clipped(pixmap, x, row_y, w, row_h_actual, 4.0, bg, row_y, row_y + row_h_actual);
        }

        // Border — only when fully visible (no partial border overflow)
        if is_active && row_y >= y && row_bot <= y + h {
            draw::stroke_rounded_rect(pixmap, x + 0.5, row_y + 0.5, w - 1.0, row_h_actual - 1.0, 4.0, colors.active_border, 1.0);
        }

        let name_color = if item.selected { colors.selected_fg } else { colors.fg };

        let has_prefix = item.prefix_icon.is_some();
        let has_suffix = item.suffix_icon.is_some();

        // Icon
        let icon_y = row_y + ((row_h - 4.0 - icon_size) / 2.0).round();
        let icon_x = x + padding_x;

        if let Some(icon) = item.prefix_icon {
            draw::draw_pixmap_clipped(pixmap, icon, icon_x, icon_y, row_y, row_y + row_h_actual);
        }

        let text_x = if has_prefix { x + padding_x + icon_size + gap } else { x + padding_x };
        let max_text_w = if has_suffix { w - (text_x - x) - padding_x - icon_size - gap } else { w - (text_x - x) - padding_x };
        let content_h = if item.subtitle.is_some() { row_content_h } else { name_h };
        let text_y = (row_y + (row_h - 4.0 - content_h) / 2.0).round();

        // Title — clip to name line area only (prevents wrap bleed into subtitle space)
        draw::draw_text_clipped(
            pixmap, font_system, swash_cache,
            text_x, text_y, item.title, font_size, font_family, name_color,
            text_y, text_y + name_h, max_text_w,
        );

        // Subtitle
        if let Some(desc) = item.subtitle {
            let desc_y = (text_y + name_h + 2.0).round();
            let desc_color = if item.selected { colors.selected_desc_fg } else { colors.desc_fg };
            // Subtitle — clip to desc line area only
            draw::draw_text_clipped(
                pixmap, font_system, swash_cache,
                text_x, desc_y, desc, desc_size, font_family, desc_color,
                desc_y, desc_y + desc_h, max_text_w,
            );
        }

        // Suffix icon
        if let Some(suffix) = item.suffix_icon {
            let sx = (x + w - padding_x - icon_size).round();
            draw::draw_pixmap_clipped(pixmap, suffix, sx, icon_y, row_y, row_y + row_h_actual);
        }
    }
}
