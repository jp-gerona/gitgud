//! Multi-line text editor state for composing commit messages in-TUI, with
//! vi/vim-style modal editing. Cursor is tracked as `(row, char-col)` so the
//! column math stays Unicode-safe; byte indices are computed only when
//! mutating the underlying line strings.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    /// Buffer holds the chars typed after the leading `:`.
    Command(String),
}

/// Result of executing a `:command`. The app interprets the intent and either
/// runs the commit, cancels, or leaves the editor open with the status message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmitIntent {
    None,
    Commit,
    Cancel,
}

pub struct CommitEditor {
    pub lines: Vec<String>,
    pub row: usize,
    pub col: usize,
    pub mode: EditorMode,
    /// First key of a two-key sequence (e.g. `g` waiting for `g`, `d` waiting
    /// for `d`/`w`). Cleared after the second key resolves or any other action.
    pub pending_op: Option<char>,
    /// vim-style transient message shown in the editor's status row.
    pub status_message: Option<String>,
}

impl CommitEditor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            row: 0,
            col: 0,
            mode: EditorMode::Normal,
            pending_op: None,
            status_message: None,
        }
    }

    pub fn message(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_blank(&self) -> bool {
        self.lines.iter().all(|l| l.trim().is_empty())
    }

    // --- mode transitions ------------------------------------------------

    pub fn enter_normal(&mut self) {
        self.mode = EditorMode::Normal;
        self.pending_op = None;
    }

    pub fn enter_insert(&mut self) {
        self.mode = EditorMode::Insert;
        self.pending_op = None;
        self.status_message = None;
    }

    pub fn enter_insert_after(&mut self) {
        let len = char_len(&self.lines[self.row]);
        if self.col < len {
            self.col += 1;
        }
        self.enter_insert();
    }

    pub fn enter_insert_line_start(&mut self) {
        self.col = 0;
        self.enter_insert();
    }

    pub fn enter_insert_line_end(&mut self) {
        self.col = char_len(&self.lines[self.row]);
        self.enter_insert();
    }

    pub fn open_line_below(&mut self) {
        self.lines.insert(self.row + 1, String::new());
        self.row += 1;
        self.col = 0;
        self.enter_insert();
    }

    pub fn open_line_above(&mut self) {
        self.lines.insert(self.row, String::new());
        self.col = 0;
        self.enter_insert();
    }

    pub fn enter_command(&mut self) {
        self.mode = EditorMode::Command(String::new());
        self.pending_op = None;
        self.status_message = None;
    }

    pub fn cancel_command(&mut self) {
        // Esc from command mode → discard input, return to normal.
        // Leave status_message alone so prior messages remain visible.
        self.enter_normal();
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    // --- command-mode editing -------------------------------------------

    pub fn command_append(&mut self, c: char) {
        if let EditorMode::Command(ref mut s) = self.mode {
            s.push(c);
        }
    }

    pub fn command_backspace(&mut self) {
        if let EditorMode::Command(ref mut s) = self.mode
            && s.pop().is_none()
        {
            // Empty buffer + backspace → leave command mode (vim behavior).
            self.enter_normal();
        }
    }

    /// Execute the current command-mode input. Always exits command mode.
    pub fn execute_command(&mut self) -> SubmitIntent {
        let cmd = match &self.mode {
            EditorMode::Command(s) => s.clone(),
            _ => return SubmitIntent::None,
        };
        self.enter_normal();
        match cmd.as_str() {
            "" => SubmitIntent::None,
            "w" | "wq" | "x" => SubmitIntent::Commit,
            "q" => {
                if self.is_blank() {
                    SubmitIntent::Cancel
                } else {
                    self.status_message =
                        Some("E37: No write since last change (use :q! to force)".into());
                    SubmitIntent::None
                }
            }
            "q!" => SubmitIntent::Cancel,
            other => {
                self.status_message = Some(format!("E492: Not an editor command: {other}"));
                SubmitIntent::None
            }
        }
    }

    // --- text mutations (INSERT mode + a few normal-mode ops) -----------

    pub fn insert_char(&mut self, c: char) {
        let i = byte_index(&self.lines[self.row], self.col);
        self.lines[self.row].insert(i, c);
        self.col += 1;
    }

    pub fn insert_newline(&mut self) {
        let i = byte_index(&self.lines[self.row], self.col);
        let rest = self.lines[self.row].split_off(i);
        self.lines.insert(self.row + 1, rest);
        self.row += 1;
        self.col = 0;
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            let start = byte_index(&self.lines[self.row], self.col - 1);
            let end = byte_index(&self.lines[self.row], self.col);
            self.lines[self.row].replace_range(start..end, "");
            self.col -= 1;
        } else if self.row > 0 {
            let removed = self.lines.remove(self.row);
            self.row -= 1;
            self.col = char_len(&self.lines[self.row]);
            self.lines[self.row].push_str(&removed);
        }
    }

    /// `Delete` key in insert mode; also `x` in normal mode. Removes the char
    /// at the cursor; if the cursor is at end-of-line, joins the next line.
    /// Clamps cursor back if it would sit past the new last char.
    pub fn delete_at_cursor(&mut self) {
        let len = char_len(&self.lines[self.row]);
        if self.col < len {
            let start = byte_index(&self.lines[self.row], self.col);
            let end = byte_index(&self.lines[self.row], self.col + 1);
            self.lines[self.row].replace_range(start..end, "");
            let new_len = char_len(&self.lines[self.row]);
            if self.col >= new_len && new_len > 0 {
                self.col = new_len - 1;
            }
        } else if self.row + 1 < self.lines.len() {
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
        }
    }

    // --- movement -------------------------------------------------------

    pub fn move_left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        }
    }

    pub fn move_right(&mut self) {
        let len = char_len(&self.lines[self.row]);
        if self.col < len {
            self.col += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.col = self.col.min(char_len(&self.lines[self.row]));
        }
    }

    pub fn move_down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = self.col.min(char_len(&self.lines[self.row]));
        }
    }

    pub fn move_line_start(&mut self) {
        self.col = 0;
    }

    pub fn move_line_end(&mut self) {
        self.col = char_len(&self.lines[self.row]);
    }

    pub fn goto_top(&mut self) {
        self.row = 0;
        self.col = self.col.min(char_len(&self.lines[self.row]));
    }

    pub fn goto_bottom(&mut self) {
        self.row = self.lines.len().saturating_sub(1);
        self.col = self.col.min(char_len(&self.lines[self.row]));
    }

    pub fn move_word_forward(&mut self) {
        let chars: Vec<char> = self.lines[self.row].chars().collect();
        let len = chars.len();
        let mut col = self.col;
        if col < len {
            let starting = char_class(chars[col]);
            if starting != CharClass::Space {
                while col < len && char_class(chars[col]) == starting {
                    col += 1;
                }
            }
        }
        while col < len && chars[col].is_whitespace() {
            col += 1;
        }
        if col < len {
            self.col = col;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        } else {
            self.col = len;
        }
    }

    pub fn move_word_back(&mut self) {
        if self.col == 0 {
            if self.row > 0 {
                self.row -= 1;
                self.col = char_len(&self.lines[self.row]);
            }
            return;
        }
        let chars: Vec<char> = self.lines[self.row].chars().collect();
        let mut col = self.col - 1;
        while col > 0 && chars[col].is_whitespace() {
            col -= 1;
        }
        if col == 0 {
            self.col = 0;
            return;
        }
        let starting = char_class(chars[col]);
        while col > 0 && char_class(chars[col - 1]) == starting {
            col -= 1;
        }
        self.col = col;
    }

    // --- normal-mode delete ops -----------------------------------------

    pub fn delete_line(&mut self) {
        if self.lines.len() == 1 {
            self.lines[0].clear();
            self.col = 0;
        } else {
            self.lines.remove(self.row);
            if self.row >= self.lines.len() {
                self.row = self.lines.len() - 1;
            }
            self.col = self.col.min(char_len(&self.lines[self.row]));
        }
    }

    pub fn delete_to_end_of_line(&mut self) {
        let i = byte_index(&self.lines[self.row], self.col);
        self.lines[self.row].truncate(i);
        let new_len = char_len(&self.lines[self.row]);
        if self.col > 0 && self.col >= new_len {
            self.col = new_len.saturating_sub(1);
        }
    }

    pub fn delete_word_forward(&mut self) {
        let chars: Vec<char> = self.lines[self.row].chars().collect();
        let len = chars.len();
        let mut col = self.col;
        if col < len {
            let starting = char_class(chars[col]);
            if starting != CharClass::Space {
                while col < len && char_class(chars[col]) == starting {
                    col += 1;
                }
            }
        }
        while col < len && chars[col].is_whitespace() {
            col += 1;
        }
        let start_byte = byte_index(&self.lines[self.row], self.col);
        let end_byte = byte_index(&self.lines[self.row], col);
        self.lines[self.row].replace_range(start_byte..end_byte, "");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CharClass {
    Word,
    Punct,
    Space,
}

fn char_class(c: char) -> CharClass {
    if c.is_whitespace() {
        CharClass::Space
    } else if c.is_alphanumeric() || c == '_' {
        CharClass::Word
    } else {
        CharClass::Punct
    }
}

fn byte_index(s: &str, char_col: usize) -> usize {
    s.char_indices()
        .nth(char_col)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_(e: &mut CommitEditor, text: &str) {
        for c in text.chars() {
            e.insert_char(c);
        }
    }

    #[test]
    fn insert_simple_text() {
        let mut e = CommitEditor::new();
        type_(&mut e, "fix");
        assert_eq!(e.message(), "fix");
        assert_eq!((e.row, e.col), (0, 3));
    }

    #[test]
    fn newline_splits_line_and_moves_cursor() {
        let mut e = CommitEditor::new();
        type_(&mut e, "hello");
        e.col = 2;
        e.insert_newline();
        assert_eq!(e.lines, vec!["he".to_string(), "llo".to_string()]);
        assert_eq!((e.row, e.col), (1, 0));
    }

    #[test]
    fn backspace_at_line_start_joins_with_previous() {
        let mut e = CommitEditor::new();
        type_(&mut e, "ab");
        e.insert_newline();
        type_(&mut e, "cd");
        e.row = 1;
        e.col = 0;
        e.backspace();
        assert_eq!(e.message(), "abcd");
        assert_eq!((e.row, e.col), (0, 2));
    }

    #[test]
    fn cursor_clamps_when_moving_to_shorter_line() {
        let mut e = CommitEditor::new();
        type_(&mut e, "longer");
        e.insert_newline();
        type_(&mut e, "hi");
        e.row = 0;
        e.col = 6;
        e.move_down();
        assert_eq!((e.row, e.col), (1, 2));
    }

    #[test]
    fn blank_is_detected_with_only_whitespace() {
        let mut e = CommitEditor::new();
        assert!(e.is_blank());
        e.insert_char(' ');
        e.insert_newline();
        e.insert_char('\t');
        assert!(e.is_blank());
        e.insert_char('x');
        assert!(!e.is_blank());
    }

    #[test]
    fn enter_insert_after_advances_in_middle() {
        let mut e = CommitEditor::new();
        type_(&mut e, "abc");
        e.col = 1;
        e.enter_insert_after();
        assert_eq!(e.col, 2);
        assert_eq!(e.mode, EditorMode::Insert);
    }

    #[test]
    fn enter_insert_after_stays_at_line_end() {
        let mut e = CommitEditor::new();
        type_(&mut e, "abc");
        e.enter_insert_after();
        assert_eq!(e.col, 3);
    }

    #[test]
    fn open_line_below_inserts_blank_and_enters_insert() {
        let mut e = CommitEditor::new();
        type_(&mut e, "abc");
        e.col = 1;
        e.open_line_below();
        assert_eq!(e.lines, vec!["abc".to_string(), String::new()]);
        assert_eq!(e.row, 1);
        assert_eq!(e.col, 0);
        assert_eq!(e.mode, EditorMode::Insert);
    }

    #[test]
    fn open_line_above_pushes_current_down() {
        let mut e = CommitEditor::new();
        type_(&mut e, "abc");
        e.open_line_above();
        assert_eq!(e.lines, vec![String::new(), "abc".to_string()]);
        assert_eq!(e.row, 0);
        assert_eq!(e.mode, EditorMode::Insert);
    }

    #[test]
    fn delete_at_cursor_in_middle_keeps_col() {
        let mut e = CommitEditor::new();
        type_(&mut e, "abc");
        e.col = 1;
        e.delete_at_cursor();
        assert_eq!(e.lines[0], "ac");
        assert_eq!(e.col, 1);
    }

    #[test]
    fn delete_at_cursor_on_last_char_clamps_back() {
        let mut e = CommitEditor::new();
        type_(&mut e, "abc");
        e.col = 2;
        e.delete_at_cursor();
        assert_eq!(e.lines[0], "ab");
        assert_eq!(e.col, 1);
    }

    #[test]
    fn delete_line_when_only_line_clears_it() {
        let mut e = CommitEditor::new();
        type_(&mut e, "abc");
        e.delete_line();
        assert_eq!(e.lines, vec![String::new()]);
        assert_eq!((e.row, e.col), (0, 0));
    }

    #[test]
    fn delete_line_keeps_row_in_range() {
        let mut e = CommitEditor::new();
        type_(&mut e, "first");
        e.insert_newline();
        type_(&mut e, "second");
        e.delete_line();
        assert_eq!(e.lines, vec!["first".to_string()]);
        assert_eq!(e.row, 0);
    }

    #[test]
    fn delete_to_end_of_line_truncates_from_cursor() {
        let mut e = CommitEditor::new();
        type_(&mut e, "abcdef");
        e.col = 3;
        e.delete_to_end_of_line();
        assert_eq!(e.lines[0], "abc");
    }

    #[test]
    fn move_word_forward_skips_word_and_space() {
        let mut e = CommitEditor::new();
        type_(&mut e, "hello world foo");
        e.col = 0;
        e.move_word_forward();
        assert_eq!(e.col, 6);
        e.move_word_forward();
        assert_eq!(e.col, 12);
    }

    #[test]
    fn move_word_back_returns_to_previous_word() {
        let mut e = CommitEditor::new();
        type_(&mut e, "hello world");
        e.col = 11;
        e.move_word_back();
        assert_eq!(e.col, 6);
        e.move_word_back();
        assert_eq!(e.col, 0);
    }

    #[test]
    fn goto_top_and_bottom_span_lines() {
        let mut e = CommitEditor::new();
        type_(&mut e, "a");
        e.insert_newline();
        type_(&mut e, "b");
        e.insert_newline();
        type_(&mut e, "c");
        assert_eq!(e.row, 2);
        e.goto_top();
        assert_eq!(e.row, 0);
        e.goto_bottom();
        assert_eq!(e.row, 2);
    }

    #[test]
    fn command_wq_returns_commit_intent() {
        let mut e = CommitEditor::new();
        e.enter_command();
        e.command_append('w');
        e.command_append('q');
        assert_eq!(e.execute_command(), SubmitIntent::Commit);
        assert_eq!(e.mode, EditorMode::Normal);
    }

    #[test]
    fn command_q_bang_returns_cancel_intent() {
        let mut e = CommitEditor::new();
        e.enter_command();
        e.command_append('q');
        e.command_append('!');
        assert_eq!(e.execute_command(), SubmitIntent::Cancel);
    }

    #[test]
    fn command_q_on_blank_buffer_returns_cancel() {
        let mut e = CommitEditor::new();
        e.enter_command();
        e.command_append('q');
        assert_eq!(e.execute_command(), SubmitIntent::Cancel);
    }

    #[test]
    fn command_q_on_non_blank_buffer_sets_status_and_returns_none() {
        let mut e = CommitEditor::new();
        type_(&mut e, "wip");
        e.enter_command();
        e.command_append('q');
        assert_eq!(e.execute_command(), SubmitIntent::None);
        assert!(e.status_message.as_deref().unwrap().starts_with("E37"));
    }

    #[test]
    fn command_unknown_sets_status_and_returns_none() {
        let mut e = CommitEditor::new();
        e.enter_command();
        e.command_append('z');
        assert_eq!(e.execute_command(), SubmitIntent::None);
        assert!(e.status_message.as_deref().unwrap().starts_with("E492"));
    }

    #[test]
    fn command_backspace_on_empty_returns_to_normal() {
        let mut e = CommitEditor::new();
        e.enter_command();
        e.command_backspace();
        assert_eq!(e.mode, EditorMode::Normal);
    }
}
