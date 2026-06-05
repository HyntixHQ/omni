use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Color, Pixmap};
use wisp::draw::{self, color_from_hex};
use wisp::events::MouseDispatcher;
use wisp::input::InputAction;
use wisp_components::badge::{badge, BadgeProps, BadgeVariant};
use wisp_components::input;
use wisp_components::kbd_combo::{kbd_combo, measure_kbd_combo, KbdComboProps};
use wisp_components::list_view::ListState;
use wisp_components::Icon;

use crate::data::{ShortcutCategory, ShortcutEntry};

pub const HELP_WIDTH: i32 = 600;
pub const HELP_HEADER_H: f32 = 42.0;
pub const HELP_FOOTER_H: f32 = 28.0;
pub const HELP_PAD: f32 = 10.0;
pub const HELP_SECTION_HEADER_H: f32 = 24.0;
pub const HELP_ROW_H: f32 = 26.0;
pub const HELP_MIN_HEIGHT: f32 = 200.0;
pub const HELP_MAX_HEIGHT: f32 = 560.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Header,
    Entry,
}

#[derive(Debug, Clone)]
pub struct FlatRow {
    pub kind: RowKind,
    pub text: String,
    pub entry: Option<ShortcutEntry>,
}

pub struct HelpState {
    pub categories: Vec<ShortcutCategory>,
    pub filter: String,
    pub list_state: ListState,
    pub mouse: MouseDispatcher,
    pub all_rows: Vec<FlatRow>,
    pub visible_rows: Vec<FlatRow>,
}

impl HelpState {
    pub fn new(categories: Vec<ShortcutCategory>) -> Self {
        let mut s = Self {
            categories,
            filter: String::new(),
            list_state: ListState::new(),
            mouse: MouseDispatcher::new(),
            all_rows: Vec::new(),
            visible_rows: Vec::new(),
        };
        s.rebuild_rows();
        s
    }

    pub fn rebuild_rows(&mut self) {
        let mut rows = Vec::new();
        for cat in &self.categories {
            let entries: Vec<&ShortcutEntry> = cat
                .entries
                .iter()
                .filter(|e| e.matches(&self.filter))
                .collect();
            if entries.is_empty() {
                continue;
            }
            rows.push(FlatRow {
                kind: RowKind::Header,
                text: cat.name.clone(),
                entry: None,
            });
            for e in entries {
                rows.push(FlatRow {
                    kind: RowKind::Entry,
                    text: e.description.clone(),
                    entry: Some(e.clone()),
                });
            }
        }
        self.all_rows = rows;
        self.visible_rows = self.all_rows.clone();
    }

    pub fn set_filter(&mut self, q: impl Into<String>) {
        self.filter = q.into();
        self.list_state.selected_index = 0;
        self.rebuild_rows();
    }

    pub fn total_rows(&self) -> usize {
        self.visible_rows.len()
    }

    pub fn preferred_height(&self) -> i32 {
        let rows_h = self.visible_rows.len() as f32 * HELP_ROW_H;
        let total = HELP_PAD * 2.0 + HELP_HEADER_H + 8.0 + rows_h + HELP_FOOTER_H;
        let h = total.clamp(HELP_MIN_HEIGHT, HELP_MAX_HEIGHT);
        h.round() as i32
    }
}

pub fn compute_row_height(font_system: &mut FontSystem, font_size: f32, font_family: &str) -> f32 {
    let (ascent, descent) = draw::text_metrics(font_system, "Ag", font_size, font_family);
    (ascent + descent).max(HELP_ROW_H)
}

