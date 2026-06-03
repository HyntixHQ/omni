use tiny_skia::{Color, Pixmap};
use wisp_components::list_view::{self, ListColors, ListItem, ListState};

pub const CLIPBOARD_WIDTH: i32 = 400;
pub const CLIPBOARD_HEIGHT: i32 = 340;
const MAX_ENTRIES: usize = 100;

#[derive(Clone, Debug)]
pub struct ClipboardEntry {
    pub text: String,
}

pub struct ClipboardState {
    pub entries: Vec<ClipboardEntry>,
    pub filter: String,
    pub list_state: ListState,
}

impl ClipboardState {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            filter: String::new(),
            list_state: ListState::new(),
        }
    }

    pub fn with_entries(entries: Vec<ClipboardEntry>) -> Self {
        Self {
            entries,
            filter: String::new(),
            list_state: ListState::new(),
        }
    }

    pub fn push(&mut self, text: String) {
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.entries.first() {
            if last.text == text {
                return;
            }
        }
        self.entries.insert(0, ClipboardEntry { text });
        if self.entries.len() > MAX_ENTRIES {
            self.entries.truncate(MAX_ENTRIES);
        }
    }

    pub fn filtered(&self) -> Vec<ClipboardEntry> {
        if self.filter.is_empty() {
            self.entries.clone()
        } else {
            let q = self.filter.to_lowercase();
            self.entries
                .iter()
                .filter(|e| e.text.to_lowercase().contains(&q))
                .cloned()
                .collect()
        }
    }

    pub fn selected(&self) -> Option<String> {
        let items = self.filtered();
        if items.is_empty() || self.list_state.selected_index >= items.len() {
            return None;
        }
        Some(items[self.list_state.selected_index].text.clone())
    }

    pub fn select_next(&mut self) {
        let items = self.filtered();
        if items.is_empty() {
            return;
        }
        self.list_state.select_next(items.len());
    }

    pub fn select_prev(&mut self) {
        self.list_state.select_prev();
    }

    pub fn set_filter(&mut self, text: &str) {
        self.filter = text.to_string();
        self.list_state.selected_index = 0;
    }

    pub fn append_char(&mut self, ch: char) {
        self.filter.push(ch);
        self.list_state.selected_index = 0;
    }

    pub fn backspace(&mut self) {
        self.filter.pop();
        self.list_state.selected_index = 0;
    }
}

/// Draw clipboard list into pixmap.
pub fn draw_clipboard(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    bg: Color,
    list_colors: &ListColors,
    entry_bg: Color,
    border: Color,
    font_size: f32,
    placeholder_fg: Color,
    caret: Color,
    font_family: &str,
    window_radius: f32,
    state: &mut ClipboardState,
    cursor_visible: bool,
    mouse_y: f32,
) {
    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;
    let pad = 10.0;
    let _header_h = 42.0;
    let gap = 8.0;
    let footer_h = 28.0;
    let inner_w = w - pad * 2.0;

    if window_radius > 0.0 {
        wisp::draw::fill_rounded_rect(pixmap, 0.0, 0.0, w, h, [window_radius; 4], bg);
    } else {
        pixmap.fill(bg);
    }

    // Search input
    wisp_components::input::input(
        pixmap, font_system, swash_cache, pad, 6.0, inner_w, 42.0,
        &wisp_components::input::InputProps {
            value: &state.filter,
            cursor_at: state.filter.len(),
            placeholder: "Search clipboard...",
            cursor_visible,
            font_size,
            font_family,
            bg: entry_bg,
            fg: list_colors.fg,
            placeholder_fg,
            caret_color: caret,
            border_color: border,
            border_radius: 6.0,
            focused: true,
            ring_color: border,
            ring_width: 3.0,
            size: wisp::Size::Medium,
            prefix_icon: None,
            suffix_icon: None,
            cleanable: false,
            loading: false,
            disabled: false,
            shadow: false,
            selection: None,
            selection_bg: Color::from_rgba8(59, 130, 246, 80),
            gap: 6.0,
        },
    );

    // List
    let list_y = 6.0 + 42.0 + gap;
    let list_h = h - list_y - footer_h - pad;
    let row_h = list_view::row_height_compact(font_system, font_size, font_family);
    let filtered = state.filtered();
    let list_len = filtered.len();
    let list_state = &mut state.list_state;
    if list_state.selected_index >= list_len.saturating_sub(1) {
        list_state.selected_index = list_len.saturating_sub(1);
    }
    let sel = list_state.selected_index;
    let list_items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, e)| ListItem {
            title: e.text.as_str(),
            subtitle: None,
            prefix_icon: None,
            suffix_icon: None,
            selected: i == sel,
            disabled: false,
        })
        .collect();

    list_state.ensure_selected_visible(row_h);

    list_view::draw_list(
        pixmap, font_system, swash_cache,
        pad, list_y, inner_w, list_h,
        mouse_y,
        &list_items,
        &mut state.list_state,
        font_size,
        font_family,
        list_colors,
    );

    // Footer hint
    let hint_y = h - footer_h + 4.0;
    wisp_components::badge::badge(
        pixmap, font_system, swash_cache, pad, hint_y,
        &wisp_components::badge::BadgeProps {
            text: "Esc",
            variant: wisp_components::badge::BadgeVariant::Secondary,
            ..Default::default()
        },
        font_family,
    );
    let badge_w = wisp_components::badge::badge_size("Esc", font_system, font_family);
    wisp::draw::draw_text(
        pixmap, font_system, swash_cache,
        "to close", pad + badge_w + 6.0, hint_y + 4.0, 11.0, font_family, list_colors.desc_fg,
    );

    // Enter badge
    let enter_w = wisp_components::badge::badge_size("Enter", font_system, font_family);
    let enter_x = w - pad - enter_w;
    wisp_components::badge::badge(
        pixmap, font_system, swash_cache, enter_x, hint_y,
        &wisp_components::badge::BadgeProps {
            text: "Enter",
            variant: wisp_components::badge::BadgeVariant::Secondary,
            ..Default::default()
        },
        font_family,
    );
    let confirm_label_x = enter_x - 6.0 - wisp::draw::text_width(font_system, "to paste", 11.0, font_family);
    wisp::draw::draw_text(
        pixmap, font_system, swash_cache,
        "to paste", confirm_label_x, hint_y + 4.0, 11.0, font_family, list_colors.desc_fg,
    );
}
