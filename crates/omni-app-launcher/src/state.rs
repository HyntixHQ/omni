use wisp::events::MouseDispatcher;
use wisp_components::list_view::ListState;

use crate::config::{self, Config};
use crate::icon_cache::IconCache;

pub struct OmniApp {
    pub text_editor: wisp::text::TextEditor,
    pub app_index: omni_search::app_index::AppIndex,
    pub search_engine: omni_search::engine::SearchEngine,
    pub results: Vec<omni_core::models::SearchResult>,
    pub selected_index: usize,
    pub icon_cache: IconCache,
    pub list_state: ListState,
    pub mouse: MouseDispatcher,
}

impl OmniApp {
    pub fn new(cfg: &Config) -> Self {
        let mut app_index = omni_search::app_index::AppIndex::new();
        let _ = app_index.refresh();
        let mut slf = Self {
            text_editor: wisp::text::TextEditor::new(),
            app_index,
            search_engine: omni_search::engine::SearchEngine::new(),
            results: Vec::new(),
            selected_index: 0,
            icon_cache: IconCache::new(cfg.icons.theme.clone(), cfg.icons.size as u32),
            list_state: ListState::new(),
            mouse: MouseDispatcher::new(),
        };
        slf.run_search();
        slf
    }

    pub fn query(&self) -> &str {
        &self.text_editor.text
    }

    pub fn cursor_at(&self) -> usize {
        self.text_editor.cursor
    }

    pub fn search(&mut self, query: &str) {
        if self.text_editor.text != query {
            self.text_editor.set_text(query);
            self.selected_index = 0;
            self.list_state.scroll.set_offset(0.0);
            self.run_search();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.text_editor.insert(ch);
        self.selected_index = 0;
        self.list_state.scroll.set_offset(0.0);
        self.run_search();
    }

    pub fn backspace(&mut self) {
        self.text_editor.backspace();
        self.selected_index = 0;
        self.list_state.scroll.set_offset(0.0);
        self.run_search();
    }

    pub fn delete(&mut self) {
        self.text_editor.delete();
        self.selected_index = 0;
        self.list_state.scroll.set_offset(0.0);
        self.run_search();
    }

    pub fn move_cursor_left(&mut self) {
        self.text_editor.move_left();
    }

    pub fn move_cursor_right(&mut self) {
        self.text_editor.move_right();
    }

    pub fn move_cursor_start(&mut self) {
        self.text_editor.move_start();
    }

    pub fn move_cursor_end(&mut self) {
        self.text_editor.move_end();
    }

    pub fn delete_word_backward(&mut self) {
        self.text_editor.delete_word_backward();
        self.selected_index = 0;
        self.list_state.scroll.set_offset(0.0);
        self.run_search();
    }

    pub fn move_word_left(&mut self) {
        self.text_editor.move_word_left();
    }

    pub fn move_word_right(&mut self) {
        self.text_editor.move_word_right();
    }

    pub fn delete_word_forward(&mut self) {
        self.text_editor.delete_word_forward();
        self.selected_index = 0;
        self.list_state.scroll.set_offset(0.0);
        self.run_search();
    }

    fn run_search(&mut self) {
        if self.text_editor.text.is_empty() {
            self.results = self
                .app_index
                .apps()
                .iter()
                .enumerate()
                .map(|(i, entry)| omni_core::models::SearchResult {
                    score: (config::MAX_SEARCH_RESULTS).saturating_sub(i) as i64,
                    matches: Vec::new(),
                    entry: entry.clone(),
                })
                .collect();
        } else {
            self.results = self
                .search_engine
                .search(&self.query(), self.app_index.apps());
        }
        self.selected_index = self.selected_index.min(self.results.len().saturating_sub(1));
    }

    pub fn select_next(&mut self) {
        if self.results.is_empty() { return; }
        self.selected_index = (self.selected_index + 1).min(self.results.len() - 1);
    }

    pub fn select_prev(&mut self) {
        if self.results.is_empty() { return; }
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn launch_selected(exec: &str) {
        let sanitized = exec
            .replace("%f", "").replace("%F", "").replace("%u", "")
            .replace("%U", "").replace("%d", "").replace("%D", "")
            .replace("%n", "").replace("%N", "").replace("%i", "")
            .replace("%c", "").replace("%k", "").replace("%v", "").replace("%m", "")
            .trim().to_string();
        if !sanitized.is_empty() {
            let _ = std::process::Command::new("sh").arg("-c").arg(&sanitized).spawn();
        }
    }
}

pub fn handle_action(app: &mut OmniApp, action: wisp::input::InputAction, row_height: f32, _body_h: f32) {
    match action {
        wisp::input::InputAction::Confirm => {
            if !app.results.is_empty() {
                let exec = app.results[app.selected_index].entry.exec.clone();
                OmniApp::launch_selected(&exec);
            }
        }
        wisp::input::InputAction::Cancel => {}
        wisp::input::InputAction::Backspace => app.backspace(),
        wisp::input::InputAction::Delete => app.delete(),
        wisp::input::InputAction::MoveCursorLeft => app.move_cursor_left(),
        wisp::input::InputAction::MoveCursorRight => app.move_cursor_right(),
        wisp::input::InputAction::MoveCursorStart => app.move_cursor_start(),
        wisp::input::InputAction::MoveCursorEnd => app.move_cursor_end(),
        wisp::input::InputAction::DeleteWordBackward => app.delete_word_backward(),
        wisp::input::InputAction::DeleteWordForward => app.delete_word_forward(),
        wisp::input::InputAction::MoveWordLeft => app.move_word_left(),
        wisp::input::InputAction::MoveWordRight => app.move_word_right(),
        wisp::input::InputAction::SelectLeft => app.text_editor.select_left(),
        wisp::input::InputAction::SelectRight => app.text_editor.select_right(),
        wisp::input::InputAction::SelectStart => app.text_editor.select_start(),
        wisp::input::InputAction::SelectEnd => app.text_editor.select_end(),
        wisp::input::InputAction::SelectWordLeft => app.text_editor.select_word_left(),
        wisp::input::InputAction::SelectWordRight => app.text_editor.select_word_right(),
        wisp::input::InputAction::SelectAll => app.text_editor.select_all(),
        wisp::input::InputAction::Copy => {}
        wisp::input::InputAction::Cut => {}
        wisp::input::InputAction::Paste => {}
        wisp::input::InputAction::SelectPrev => {
            app.select_prev();
            app.list_state.selected_index = app.selected_index;
            app.list_state.ensure_selected_visible(row_height);
        }
        wisp::input::InputAction::SelectNext => {
            app.select_next();
            app.list_state.selected_index = app.selected_index;
            app.list_state.ensure_selected_visible(row_height);
        }
        wisp::input::InputAction::AppendChar(ch) => app.insert_char(ch),
        wisp::input::InputAction::None => {}
    }
}
