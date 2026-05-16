use unicode_segmentation::UnicodeSegmentation;

/// A text editor buffer with grapheme-cluster-aware cursor and selection.
/// Inspired by GPUI's InputState, simplified for immediate-mode rendering.
#[derive(Debug, Clone)]
pub struct TextEditor {
    /// The full text content.
    pub text: String,
    /// Cursor position as grapheme cluster index (byte offset).
    pub cursor: usize,
    /// Selection anchor in grapheme cluster offset. None when no active selection.
    pub select_anchor: Option<usize>,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            select_anchor: None,
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.select_anchor = None;
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.len();
        self.select_anchor = None;
    }

    pub fn selected_text(&self) -> &str {
        if let Some(anchor) = self.select_anchor {
            let start = self.cursor.min(anchor);
            let end = self.cursor.max(anchor);
            &self.text[start..end]
        } else {
            ""
        }
    }

    pub fn has_selection(&self) -> bool {
        self.select_anchor.is_some()
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.select_anchor.map(|anchor| {
            let start = self.cursor.min(anchor);
            let end = self.cursor.max(anchor);
            (start, end)
        })
    }

    /// Delete the selected range if any, returning the deleted text.
    fn delete_selection(&mut self) -> String {
        if let Some((start, end)) = self.selection_range() {
            let deleted = self.text[start..end].to_string();
            self.text.drain(start..end);
            self.cursor = start;
            self.select_anchor = None;
            deleted
        } else {
            String::new()
        }
    }

    /// Return the grapheme cluster byte indices up to `byte_offset`.
    fn grapheme_offsets_up_to(&self, byte_offset: usize) -> Vec<usize> {
        self.text[..byte_offset]
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .collect()
    }

    /// Number of grapheme clusters before the cursor.
    pub fn cursor_grapheme_index(&self) -> usize {
        self.grapheme_offsets_up_to(self.cursor).len()
    }

    /// Move cursor one grapheme cluster left.
    pub fn move_left(&mut self) {
        self.clear_selection_for_movement();
        self.cursor = self.previous_grapheme_boundary(self.cursor);
    }

    /// Move cursor one grapheme cluster right.
    pub fn move_right(&mut self) {
        self.clear_selection_for_movement();
        self.cursor = self.next_grapheme_boundary(self.cursor);
    }

    /// Move cursor to start of text.
    pub fn move_start(&mut self) {
        self.clear_selection_for_movement();
        self.cursor = 0;
    }

    /// Move cursor to end of text.
    pub fn move_end(&mut self) {
        self.clear_selection_for_movement();
        self.cursor = self.text.len();
    }

    /// Move cursor to previous word boundary.
    pub fn move_word_left(&mut self) {
        self.clear_selection_for_movement();
        self.cursor = self.previous_word_boundary(self.cursor);
    }

    /// Move cursor to next word boundary.
    pub fn move_word_right(&mut self) {
        self.clear_selection_for_movement();
        self.cursor = self.next_word_boundary(self.cursor);
    }

    /// Select left one grapheme cluster.
    pub fn select_left(&mut self) {
        self.ensure_selection_anchor();
        self.cursor = self.previous_grapheme_boundary(self.cursor);
    }

    /// Select right one grapheme cluster.
    pub fn select_right(&mut self) {
        self.ensure_selection_anchor();
        self.cursor = self.next_grapheme_boundary(self.cursor);
    }

    /// Select to start of text.
    pub fn select_start(&mut self) {
        self.ensure_selection_anchor();
        self.cursor = 0;
    }

    /// Select to end of text.
    pub fn select_end(&mut self) {
        self.ensure_selection_anchor();
        self.cursor = self.text.len();
    }

    /// Select to previous word boundary.
    pub fn select_word_left(&mut self) {
        self.ensure_selection_anchor();
        self.cursor = self.previous_word_boundary(self.cursor);
    }

    /// Select to next word boundary.
    pub fn select_word_right(&mut self) {
        self.ensure_selection_anchor();
        self.cursor = self.next_word_boundary(self.cursor);
    }

    fn clear_selection_for_movement(&mut self) {
        self.select_anchor = None;
    }

    fn ensure_selection_anchor(&mut self) {
        if self.select_anchor.is_none() {
            self.select_anchor = Some(self.cursor);
        }
    }

    /// Insert text at the current cursor position (replaces selection if any).
    pub fn insert(&mut self, ch: char) {
        self.delete_selection();
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.select_anchor = None;
    }

    /// Backspace: delete one grapheme cluster before cursor, or delete selection.
    pub fn backspace(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else {
            let prev = self.previous_grapheme_boundary(self.cursor);
            self.text.drain(prev..self.cursor);
            self.cursor = prev;
        }
    }

    /// Delete: delete one grapheme cluster after cursor, or delete selection.
    pub fn delete(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else {
            let next = self.next_grapheme_boundary(self.cursor);
            self.text.drain(self.cursor..next);
        }
    }

    /// Delete from cursor to previous word boundary.
    pub fn delete_word_backward(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else {
            let word_start = self.previous_word_boundary(self.cursor);
            self.text.drain(word_start..self.cursor);
            self.cursor = word_start;
        }
    }

    /// Delete from cursor to next word boundary.
    pub fn delete_word_forward(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else {
            let word_end = self.next_word_boundary(self.cursor);
            self.text.drain(self.cursor..word_end);
        }
    }

    /// Select all text.
    pub fn select_all(&mut self) {
        self.select_anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// Copy selected text. Returns the selected text.
    pub fn copy_selection(&self) -> String {
        self.selected_text().to_string()
    }

    /// Cut selected text. Returns the cut text.
    pub fn cut_selection(&mut self) -> String {
        let deleted = self.delete_selection();
        deleted
    }

    /// Paste text at cursor (replaces selection).
    pub fn paste(&mut self, text: &str) {
        let replaced = self.delete_selection();
        let start = self.cursor;
        let new_end = start + text.len();
        self.text.insert_str(start, text);
        self.cursor = new_end;
        self.select_anchor = None;
        let _ = replaced;
    }

    // ── Boundary helpers ──────────────────────────────────────

    fn previous_grapheme_boundary(&self, offset: usize) -> usize {
        let grapheme_offsets: Vec<usize> = self.text.grapheme_indices(true).map(|(i, _)| i).collect();
        if grapheme_offsets.is_empty() {
            return 0;
        }
        let mut prev = 0;
        for &g_offset in &grapheme_offsets {
            if g_offset >= offset {
                return prev;
            }
            prev = g_offset;
        }
        prev
    }

    fn next_grapheme_boundary(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .find_map(|(i, s)| {
                if i > offset {
                    Some(i + s.len())
                } else if i == offset {
                    Some(i + s.len())
                } else {
                    None
                }
            })
            .unwrap_or(self.text.len())
    }

    fn previous_word_boundary(&self, offset: usize) -> usize {
        let left_part = &self.text[..offset];
        UnicodeSegmentation::split_word_bound_indices(left_part)
            .rfind(|(_, s)| s.chars().any(|c| !c.is_whitespace()))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_word_boundary(&self, offset: usize) -> usize {
        let right_part = &self.text[offset..];
        UnicodeSegmentation::split_word_bound_indices(right_part)
            .find(|(_, s)| s.chars().any(|c| !c.is_whitespace()))
            .map(|(i, s)| offset + i + s.len())
            .unwrap_or(self.text.len())
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}
