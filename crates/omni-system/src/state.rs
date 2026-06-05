use cosmic_text::SwashCache;
use tiny_skia::{Color, Pixmap};
use wisp::draw::{self, color_from_hex};
use wisp::events::MouseDispatcher;
use wisp::input::InputAction;
use wisp_components::badge::{badge, BadgeColors, BadgeProps, BadgeVariant};
use wisp_components::input;
use wisp_components::list_view::{draw_list, row_height, ListColors, ListItem, ListState};
use wisp_components::Icon;

use crate::commands::{preset_commands, SystemEntry};
use crate::compositor::{compositor_name, detect_compositor, Compositor};

pub const SYSTEM_WIDTH: i32 = 600;
pub const SYSTEM_HEIGHT: i32 = 400;

pub struct SystemState {
    pub list_state: ListState,
    pub entries: Vec<SystemEntry>,
    pub all_entries: Vec<SystemEntry>,
    pub filter: String,
    pub compositor: Compositor,
    pub mouse: MouseDispatcher,
}

impl SystemState {
    pub fn new() -> Self {
        let compositor = detect_compositor();
        let all = preset_commands(compositor);
        Self {
            list_state: ListState::new(),
            entries: all.clone(),
            all_entries: all,
            filter: String::new(),
            compositor,
            mouse: MouseDispatcher::new(),
        }
    }

    pub fn filtered(&self) -> Vec<SystemEntry> {
        if self.filter.is_empty() {
            self.all_entries.clone()
        } else {
            let q = self.filter.to_lowercase();
            self.all_entries
                .iter()
                .filter(|e| {
                    e.name.to_lowercase().contains(&q)
                        || e.description.to_lowercase().contains(&q)
                        || e.category.to_lowercase().contains(&q)
                        || e.command.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        }
    }

    pub fn selected(&self) -> Option<SystemEntry> {
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

    pub fn execute_selected(&self) {
        if let Some(entry) = self.selected() {
            entry.execute();
        }
    }
}

impl Default for SystemState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn handle_action(state: &mut SystemState, action: InputAction, _row_height: f32, _body_h: f32) {
    match action {
        InputAction::SelectNext => state.select_next(),
        InputAction::SelectPrev => state.select_prev(),
        InputAction::AppendChar(ch) => state.append_char(ch),
        InputAction::Backspace => state.backspace(),
        _ => {}
    }
}

pub fn compute_row_height(
    font_system: &mut cosmic_text::FontSystem,
    font_size: f32,
    font_family: &str,
) -> f32 {
    row_height(font_system, font_size, font_family)
}

fn system_category_icon(category: &str) -> Option<Icon> {
    Some(match category {
        "Session" => Icon::Lock,
        "Power" => Icon::Power,
        "Display" => Icon::Monitor,
        "Audio" => Icon::Speaker,
        "Brightness" => Icon::Sun,
        "Bluetooth" => Icon::Bluetooth,
        "Network" => Icon::Wifi,
        _ => return None,
    })
}

pub fn draw_system(
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
    destructive_fg: Color,
    font_size: f32,
    font_family: &str,
    window_radius: f32,
    state: &mut SystemState,
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
        pixmap,
        font_system,
        swash_cache,
        pad,
        6.0,
        inner_w,
        header_h,
        &input::InputProps {
            value: &state.filter,
            cursor_at: state.filter.len(),
            placeholder: "Search system commands...",
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

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|e| ListItem {
            title: &e.name,
            subtitle: Some(&e.description),
            prefix_icon: None,
            suffix_icon: None,
            selected: false,
            disabled: e.command.is_empty(),
        })
        .collect();
    let _ = sel;

    draw_list(
        pixmap,
        font_system,
        swash_cache,
        pad,
        list_y,
        inner_w,
        list_h,
        mouse_y,
        &items,
        &mut state.list_state,
        font_size,
        font_family,
        &list_colors,
    );

    let _ = (accent, dim_fg, destructive_fg);
    let hint_y = h - footer_h + 4.0;
    let hint_text = format!(
        "{} · {} commands",
        compositor_name(state.compositor),
        list_len
    );
    let hint_w = draw::text_width(font_system, &hint_text, 11.0, font_family);
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        &hint_text,
        w - pad - hint_w,
        hint_y + 4.0,
        11.0,
        font_family,
        desc_fg,
    );

    let mut badge_x = pad;
    let visible_first = state.list_state.hover_index.unwrap_or(sel);
    if let Some(item) = filtered.get(visible_first) {
        let badge_colors = BadgeColors {
            bg: Color::from_rgba8(0, 0, 0, 0),
            fg: desc_fg,
            border: Some(border),
        };
        let (bw, _) = badge(
            pixmap,
            font_system,
            swash_cache,
            badge_x,
            hint_y + 2.0,
            &BadgeProps {
                text: &item.category,
                variant: BadgeVariant::Outline,
                focused: false,
                colors: Some(badge_colors),
                icon: None,
                lucide_icon: system_category_icon(&item.category),
            },
            font_family,
        );
        badge_x += bw + 6.0;
        if item.destructive {
            let badge_colors = BadgeColors {
                bg: Color::from_rgba8(69, 26, 26, 200),
                fg: destructive_fg,
                border: None,
            };
            let (bw, _) = badge(
                pixmap,
                font_system,
                swash_cache,
                badge_x,
                hint_y + 2.0,
                &BadgeProps {
                    text: "destructive",
                    variant: BadgeVariant::Destructive,
                    focused: false,
                    colors: Some(badge_colors),
                    icon: None,
                    lucide_icon: Some(Icon::AlertTriangle),
                },
                font_family,
            );
            let _ = bw;
        }
    }
}
