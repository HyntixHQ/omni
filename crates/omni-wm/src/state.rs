use cosmic_text::SwashCache;
use tiny_skia::{Color, Pixmap};
use wisp::draw::{self, color_from_hex};
use wisp::events::MouseDispatcher;
use wisp::input::InputAction;
use wisp_components::input;
use wisp_components::list_view::ListState;
use wisp_components::{draw_preview_rows, preview_row_height, ListColors, PreviewRow, PreviewRowSize, PreviewColors};

use crate::compositor::{compositor_name, query_monitors, query_workspaces, Compositor};
use crate::ops::{all_operations, workspace_dynamic_ops, CustomLayout, WmEntry};

pub const WM_WIDTH: i32 = 600;
pub const WM_HEIGHT: i32 = 400;

pub struct WmState {
    pub list_state: ListState,
    pub entries: Vec<WmEntry>,
    pub all_entries: Vec<WmEntry>,
    pub filter: String,
    pub compositor: Compositor,
    pub workspaces: Vec<u32>,
    pub monitor_count: usize,
    pub mouse: MouseDispatcher,
}

impl WmState {
    pub fn new(compositor: Compositor, custom: &[CustomLayout]) -> Self {
        let mut all = all_operations(custom);
        let workspaces = query_workspaces(compositor);
        let monitors = query_monitors(compositor);
        let monitor_count = monitors.len();
        all.extend(workspace_dynamic_ops(compositor, &workspaces));
        Self {
            list_state: ListState::new(),
            entries: all.clone(),
            all_entries: all,
            filter: String::new(),
            compositor,
            workspaces,
            monitor_count,
            mouse: MouseDispatcher::new(),
        }
    }

    pub fn filtered(&self) -> Vec<WmEntry> {
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
                })
                .cloned()
                .collect()
        }
    }

    pub fn selected(&self) -> Option<WmEntry> {
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

    pub fn execute_selected(&self) {
        if let Some(entry) = self.selected() {
            entry.execute(self.compositor);
        }
    }
}

pub fn handle_action(state: &mut WmState, action: InputAction, row_height: f32, body_h: f32) {
    let _ = (row_height, body_h);
    match action {
        InputAction::SelectNext => state.select_next(),
        InputAction::SelectPrev => state.select_prev(),
        InputAction::AppendChar(ch) => state.append_char(ch),
        InputAction::Backspace => state.backspace(),
        _ => {}
    }
}

pub fn compute_row_height(font_system: &mut cosmic_text::FontSystem, font_size: f32, font_family: &str) -> f32 {
    preview_row_height(font_system, font_size, font_family)
}

pub fn draw_wm(
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
    state: &mut WmState,
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
            placeholder: "Search window operations...",
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

    let preview_size = PreviewRowSize::default();
    let preview_colors = PreviewColors {
        accent,
        outline: border,
    };
    let list_colors = ListColors {
        bg: Color::from_rgba8(0, 0, 0, 0),
        hover: Color::from_rgba8(255, 255, 255, 20),
        active_bg: selected_bg,
        active_border: border,
        active_highlight: true,
        fg,
        selected_fg: fg,
        desc_fg,
        selected_desc_fg: desc_fg,
    };

    let rows: Vec<PreviewRow> = filtered
        .iter()
        .map(|e| PreviewRow {
            title: &e.name,
            subtitle: Some(&e.category),
            preview: e.preview,
            selected: false,
            disabled: e.command_for(state.compositor).is_none(),
        })
        .collect();
    let _ = sel;

    draw_preview_rows(
        pixmap, font_system, swash_cache,
        pad, list_y, inner_w, list_h,
        mouse_y, &rows, &mut state.list_state,
        font_size, font_family, &list_colors, preview_size, &preview_colors,
    );

    let hint_y = h - footer_h + 4.0;
    let hint_text = format!("{} ({} ops)", compositor_name(state.compositor), list_len);
    if state.monitor_count > 0 || !state.workspaces.is_empty() {
        let detail = format!("{} workspaces · {} monitors", state.workspaces.len(), state.monitor_count);
        draw::draw_text(
            pixmap, font_system, swash_cache,
            &detail, pad, hint_y + 4.0, 11.0, font_family, dim_fg,
        );
    }
    let hint_w = draw::text_width(font_system, &hint_text, 11.0, font_family);
    draw::draw_text(
        pixmap, font_system, swash_cache,
        &hint_text, w - pad - hint_w, hint_y + 4.0, 11.0, font_family, dim_fg,
    );
}
