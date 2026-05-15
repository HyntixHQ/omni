use omni_core::models::SearchResult;
use omni_search::app_index::AppIndex;
use omni_search::engine::SearchEngine;

use crate::config;
use crate::events::MouseDispatcher;
use crate::icon_lookup::IconCache;
use crate::components::list_view::ListViewState;

pub struct OmniApp {
    pub query: String,
    pub cursor_at: usize,
    pub app_index: AppIndex,
    pub search_engine: SearchEngine,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub icon_cache: IconCache,
    pub list_state: ListViewState,
    pub mouse: MouseDispatcher,
}

impl OmniApp {
    pub fn new(cfg: &config::Config) -> Self {
        let mut app_index = AppIndex::new();
        let _ = app_index.refresh();
        let mut slf = Self {
            query: String::new(),
            cursor_at: 0,
            app_index,
            search_engine: SearchEngine::new(),
            results: Vec::new(),
            selected_index: 0,
            icon_cache: IconCache::new(cfg.icons.theme.clone(), cfg.icons.size as u32),
            list_state: ListViewState::default(),
            mouse: MouseDispatcher::new(),
        };
        slf.run_search();
        slf
    }


    pub fn search(&mut self, query: &str) {
        if self.query != query {
            self.query = query.to_string();
            self.cursor_at = self.query.chars().count();
            self.selected_index = 0;
            self.list_state.scroll_offset = 0.0;
            self.run_search();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let mut chars: Vec<char> = self.query.chars().collect();
        if self.cursor_at <= chars.len() {
            chars.insert(self.cursor_at, ch);
            self.query = chars.into_iter().collect();
            self.cursor_at += 1;
            self.selected_index = 0;
            self.list_state.scroll_offset = 0.0;
            self.run_search();
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_at > 0 {
            let mut chars: Vec<char> = self.query.chars().collect();
            chars.remove(self.cursor_at - 1);
            self.query = chars.into_iter().collect();
            self.cursor_at -= 1;
            self.selected_index = 0;
            self.list_state.scroll_offset = 0.0;
            self.run_search();
        }
    }

    pub fn delete(&mut self) {
        let mut chars: Vec<char> = self.query.chars().collect();
        if self.cursor_at < chars.len() {
            chars.remove(self.cursor_at);
            self.query = chars.into_iter().collect();
            self.selected_index = 0;
            self.list_state.scroll_offset = 0.0;
            self.run_search();
        }
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor_at = self.cursor_at.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        let count = self.query.chars().count();
        if self.cursor_at < count {
            self.cursor_at += 1;
        }
    }

    pub fn move_cursor_start(&mut self) {
        self.cursor_at = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_at = self.query.chars().count();
    }

    pub fn delete_word_backward(&mut self) {
        if self.cursor_at == 0 {
            return;
        }
        let chars: Vec<char> = self.query.chars().collect();
        let mut i = self.cursor_at;
        // skip trailing spaces
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        // find start of word
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        let mut new_chars = chars;
        new_chars.drain(i..self.cursor_at);
        self.query = new_chars.into_iter().collect();
        self.cursor_at = i;
        self.selected_index = 0;
        self.list_state.scroll_offset = 0.0;
        self.run_search();
    }

    pub fn move_word_left(&mut self) {
        if self.cursor_at == 0 {
            return;
        }
        let chars: Vec<char> = self.query.chars().collect();
        let mut i = self.cursor_at;
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        self.cursor_at = i;
    }

    pub fn move_word_right(&mut self) {
        let chars: Vec<char> = self.query.chars().collect();
        let count = chars.len();
        if self.cursor_at >= count {
            return;
        }
        let mut i = self.cursor_at;
        while i < count && chars[i].is_whitespace() {
            i += 1;
        }
        while i < count && !chars[i].is_whitespace() {
            i += 1;
        }
        self.cursor_at = i;
    }

    pub fn delete_word_forward(&mut self) {
        let chars: Vec<char> = self.query.chars().collect();
        let count = chars.len();
        if self.cursor_at >= count {
            return;
        }
        let mut i = self.cursor_at;
        // skip leading spaces
        while i < count && chars[i].is_whitespace() {
            i += 1;
        }
        // find end of word
        while i < count && !chars[i].is_whitespace() {
            i += 1;
        }
        let mut new_chars = chars;
        new_chars.drain(self.cursor_at..i);
        self.query = new_chars.into_iter().collect();
        self.selected_index = 0;
        self.list_state.scroll_offset = 0.0;
        self.run_search();
    }

    fn run_search(&mut self) {
        if self.query.is_empty() {
            self.results = self
                .app_index
                .apps()
                .iter()
                .enumerate()
                .map(|(i, entry)| SearchResult {
                    score: (config::MAX_SEARCH_RESULTS as usize).saturating_sub(i) as i64,
                    matches: Vec::new(),
                    entry: entry.clone(),
                })
                .collect();
        } else {
            self.results = self
                .search_engine
                .search(&self.query, self.app_index.apps());
        }
        self.selected_index = self
            .selected_index
            .min(self.results.len().saturating_sub(1));
    }

    pub fn select_next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1).min(self.results.len() - 1);
    }

    pub fn select_prev(&mut self) {
        if self.results.is_empty() {
            return;
        }
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn launch_selected(exec: &str) {
        let sanitized = exec
            .replace("%f", "")
            .replace("%F", "")
            .replace("%u", "")
            .replace("%U", "")
            .replace("%d", "")
            .replace("%D", "")
            .replace("%n", "")
            .replace("%N", "")
            .replace("%i", "")
            .replace("%c", "")
            .replace("%k", "")
            .replace("%v", "")
            .replace("%m", "")
            .trim()
            .to_string();
        if !sanitized.is_empty() {
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(&sanitized)
                .spawn();
        }
    }
}
