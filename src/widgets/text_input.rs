use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A single-line text input with full readline-style cursor movement.
///
/// The cursor is stored as a byte offset into `value`. All operations
/// convert to char indices internally to handle multi-byte UTF-8 correctly.
#[derive(Debug, Default, Clone)]
pub struct TextInput {
    value: String,
    /// Byte offset of the cursor within `value`.
    cursor: usize,
}

impl TextInput {
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Cursor position as a byte offset (for rendering).
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Cursor position as a char index (for rendering a visual cursor).
    pub fn cursor_char_index(&self) -> usize {
        self.value[..self.cursor].chars().count()
    }

    /// Insert a string at the cursor (e.g. from a paste event).
    /// Control characters are skipped; newlines/carriage returns stop insertion
    /// so an accidental trailing newline doesn't submit a form.
    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            if !ch.is_control() {
                self.value.insert(self.cursor, ch);
                self.cursor += ch.len_utf8();
            }
        }
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn set_value(&mut self, s: &str) {
        self.value = s.to_owned();
        self.cursor = self.value.len();
    }

    // --- Private helpers ---

    fn char_count(&self) -> usize {
        self.value.chars().count()
    }

    /// Convert a char index to a byte offset.
    fn char_to_byte(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.value.len())
    }

    /// Convert the cursor byte offset to a char index.
    fn cursor_to_char(&self) -> usize {
        self.value[..self.cursor].chars().count()
    }

    fn set_cursor_from_char(&mut self, char_idx: usize) {
        self.cursor = self.char_to_byte(char_idx.min(self.char_count()));
    }

    /// Char index of the start of the word before the cursor (for Alt+B / Ctrl+Left / Ctrl+W).
    fn prev_word_char(&self) -> usize {
        let chars: Vec<char> = self.value.chars().collect();
        let mut i = self.cursor_to_char();
        // Skip trailing non-alphanumeric
        while i > 0 && !chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        // Skip word characters
        while i > 0 && chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        i
    }

    /// Char index of the end of the word after the cursor (for Alt+F / Ctrl+Right).
    fn next_word_char(&self) -> usize {
        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let mut i = self.cursor_to_char();
        // Skip leading non-alphanumeric
        while i < len && !chars[i].is_alphanumeric() {
            i += 1;
        }
        // Skip word characters
        while i < len && chars[i].is_alphanumeric() {
            i += 1;
        }
        i
    }

    /// Handle a key event. Returns `true` if the key was consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        match (key.code, ctrl, alt) {
            // Cursor to start
            (KeyCode::Home, _, _) | (KeyCode::Char('a'), true, false) => {
                self.cursor = 0;
            }

            // Cursor to end
            (KeyCode::End, _, _) | (KeyCode::Char('e'), true, false) => {
                self.cursor = self.value.len();
            }

            // Cursor left one char (Ctrl+B or plain Left)
            (KeyCode::Left, false, false) | (KeyCode::Char('b'), true, false) => {
                let ci = self.cursor_to_char();
                self.set_cursor_from_char(ci.saturating_sub(1));
            }

            // Cursor right one char (Ctrl+F or plain Right)
            (KeyCode::Right, false, false) | (KeyCode::Char('f'), true, false) => {
                let ci = self.cursor_to_char();
                self.set_cursor_from_char((ci + 1).min(self.char_count()));
            }

            // Cursor left one word (Alt+B or Ctrl+Left)
            (KeyCode::Left, true, false) | (KeyCode::Char('b'), false, true) => {
                let i = self.prev_word_char();
                self.set_cursor_from_char(i);
            }

            // Cursor right one word (Alt+F or Ctrl+Right)
            (KeyCode::Right, true, false) | (KeyCode::Char('f'), false, true) => {
                let i = self.next_word_char();
                self.set_cursor_from_char(i);
            }

            // Delete char before cursor
            (KeyCode::Backspace, false, false) => {
                if self.cursor > 0 {
                    let ci = self.cursor_to_char();
                    let byte_start = self.char_to_byte(ci - 1);
                    self.value.remove(byte_start);
                    self.cursor = byte_start;
                }
            }

            // Delete char at cursor
            (KeyCode::Delete, false, false) => {
                if self.cursor < self.value.len() {
                    self.value.remove(self.cursor);
                }
            }

            // Delete word before cursor (Ctrl+W)
            (KeyCode::Char('w'), true, false) => {
                let end = self.cursor;
                let start_ci = self.prev_word_char();
                let start = self.char_to_byte(start_ci);
                if start < end {
                    self.value.drain(start..end);
                    self.cursor = start;
                }
            }

            // Delete from start to cursor (Ctrl+U)
            (KeyCode::Char('u'), true, false) => {
                self.value.drain(0..self.cursor);
                self.cursor = 0;
            }

            // Delete from cursor to end (Ctrl+K)
            (KeyCode::Char('k'), true, false) => {
                self.value.truncate(self.cursor);
            }

            // Insert printable character
            (KeyCode::Char(c), false, false) => {
                self.value.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }

            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn alt(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::ALT)
    }

    fn type_str(input: &mut TextInput, s: &str) {
        for c in s.chars() {
            input.handle_key(key(KeyCode::Char(c)));
        }
    }

    #[test]
    fn type_and_read() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        assert_eq!(t.value(), "hello");
        assert_eq!(t.cursor(), 5);
    }

    #[test]
    fn backspace_deletes_before_cursor() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(key(KeyCode::Backspace));
        assert_eq!(t.value(), "hell");
        assert_eq!(t.cursor(), 4);
    }

    #[test]
    fn backspace_at_start_is_no_op() {
        let mut t = TextInput::default();
        type_str(&mut t, "hi");
        t.handle_key(ctrl(KeyCode::Char('a'))); // go to start
        t.handle_key(key(KeyCode::Backspace));
        assert_eq!(t.value(), "hi");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn delete_removes_char_at_cursor() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(ctrl(KeyCode::Char('a'))); // cursor to start
        t.handle_key(key(KeyCode::Delete));
        assert_eq!(t.value(), "ello");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn delete_at_end_is_no_op() {
        let mut t = TextInput::default();
        type_str(&mut t, "hi");
        t.handle_key(key(KeyCode::Delete));
        assert_eq!(t.value(), "hi");
    }

    #[test]
    fn ctrl_a_moves_to_start() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(ctrl(KeyCode::Char('a')));
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn home_moves_to_start() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(key(KeyCode::Home));
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn ctrl_e_moves_to_end() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(ctrl(KeyCode::Char('a')));
        t.handle_key(ctrl(KeyCode::Char('e')));
        assert_eq!(t.cursor(), 5);
    }

    #[test]
    fn end_moves_to_end() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(key(KeyCode::Home));
        t.handle_key(key(KeyCode::End));
        assert_eq!(t.cursor(), 5);
    }

    #[test]
    fn left_moves_cursor_back() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(key(KeyCode::Left));
        assert_eq!(t.cursor(), 4);
    }

    #[test]
    fn right_moves_cursor_forward() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(key(KeyCode::Home));
        t.handle_key(key(KeyCode::Right));
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn ctrl_b_moves_left() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(ctrl(KeyCode::Char('b')));
        assert_eq!(t.cursor(), 4);
    }

    #[test]
    fn ctrl_f_moves_right() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(key(KeyCode::Home));
        t.handle_key(ctrl(KeyCode::Char('f')));
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn alt_b_moves_back_one_word() {
        let mut t = TextInput::default();
        type_str(&mut t, "foo bar");
        t.handle_key(alt(KeyCode::Char('b')));
        assert_eq!(t.cursor(), 4); // before "bar"
    }

    #[test]
    fn alt_f_moves_forward_one_word() {
        let mut t = TextInput::default();
        type_str(&mut t, "foo bar");
        t.handle_key(key(KeyCode::Home));
        t.handle_key(alt(KeyCode::Char('f')));
        assert_eq!(t.cursor(), 3); // after "foo"
    }

    #[test]
    fn ctrl_left_moves_back_one_word() {
        let mut t = TextInput::default();
        type_str(&mut t, "foo bar");
        t.handle_key(ctrl(KeyCode::Left));
        assert_eq!(t.cursor(), 4);
    }

    #[test]
    fn ctrl_right_moves_forward_one_word() {
        let mut t = TextInput::default();
        type_str(&mut t, "foo bar");
        t.handle_key(key(KeyCode::Home));
        t.handle_key(ctrl(KeyCode::Right));
        assert_eq!(t.cursor(), 3);
    }

    #[test]
    fn ctrl_w_deletes_word_before_cursor() {
        let mut t = TextInput::default();
        type_str(&mut t, "foo bar");
        t.handle_key(ctrl(KeyCode::Char('w')));
        assert_eq!(t.value(), "foo ");
        assert_eq!(t.cursor(), 4);
    }

    #[test]
    fn ctrl_w_at_start_is_no_op() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.handle_key(key(KeyCode::Home));
        t.handle_key(ctrl(KeyCode::Char('w')));
        assert_eq!(t.value(), "hello");
    }

    #[test]
    fn ctrl_u_deletes_to_start() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello world");
        t.handle_key(ctrl(KeyCode::Left)); // before "world"
        t.handle_key(ctrl(KeyCode::Char('u')));
        assert_eq!(t.value(), "world");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn ctrl_k_deletes_to_end() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello world");
        t.handle_key(ctrl(KeyCode::Left)); // before "world"
        t.handle_key(ctrl(KeyCode::Char('k')));
        assert_eq!(t.value(), "hello ");
        assert_eq!(t.cursor(), 6);
    }

    #[test]
    fn clear_resets_everything() {
        let mut t = TextInput::default();
        type_str(&mut t, "hello");
        t.clear();
        assert_eq!(t.value(), "");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn set_value_puts_cursor_at_end() {
        let mut t = TextInput::default();
        t.set_value("hello");
        assert_eq!(t.value(), "hello");
        assert_eq!(t.cursor(), 5);
    }

    #[test]
    fn multibyte_utf8_cursor_stays_on_char_boundaries() {
        let mut t = TextInput::default();
        // "café" — 'é' is 2 bytes
        type_str(&mut t, "café");
        assert_eq!(t.value(), "café");
        // cursor is at byte 5: c(1) + a(1) + f(1) + é(2) = 5
        assert_eq!(t.cursor(), 5);

        t.handle_key(key(KeyCode::Backspace));
        assert_eq!(t.value(), "caf");
        assert_eq!(t.cursor(), 3);
    }

    #[test]
    fn insert_in_middle_of_string() {
        let mut t = TextInput::default();
        type_str(&mut t, "helo");
        // move cursor to before 'o'
        t.handle_key(key(KeyCode::Left));
        type_str(&mut t, "l");
        assert_eq!(t.value(), "hello");
    }

    #[test]
    fn unhandled_key_returns_false() {
        let mut t = TextInput::default();
        let consumed = t.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
        assert!(!consumed);
    }

    #[test]
    fn insert_str_pastes_text_at_cursor() {
        let mut t = TextInput::default();
        t.insert_str("https://example.com/cat.gif");
        assert_eq!(t.value(), "https://example.com/cat.gif");
        assert_eq!(t.cursor(), "https://example.com/cat.gif".len());
    }

    #[test]
    fn insert_str_stops_at_newline() {
        let mut t = TextInput::default();
        t.insert_str("hello\nworld");
        assert_eq!(t.value(), "hello");
    }

    #[test]
    fn insert_str_stops_at_carriage_return() {
        let mut t = TextInput::default();
        t.insert_str("hello\rworld");
        assert_eq!(t.value(), "hello");
    }

    #[test]
    fn insert_str_skips_control_chars() {
        let mut t = TextInput::default();
        t.insert_str("hel\x01lo");
        assert_eq!(t.value(), "hello");
    }

    #[test]
    fn insert_str_in_middle_of_existing_text() {
        let mut t = TextInput::default();
        type_str(&mut t, "helo");
        t.handle_key(key(KeyCode::Left)); // before 'o'
        t.insert_str("l");
        assert_eq!(t.value(), "hello");
    }

    #[test]
    fn cursor_char_index_matches_visual_position() {
        let mut t = TextInput::default();
        type_str(&mut t, "café");
        assert_eq!(t.cursor_char_index(), 4);
        t.handle_key(key(KeyCode::Left));
        assert_eq!(t.cursor_char_index(), 3);
    }
}
