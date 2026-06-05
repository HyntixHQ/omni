use cosmic_text::SwashCache;
use tiny_skia::{Color, Pixmap};
use wisp::draw::{self, color_from_hex};
use wisp::events::MouseDispatcher;
use wisp::input::InputAction;
use wisp::text::TextEditor;
use wisp_components::badge::{badge, BadgeColors, BadgeProps, BadgeVariant};
use wisp_components::input;
use wisp_components::list_view::{draw_list, row_height, ListColors, ListItem, ListState};

use crate::snippet::{expand_placeholders, Snippet};

pub const SNIPPETS_WIDTH: i32 = 600;
pub const SNIPPETS_HEIGHT: i32 = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Name,
    Shortcut,
    Content,
}

impl FormField {
    fn next(self) -> Self {
        match self {
            FormField::Name => FormField::Shortcut,
            FormField::Shortcut => FormField::Content,
            FormField::Content => FormField::Name,
        }
    }
    fn prev(self) -> Self {
        match self {
            FormField::Name => FormField::Content,
            FormField::Shortcut => FormField::Name,
            FormField::Content => FormField::Shortcut,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SnippetMode {
    Browse,
    Form {
        editing: Option<usize>,
        field: FormField,
        error: Option<String>,
    },
    ConfirmDelete {
        index: usize,
    },
}

pub struct SnippetsState {
    pub list_state: ListState,
    pub all_entries: Vec<Snippet>,
    pub filter: String,
    pub last_clipboard: Option<String>,
    pub mouse: MouseDispatcher,
    pub mode: SnippetMode,
    pub form_name: TextEditor,
    pub form_shortcut: TextEditor,
    pub form_content: TextEditor,
    pub saved_flash: Option<std::time::Instant>,
}

impl SnippetsState {
    pub fn new(snippets: Vec<Snippet>, last_clipboard: Option<String>) -> Self {
        Self {
            list_state: ListState::new(),
            all_entries: snippets,
            filter: String::new(),
            last_clipboard,
            mouse: MouseDispatcher::new(),
            mode: SnippetMode::Browse,
            form_name: TextEditor::new(),
            form_shortcut: TextEditor::new(),
            form_content: TextEditor::new(),
            saved_flash: None,
        }
    }

    pub fn reload(&mut self, snippets: Vec<Snippet>) {
        self.all_entries = snippets;
        self.filter.clear();
        self.list_state.selected_index = 0;
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

    pub fn expand(&self, snippet: &Snippet) -> String {
        expand_placeholders(&snippet.content, self.last_clipboard.as_deref())
    }

    pub fn exact_shortcut_match(&self) -> Option<Snippet> {
        if self.filter.is_empty() {
            return None;
        }
        self.all_entries
            .iter()
            .find(|s| s.shortcut.as_deref() == Some(self.filter.as_str()))
            .cloned()
    }

    pub fn open_new(&mut self) {
        self.form_name.clear();
        self.form_shortcut.clear();
        self.form_content.clear();
        self.mode = SnippetMode::Form {
            editing: None,
            field: FormField::Name,
            error: None,
        };
    }

    pub fn open_edit(&mut self) {
        if let Some(snip) = self.selected() {
            self.form_name.set_text(&snip.name);
            let sc = snip.shortcut.clone().unwrap_or_default();
            self.form_shortcut.set_text(&sc);
            self.form_content.set_text(&snip.content);
            let idx = self.list_state.selected_index;
            self.mode = SnippetMode::Form {
                editing: Some(idx),
                field: FormField::Name,
                error: None,
            };
        }
    }

    pub fn request_delete(&mut self) {
        if !self.all_entries.is_empty() {
            let idx = self
                .filtered()
                .get(self.list_state.selected_index)
                .and_then(|sel| {
                    self.all_entries
                        .iter()
                        .position(|s| s.name == sel.name && s.content == sel.content)
                })
                .unwrap_or(0);
            self.mode = SnippetMode::ConfirmDelete { index: idx };
        }
    }

    pub fn save_form(&mut self) -> bool {
        let (editing, _field) = match &self.mode {
            SnippetMode::Form { editing, field, .. } => (*editing, *field),
            _ => return false,
        };

        let name = self.form_name.text.trim().to_string();
        let content = self.form_content.text.clone();
        let shortcut_raw = self.form_shortcut.text.trim().to_string();
        let shortcut = if shortcut_raw.is_empty() {
            None
        } else {
            Some(shortcut_raw)
        };

        if name.is_empty() {
            self.set_form_error("Name is required");
            return false;
        }
        if content.is_empty() {
            self.set_form_error("Content is required");
            return false;
        }

        if let Some(sc) = &shortcut {
            let dup = self
                .all_entries
                .iter()
                .enumerate()
                .any(|(i, s)| Some(i) != editing && s.shortcut.as_deref() == Some(sc.as_str()));
            if dup {
                self.set_form_error("Shortcut already in use");
                return false;
            }
        }

        let new_snip = Snippet {
            name,
            content,
            shortcut,
        };

        match editing {
            Some(idx) => {
                if idx < self.all_entries.len() {
                    self.all_entries[idx] = new_snip;
                }
            }
            None => self.all_entries.push(new_snip),
        }

        self.mode = SnippetMode::Browse;
        self.saved_flash = Some(std::time::Instant::now());
        true
    }

    pub fn confirm_delete(&mut self) {
        if let SnippetMode::ConfirmDelete { index } = self.mode {
            if index < self.all_entries.len() {
                self.all_entries.remove(index);
            }
            self.mode = SnippetMode::Browse;
            self.list_state.selected_index = 0;
            self.saved_flash = Some(std::time::Instant::now());
        }
    }

    pub fn cancel_form(&mut self) {
        self.mode = SnippetMode::Browse;
    }

    fn set_form_error(&mut self, msg: &str) {
        if let SnippetMode::Form { error, .. } = &mut self.mode {
            *error = Some(msg.to_string());
        }
    }

    pub fn active_editor(&mut self) -> Option<&mut TextEditor> {
        let field = match &self.mode {
            SnippetMode::Form { field, .. } => *field,
            _ => return None,
        };
        Some(match field {
            FormField::Name => &mut self.form_name,
            FormField::Shortcut => &mut self.form_shortcut,
            FormField::Content => &mut self.form_content,
        })
    }
}

pub fn handle_action(state: &mut SnippetsState, action: InputAction) {
    match &state.mode {
        SnippetMode::Browse => match action {
            InputAction::SelectNext => {
                let n = state.filtered().len();
                state.list_state.select_next(n);
            }
            InputAction::SelectPrev => {
                state.list_state.select_prev();
            }
            InputAction::AppendChar(ch) => {
                state.filter.push(ch);
                state.list_state.selected_index = 0;
            }
            InputAction::Backspace => {
                state.filter.pop();
                state.list_state.selected_index = 0;
            }
            InputAction::Create => state.open_new(),
            InputAction::Edit => state.open_edit(),
            InputAction::Delete => state.request_delete(),
            _ => {}
        },
        SnippetMode::Form { field, .. } => {
            let field = *field;
            match action {
                InputAction::Cancel => state.cancel_form(),
                InputAction::Save => {
                    state.save_form();
                }
                InputAction::Tab => {
                    if let SnippetMode::Form { field, .. } = &mut state.mode {
                        *field = field.next();
                    }
                }
                InputAction::BackTab => {
                    if let SnippetMode::Form { field, .. } = &mut state.mode {
                        *field = field.prev();
                    }
                }
                InputAction::Confirm => {
                    if field == FormField::Content {
                        if let Some(ed) = state.active_editor() {
                            ed.text.insert(ed.cursor, '\n');
                            ed.cursor += 1;
                        }
                    } else if let SnippetMode::Form { field, .. } = &mut state.mode {
                        *field = field.next();
                    }
                }
                InputAction::AppendChar(ch) => {
                    if let Some(ed) = state.active_editor() {
                        ed.text.insert(ed.cursor, ch);
                        ed.cursor += ch.len_utf8();
                    }
                }
                InputAction::Backspace => {
                    if let Some(ed) = state.active_editor() {
                        if ed.cursor > 0 {
                            let prev = ed.text[..ed.cursor]
                                .chars()
                                .next_back()
                                .map_or(1, char::len_utf8);
                            ed.cursor -= prev;
                            ed.text.drain(ed.cursor..ed.cursor + prev);
                        }
                    }
                }
                InputAction::Delete => {
                    if let Some(ed) = state.active_editor() {
                        if ed.cursor < ed.text.len() {
                            let next = ed.text[ed.cursor..]
                                .chars()
                                .next()
                                .map_or(1, char::len_utf8);
                            ed.text.drain(ed.cursor..ed.cursor + next);
                        }
                    }
                }
                InputAction::MoveCursorLeft => {
                    if let Some(ed) = state.active_editor() {
                        ed.cursor = ed.text[..ed.cursor].chars().next_back().map_or(0, |_| {
                            ed.text[..ed.cursor]
                                .char_indices()
                                .last()
                                .map_or(0, |(i, _)| i)
                        });
                    }
                }
                InputAction::MoveCursorRight => {
                    if let Some(ed) = state.active_editor() {
                        if let Some((i, _)) = ed.text[ed.cursor..].char_indices().nth(1) {
                            ed.cursor += i;
                        }
                    }
                }
                InputAction::MoveCursorStart => {
                    if let Some(ed) = state.active_editor() {
                        ed.cursor = 0;
                    }
                }
                InputAction::MoveCursorEnd => {
                    if let Some(ed) = state.active_editor() {
                        ed.cursor = ed.text.len();
                    }
                }
                _ => {}
            }
        }
        SnippetMode::ConfirmDelete { .. } => match action {
            InputAction::Confirm | InputAction::AppendChar('y') | InputAction::AppendChar('Y') => {
                state.confirm_delete()
            }
            InputAction::Cancel | InputAction::AppendChar('n') | InputAction::AppendChar('N') => {
                state.mode = SnippetMode::Browse;
            }
            _ => {}
        },
    }
}

pub fn compute_row_height(
    font_system: &mut cosmic_text::FontSystem,
    font_size: f32,
    font_family: &str,
) -> f32 {
    row_height(font_system, font_size, font_family)
}

const FORM_FIELD_LABEL_H: f32 = 18.0;
const FORM_FIELD_GAP: f32 = 6.0;
const FORM_NAME_H: f32 = 32.0;
const FORM_SHORTCUT_H: f32 = 32.0;
const FORM_CONTENT_H: f32 = 160.0;

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

    if window_radius > 0.0 {
        draw::fill_rounded_rect(pixmap, 0.0, 0.0, w, h, [window_radius; 4], bg);
    } else {
        pixmap.fill(bg);
    }

    match &state.mode {
        SnippetMode::Browse => draw_browse(
            pixmap,
            font_system,
            swash_cache,
            w,
            h,
            pad,
            fg,
            selected_bg,
            dim_fg,
            desc_fg,
            accent,
            border,
            placeholder_fg,
            caret,
            font_size,
            font_family,
            state,
            cursor_visible,
            mouse_y,
        ),
        SnippetMode::Form { field, error, .. } => {
            let field = *field;
            let error = error.clone();
            draw_form(
                pixmap,
                font_system,
                swash_cache,
                w,
                h,
                pad,
                fg,
                dim_fg,
                desc_fg,
                border,
                placeholder_fg,
                caret,
                font_size,
                font_family,
                state,
                cursor_visible,
                mouse_y,
                field,
                error,
            );
        }
        SnippetMode::ConfirmDelete { index } => {
            draw_confirm(
                pixmap,
                font_system,
                swash_cache,
                w,
                h,
                pad,
                fg,
                dim_fg,
                desc_fg,
                border,
                caret,
                font_size,
                font_family,
                state,
                mouse_y,
                *index,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_browse(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    w: f32,
    h: f32,
    pad: f32,
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
    state: &mut SnippetsState,
    cursor_visible: bool,
    mouse_y: f32,
) {
    let _ = accent;
    let header_h = 42.0;
    let gap = 8.0;
    let footer_h = 28.0;
    let inner_w = w - pad * 2.0;

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
    if state.list_state.selected_index >= list_len {
        state.list_state.selected_index = list_len.saturating_sub(1);
    }
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
                pixmap,
                font_system,
                swash_cache,
                badge_x,
                hint_y + 2.0,
                &BadgeProps {
                    text: sc,
                    variant: BadgeVariant::Outline,
                    focused: false,
                    colors: Some(badge_colors),
                    icon: None,
                    lucide_icon: None,
                },
                font_family,
            );
            badge_x += bw + 6.0;
        }
    }

    let saved_text = if let Some(t) = state.saved_flash {
        if t.elapsed().as_millis() < 1200 {
            Some("Saved")
        } else {
            state.saved_flash = None;
            None
        }
    } else {
        None
    };

    if let Some(text) = saved_text {
        let tw = draw::text_width(font_system, text, 11.0, font_family);
        draw::draw_text(
            pixmap,
            font_system,
            swash_cache,
            text,
            w - pad - tw,
            hint_y + 4.0,
            11.0,
            font_family,
            accent,
        );
    } else {
        let hint_text = format!("{} snippets", list_len);
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
    }

    let _ = (badge_x, dim_fg);
}

#[allow(clippy::too_many_arguments)]
fn draw_form(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    w: f32,
    h: f32,
    pad: f32,
    fg: Color,
    dim_fg: Color,
    desc_fg: Color,
    border: Color,
    placeholder_fg: Color,
    caret: Color,
    font_size: f32,
    font_family: &str,
    state: &mut SnippetsState,
    cursor_visible: bool,
    mouse_y: f32,
    active_field: FormField,
    error: Option<String>,
) {
    let _ = (dim_fg, mouse_y);
    let inner_w = w - pad * 2.0;
    let mut y = pad + 6.0;

    let is_edit = matches!(
        state.mode,
        SnippetMode::Form {
            editing: Some(_),
            ..
        }
    );
    let title = if is_edit {
        "Edit snippet"
    } else {
        "New snippet"
    };
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        title,
        pad,
        y,
        font_size,
        font_family,
        fg,
    );
    y += font_size + 10.0;

    let _ = (FormField::Name, FormField::Shortcut, FormField::Content);

    y += draw_form_field(
        pixmap,
        font_system,
        swash_cache,
        pad,
        y,
        inner_w,
        "Name *",
        active_field == FormField::Name,
        &state.form_name.text,
        state.form_name.cursor,
        cursor_visible && active_field == FormField::Name,
        fg,
        dim_fg,
        border,
        placeholder_fg,
        caret,
        font_size,
        font_family,
        FORM_NAME_H,
    );
    y += FORM_FIELD_GAP;

    y += draw_form_field(
        pixmap,
        font_system,
        swash_cache,
        pad,
        y,
        inner_w,
        "Shortcut (optional)",
        active_field == FormField::Shortcut,
        &state.form_shortcut.text,
        state.form_shortcut.cursor,
        cursor_visible && active_field == FormField::Shortcut,
        fg,
        dim_fg,
        border,
        placeholder_fg,
        caret,
        font_size,
        font_family,
        FORM_SHORTCUT_H,
    );
    y += FORM_FIELD_GAP;

    y += draw_form_field(
        pixmap,
        font_system,
        swash_cache,
        pad,
        y,
        inner_w,
        "Content *",
        active_field == FormField::Content,
        &state.form_content.text,
        state.form_content.cursor,
        cursor_visible && active_field == FormField::Content,
        fg,
        dim_fg,
        border,
        placeholder_fg,
        caret,
        font_size,
        font_family,
        FORM_CONTENT_H,
    );

    if let Some(ref err) = error {
        draw::draw_text(
            pixmap,
            font_system,
            swash_cache,
            err,
            pad,
            y + 4.0,
            11.0,
            font_family,
            color_from_hex("#ef4444"),
        );
    }

    let hint_y = h - 24.0;
    let hint = "Tab  next   Ctrl+S  save   Esc  cancel";
    let hint_w = draw::text_width(font_system, hint, 11.0, font_family);
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        hint,
        w - pad - hint_w,
        hint_y,
        11.0,
        font_family,
        desc_fg,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_form_field(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    focused: bool,
    value: &str,
    cursor: usize,
    cursor_visible: bool,
    fg: Color,
    dim_fg: Color,
    border: Color,
    placeholder_fg: Color,
    caret: Color,
    font_size: f32,
    font_family: &str,
    input_h: f32,
) -> f32 {
    let _ = dim_fg;
    let label_color = if focused { fg } else { dim_fg };
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        label,
        x,
        y,
        11.0,
        font_family,
        label_color,
    );

    let input_y = y + FORM_FIELD_LABEL_H;
    let bg_color = color_from_hex("#1a1a1a");
    let display_value = if value.is_empty() && !focused {
        ""
    } else {
        value
    };
    let placeholder = match label {
        l if l.starts_with("Name") => "Snippet name",
        l if l.starts_with("Shortcut") => ";keyword",
        l if l.starts_with("Content") => "Snippet text — {date}, {time}, {clipboard}",
        _ => "",
    };

    let input_box_y = input_y;
    let input_box_h = input_h;
    let draw_caret = cursor_visible && focused;

    draw::draw_text_clipped(
        pixmap,
        font_system,
        swash_cache,
        x + 8.0,
        input_box_y + 8.0,
        display_value,
        font_size,
        font_family,
        fg,
        input_box_y,
        input_box_y + input_box_h,
        w - 16.0,
    );

    let _ = (placeholder, placeholder_fg, caret, border, bg_color, cursor);
    let caret_x = if draw_caret {
        let pre = &value[..cursor.min(value.len())];
        x + 8.0 + draw::text_width(font_system, pre, font_size, font_family)
    } else {
        0.0
    };
    if draw_caret {
        draw::fill_rect(
            pixmap,
            caret_x.round(),
            input_box_y + 6.0,
            1.5,
            input_box_h - 12.0,
            caret,
        );
    }

    let rect_color = if focused {
        color_from_hex("#3b82f6")
    } else {
        color_from_hex("#3a3a3a")
    };
    let stroke = if focused { 2.0 } else { 1.0 };
    let _ = rect_color;
    let _ = stroke;
    let _ = w;

    FORM_FIELD_LABEL_H + input_h
}

#[allow(clippy::too_many_arguments)]
fn draw_confirm(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    w: f32,
    h: f32,
    pad: f32,
    fg: Color,
    dim_fg: Color,
    desc_fg: Color,
    border: Color,
    caret: Color,
    font_size: f32,
    font_family: &str,
    state: &SnippetsState,
    mouse_y: f32,
    index: usize,
) {
    let _ = (dim_fg, border, caret, mouse_y);
    let inner_w = w - pad * 2.0;
    let cy = h / 2.0 - 30.0;
    let name = state
        .all_entries
        .get(index)
        .map(|s| s.name.as_str())
        .unwrap_or("?");
    let line1 = format!("Delete '{}'?", name);
    let line2 = "This cannot be undone.";
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        &line1,
        pad,
        cy,
        font_size,
        font_family,
        fg,
    );
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        line2,
        pad,
        cy + font_size + 6.0,
        12.0,
        font_family,
        desc_fg,
    );

    let hint = "Y  delete    N / Esc  cancel";
    let hint_w = draw::text_width(font_system, hint, 11.0, font_family);
    draw::draw_text(
        pixmap,
        font_system,
        swash_cache,
        hint,
        w - pad - hint_w,
        h - 24.0,
        11.0,
        font_family,
        desc_fg,
    );
    let _ = inner_w;
}
