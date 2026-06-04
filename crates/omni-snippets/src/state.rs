use cosmic_text::SwashCache;
use tiny_skia::{Color, Pixmap};
use wisp::draw::{self, color_from_hex};
use wisp::events::MouseDispatcher;
use wisp::input::InputAction;
use wisp_components::badge::{badge, BadgeColors, BadgeProps, BadgeVariant};
use wisp_components::input;
use wisp_components::list_view::{draw_list, row_height, ListColors, ListItem, ListState};

use crate::snippet::{expand_placeholders, Snippet};

pub const SNIPPETS_WIDTH: i32 = 600;
pub const SNIPPETS_HEIGHT: i32 = 400;

pub struct SnippetsState {
    pub list_state: ListState,
    pub entries: Vec<Snippet>,
    pub all_entries: Vec<Snippet>,
    pub filter: String,
    pub last_clipboard: Option<String>,
    pub mouse: MouseDispatcher,
}

impl SnippetsState {
    pub fn new(snippets: Vec<Snippet>, last_clipboard: Option<String>) -> Self {
        Self {
            list_state: ListState::new(),
            entries: snippets.clone(),
            all_entries: snippets,
            filter: String::new(),
            last_clipboard,
            mouse: MouseDispatcher::new(),
        }
    }

    pub fn filtered(&self) -> Vec<Snippet> {
        if self.filter.is_empty() {
            return self.all_entries.clone();
        }
        let q = self.filter.to_lowercase();
        self.all_entries
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.content.to_lowercase().contains(&q)
                    || s.shortcut
                        .as_ref()
                        .map_or(false, |sc| sc.to_lowercase().contains(&q))
            })
            .cloned()
            .collect()
    }

    pub fn selected(&self) -> Option<Snippet> {
        let items = self.filtered();
        if items.is_empty() || self.list_state.selected_index >= items.len() {
            return None;
        }
        Some(items[self.list_state.selected_index].clone())
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

    pub fn append_char(&mut self, ch: char) {
        self.filter.push(ch);
        self.list_state.selected_index = 0;
    }

    pub fn backspace(&mut self) {
        self.filter.pop();
        self.list_state.selected_index = 0;
    }

    pub fn expand(&self, snippet: &Snippet) -> String {
        expand_placeholders(&snippet.content, self.last_clipboard.as_deref())
    }

    /// Returns Some(snippet) if the current filter exactly matches a snippet's
    /// shortcut — caller should immediately copy & close.
    pub fn exact_shortcut_match(&self) -> Option<Snippet> {
        if self.filter.is_empty() {
            return None;
        }
        self.all_entries
            .iter()
            .find(|s| {
                s.shortcut
                    .as_deref()
                    .map_or(false, |sc| sc == self.filter.as_str())
            })
            .cloned()
    }
}

pub fn handle_action(state: &mut SnippetsState, action: InputAction, _row_height: f32, _body_h: f32) {
    match action {
        InputAction::SelectNext => state.select_next(),
        InputAction::SelectPrev => state.select_prev(),
        InputAction::AppendChar(ch) => state.append_char(ch),
        InputAction::Backspace => state.backspace(),
        _ => {}
    }
}

pub fn compute_row_height(font_system: &mut cosmic_text::FontSystem, font_size: f32, font_family: &str) -> f32 {
    row_height(font_system, font_size, font_family)
}

pub fn draw_snippets(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    bg: Color,
    fg: Color,
    selected_bg: Color,
    dim_fg: Color,
    desc_fg: Color,
    accent: Color,
    border: Color,
    placeholder_fg: Color,
    caret: Color,
    font_size: f32,
    font_family: &str,
    window_radius: f32,
    state: &mut SnippetsState,
    cursor_visible: bool,
    mouse_y: f32,
) {
    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;
    let pad = 10.0;
    let header_h = 42.0;
    let gap = 8.0;
    let footer_h = 28.0;
    let inner_w = w - pad * 2.0;

    if window_radius > 0.0 {
        draw::fill_rounded_rect(pixmap, 0.0, 0.0, w, h, [window_radius; 4], bg);
    } else {
        pixmap.fill(bg);
    }

    input::input(
        pixmap, font_system, swash_cache, pad, 6.0, inner_w, header_h,
        &input::InputProps {
            value: &state.filter,
            cursor_at: state.filter.len(),
            placeholder: "Search snippets (type ;shortcut to instantly copy)...",
            cursor_visible,
            font_size,
            font_family,
            bg: color_from_hex("#1a1a1a"),
            fg,
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

    let list_y = 6.0 + header_h + gap;
    let list_h = h - list_y - footer_h - pad;

    state.mouse.clear();
    state.mouse.register_body(list_y, list_h);

    let filtered = state.filtered();
    let list_len = filtered.len();
    if state.list_state.selected_index >= list_len.saturating_sub(1) {
        state.list_state.selected_index = list_len.saturating_sub(1);
    }
    let sel = state.list_state.selected_index;
    let row_h = compute_row_height(font_system, font_size, font_family);
    state.list_state.layout(list_h, list_len, row_h);
    state.list_state.ensure_selected_visible(row_h);

    let list_colors = ListColors {
        bg: Color::from_rgba8(0, 0, 0, 0),
        hover: Color::from_rgba8(255, 255, 255, 20),
        active_bg: selected_bg,
        active_border: border,
        active_highlight: true,
        fg,
        selected_fg: fg,
        desc_fg,
        selected_desc_fg: fg,
    };

    let previews: Vec<String> = filtered.iter().map(|s| s.preview(80)).collect();
    let items: Vec<ListItem> = filtered
        .iter()
        .zip(previews.iter())
        .map(|(s, preview)| ListItem {
            title: &s.name,
            subtitle: Some(preview.as_str()),
            prefix_icon: None,
            suffix_icon: None,
            selected: false,
            disabled: false,
        })
        .collect();
    let _ = sel;

    draw_list(
        pixmap, font_system, swash_cache,
        pad, list_y, inner_w, list_h,
        mouse_y, &items, &mut state.list_state,
        font_size, font_family, &list_colors,
    );

    let _ = (accent, dim_fg);
    let hint_y = h - footer_h + 4.0;
    let mut badge_x = pad;

    if let Some(item) = state.selected() {
        if let Some(ref sc) = item.shortcut {
            let badge_colors = BadgeColors {
                bg: Color::from_rgba8(0, 0, 0, 0),
                fg: desc_fg,
                border: Some(border),
            };
            let (bw, _) = badge(
                pixmap, font_system, swash_cache,
                badge_x, hint_y + 2.0,
                &BadgeProps {
                    text: sc,
                    variant: BadgeVariant::Outline,
                    focused: false,
                    colors: Some(badge_colors),
                    icon: None,
                },
                font_family,
            );
            badge_x += bw + 6.0;
        }
    }

    let hint_text = format!("{} snippets", list_len);
    let hint_w = draw::text_width(font_system, &hint_text, 11.0, font_family);
    draw::draw_text(
        pixmap, font_system, swash_cache,
        &hint_text, w - pad - hint_w, hint_y + 4.0, 11.0, font_family, desc_fg,
    );

    let _ = badge_x;
}