pub fn handle_action(state: &mut HelpState, action: InputAction) {
    match action {
        InputAction::AppendChar(ch) => {
            let mut s = state.filter.clone();
            s.push(ch);
            state.set_filter(s);
        }
        InputAction::Backspace => {
            let mut s = state.filter.clone();
            s.pop();
            state.set_filter(s);
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn help_section_icon(section: &str) -> Option<Icon> {
    Some(match section {
        "Global" => Icon::Keyboard,
        "Custom Apps" => Icon::AppWindow,
        "Snippets" => Icon::FileText,
        "General" => Icon::Info,
        _ => return None,
    })
}

pub fn draw_help(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    bg: Color,
    fg: Color,
    selected_bg: Color,
    desc_fg: Color,
    accent: Color,
    border: Color,
    placeholder_fg: Color,
    caret: Color,
    font_size: f32,
    font_family: &str,
    window_radius: f32,
    state: &mut HelpState,
    cursor_visible: bool,
    _mouse_y: f32,
) {
    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;
    let pad = HELP_PAD;
    let inner_w = w - pad * 2.0;

    if window_radius > 0.0 {
        draw::fill_rounded_rect(pixmap, 0.0, 0.0, w, h, [window_radius; 4], bg);
    } else {
        pixmap.fill(bg);
    }

    input::input(
        pixmap,
        font_system,
        swash_cache,
        pad,
        6.0,
        inner_w,
        HELP_HEADER_H,
        &input::InputProps {
            value: &state.filter,
            cursor_at: state.filter.len(),
            placeholder: "Search shortcuts (e.g. clipboard, super+alt, snippets)...",
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

    let list_y = 6.0 + HELP_HEADER_H + 8.0;
    let list_h = h - list_y - HELP_FOOTER_H - pad;
    let _ = selected_bg;

    state.mouse.clear();
    state.mouse.register_body(list_y, list_h);

    let n = state.visible_rows.len();
    let total_h = n as f32 * HELP_ROW_H;
    state.list_state.layout(list_h, n, HELP_ROW_H);
    state.list_state.ensure_selected_visible(HELP_ROW_H);

    if n == 0 {
        let empty = if state.filter.is_empty() {
            "No shortcuts available"
        } else {
            "No matching shortcuts"
        };
        let ew = draw::text_width(font_system, empty, 12.0, font_family);
        draw::draw_text(
            pixmap,
            font_system,
            swash_cache,
            empty,
            (w - ew) / 2.0,
            list_y + list_h / 2.0 - 6.0,
            12.0,
            font_family,
            desc_fg,
        );
    } else {
        let scroll_y = state.list_state.scroll.offset();
        let mut y = list_y - scroll_y;

        let first_visible = ((scroll_y / HELP_ROW_H).floor() as usize).saturating_sub(1);
        let last_visible = (((scroll_y + list_h) / HELP_ROW_H).ceil() as usize + 1).min(n);

        for i in first_visible..last_visible {
            let row = &state.visible_rows[i];
            let row_y = list_y - scroll_y + i as f32 * HELP_ROW_H;

            if row_y + HELP_ROW_H < list_y || row_y > list_y + list_h {
                continue;
            }
            y = row_y;

            match row.kind {
                RowKind::Header => {
                    if y >= list_y - 4.0 && y + HELP_SECTION_HEADER_H <= list_y + list_h + 4.0 {
                        let mut text_x = pad;
                        if let Some(icon) = help_section_icon(&row.text) {
                            wisp::lucide::draw_lucide_icon(
                                pixmap,
                                icon,
                                text_x,
                                y + 3.0,
                                10.0,
                                accent,
                            );
                            text_x += 14.0;
                        }
                        draw::draw_text(
                            pixmap,
                            font_system,
                            swash_cache,
                            &row.text,
                            text_x,
                            y + 4.0,
                            11.0,
                            font_family,
                            accent,
                        );
                        let line_y = y + HELP_SECTION_HEADER_H - 6.0;
                        draw::fill_rect(
                            pixmap,
                            pad,
                            line_y,
                            inner_w,
                            1.0,
                            Color::from_rgba8(255, 255, 255, 25),
                        );
                    }
                }
                RowKind::Entry => {
                    if let Some(entry) = &row.entry {
                        if y >= list_y && y + HELP_ROW_H <= list_y + list_h {
                            draw_entry_row(
                                pixmap,
                                font_system,
                                swash_cache,
                                pad,
                                y,
                                inner_w,
                                HELP_ROW_H,
                                entry,
                                fg,
                                desc_fg,
                                font_family,
                            );
                        }
                    }
                }
            }
        }
        let _ = y;
        let _ = total_h;
    }

    let hint_y = h - HELP_FOOTER_H + 4.0;
    let count = if state.visible_rows.is_empty() {
        "0 shortcuts".to_string()
    } else {
        let entry_count = state
            .visible_rows
            .iter()
            .filter(|r| r.kind == RowKind::Entry)
            .count();
        format!(
            "{entry_count} shortcut{}",
            if entry_count == 1 { "" } else { "s" }
        )
    };
    let cw = draw::text_width(font_system, &count, 11.0, font_family);
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        &count,
        pad,
        hint_y + 4.0,
        11.0,
        font_family,
        desc_fg,
    );
    let mut hint_x = w - pad;
    let close_text = "close";
    let close_tw = draw::text_width(font_system, close_text, 11.0, font_family);
    let close_text_x = hint_x - close_tw;
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        close_text,
        close_text_x,
        hint_y + 4.0,
        11.0,
        font_family,
        desc_fg,
    );
    hint_x = close_text_x - 2.0;
    let (esc_bw, _) = badge(
        pixmap,
        font_system,
        swash_cache,
        hint_x - 6.0,
        hint_y + 2.0,
        &BadgeProps {
            text: "Esc",
            variant: BadgeVariant::Secondary,
            ..Default::default()
        },
        font_family,
    );
    hint_x -= esc_bw + 8.0;
    let open_text = "open this view";
    let open_tw = draw::text_width(font_system, open_text, 11.0, font_family);
    let open_text_x = hint_x - open_tw;
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        open_text,
        open_text_x,
        hint_y + 4.0,
        11.0,
        font_family,
        desc_fg,
    );
    hint_x = open_text_x - 2.0;
    let (_f1_bw, _) = badge(
        pixmap,
        font_system,
        swash_cache,
        hint_x - 6.0,
        hint_y + 2.0,
        &BadgeProps {
            text: "F1",
            variant: BadgeVariant::Secondary,
            ..Default::default()
        },
        font_family,
    );
    let _ = (cw, accent, border);
}

fn draw_entry_row(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    entry: &ShortcutEntry,
    fg: Color,
    desc_fg: Color,
    font_family: &str,
) {
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        &entry.description,
        x + 2.0,
        y + 6.0,
        12.0,
        font_family,
        fg,
    );

    let kbd_text = if entry.keys_display.is_empty() {
        ""
    } else {
        entry.keys_display.as_str()
    };
    let props = KbdComboProps {
        keys: kbd_text,
        font_size: 10.0,
        separator: "+",
        padding_x: 5.0,
    };
    let combo_w = measure_kbd_combo(font_system, &props, font_family);
    let combo_x = x + w - 1.0 - combo_w;
    let (kbd_w, _kbd_h) = kbd_combo(
        pixmap,
        font_system,
        swash_cache,
        combo_x,
        y + 4.0,
        &props,
        font_family,
    );
    let _ = kbd_w;
    let _ = desc_fg;
    let _ = h;
}
