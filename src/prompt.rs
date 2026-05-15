//! Command-mode prompt state. A bounded edit buffer + cursor + a submitted-
//! command history with shell-style ↑/↓ recall. Unicode-safe: cursor is a char
//! index; byte offsets are computed only at mutation sites.

use std::collections::VecDeque;

const HISTORY_CAP: usize = 64;

/// State for the slash-prompt at the bottom of the Status view. The buffer
/// does **not** include the leading `/` — that's a render-time prefix.
pub struct Prompt {
    pub buffer: String,
    pub cursor: usize,
    history: VecDeque<String>,
    history_idx: Option<usize>,
    saved: Option<String>,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            history: VecDeque::new(),
            history_idx: None,
            saved: None,
        }
    }

    /// Submit the current buffer: returns the trimmed input (caller dispatches),
    /// records the raw buffer into history if non-empty, then clears.
    pub fn submit(&mut self) -> String {
        let raw = std::mem::take(&mut self.buffer);
        self.cursor = 0;
        self.history_idx = None;
        self.saved = None;
        if !raw.trim().is_empty() {
            // Drop a consecutive duplicate so ↑ doesn't repeat the same entry.
            if self.history.back().map(String::as_str) != Some(raw.as_str()) {
                if self.history.len() == HISTORY_CAP {
                    self.history.pop_front();
                }
                self.history.push_back(raw.clone());
            }
        }
        raw
    }

    // --- editing --------------------------------------------------------

    pub fn insert_char(&mut self, c: char) {
        self.detach_from_history();
        let i = byte_index(&self.buffer, self.cursor);
        self.buffer.insert(i, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        self.detach_from_history();
        if self.cursor == 0 {
            return;
        }
        let start = byte_index(&self.buffer, self.cursor - 1);
        let end = byte_index(&self.buffer, self.cursor);
        self.buffer.replace_range(start..end, "");
        self.cursor -= 1;
    }

    pub fn delete_at_cursor(&mut self) {
        self.detach_from_history();
        let len = char_len(&self.buffer);
        if self.cursor >= len {
            return;
        }
        let start = byte_index(&self.buffer, self.cursor);
        let end = byte_index(&self.buffer, self.cursor + 1);
        self.buffer.replace_range(start..end, "");
    }

    // --- movement -------------------------------------------------------

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < char_len(&self.buffer) {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = char_len(&self.buffer);
    }

    // --- history recall -------------------------------------------------

    pub fn recall_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let new_idx = match self.history_idx {
            None => {
                self.saved = Some(self.buffer.clone());
                self.history.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_idx = Some(new_idx);
        self.buffer = self.history[new_idx].clone();
        self.cursor = char_len(&self.buffer);
    }

    pub fn recall_next(&mut self) {
        match self.history_idx {
            None => {}
            Some(i) if i + 1 < self.history.len() => {
                self.history_idx = Some(i + 1);
                self.buffer = self.history[i + 1].clone();
                self.cursor = char_len(&self.buffer);
            }
            Some(_) => {
                self.history_idx = None;
                self.buffer = self.saved.take().unwrap_or_default();
                self.cursor = char_len(&self.buffer);
            }
        }
    }

    fn detach_from_history(&mut self) {
        self.history_idx = None;
        self.saved = None;
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

/// Split a command line into tokens, honoring double-quoted strings. Backslash
/// inside quotes escapes the next character (so `\"` and `\\` work). Single
/// quotes are not special. This is a small subset of POSIX shell quoting,
/// sufficient for things like `git commit -m "wip change"`.
pub fn shell_split(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut started = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                started = true;
            }
            '\\' if in_quotes => {
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            c if c.is_whitespace() && !in_quotes => {
                if started {
                    out.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_(p: &mut Prompt, text: &str) {
        for c in text.chars() {
            p.insert_char(c);
        }
    }

    #[test]
    fn insert_and_submit() {
        let mut p = Prompt::new();
        type_(&mut p, "git status");
        assert_eq!(p.buffer, "git status");
        assert_eq!(p.cursor, 10);
        let s = p.submit();
        assert_eq!(s, "git status");
        assert_eq!(p.buffer, "");
        assert_eq!(p.cursor, 0);
    }

    #[test]
    fn submit_records_into_history() {
        let mut p = Prompt::new();
        type_(&mut p, "git status");
        p.submit();
        type_(&mut p, "git add foo");
        p.submit();
        p.recall_prev();
        assert_eq!(p.buffer, "git add foo");
        p.recall_prev();
        assert_eq!(p.buffer, "git status");
        p.recall_prev();
        assert_eq!(p.buffer, "git status"); // pinned at oldest
    }

    #[test]
    fn recall_next_restores_saved_buffer() {
        let mut p = Prompt::new();
        type_(&mut p, "git status");
        p.submit();
        type_(&mut p, "wip");
        p.recall_prev();
        assert_eq!(p.buffer, "git status");
        p.recall_next();
        assert_eq!(p.buffer, "wip");
        assert_eq!(p.cursor, 3);
    }

    #[test]
    fn recall_next_with_no_recall_is_noop() {
        let mut p = Prompt::new();
        type_(&mut p, "wip");
        p.recall_next();
        assert_eq!(p.buffer, "wip");
    }

    #[test]
    fn duplicate_consecutive_submits_collapse() {
        let mut p = Prompt::new();
        type_(&mut p, "git status");
        p.submit();
        type_(&mut p, "git status");
        p.submit();
        assert_eq!(p.history.len(), 1);
    }

    #[test]
    fn blank_submit_does_not_record() {
        let mut p = Prompt::new();
        type_(&mut p, "   ");
        let s = p.submit();
        assert_eq!(s, "   ");
        assert!(p.history.is_empty());
    }

    #[test]
    fn editing_detaches_from_history() {
        let mut p = Prompt::new();
        type_(&mut p, "git status");
        p.submit();
        type_(&mut p, "git diff");
        p.submit();
        p.recall_prev(); // "git diff"
        type_(&mut p, "!");
        assert_eq!(p.buffer, "git diff!");
        // ↓ is a no-op once we've edited (detached from history navigation).
        p.recall_next();
        assert_eq!(p.buffer, "git diff!");
        // ↑ from a detached buffer treats the edit as a fresh baseline:
        // it's saved, then the latest history entry is pulled in.
        p.recall_prev();
        assert_eq!(p.buffer, "git diff");
        p.recall_next();
        assert_eq!(p.buffer, "git diff!");
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut p = Prompt::new();
        p.backspace();
        assert_eq!(p.buffer, "");
        assert_eq!(p.cursor, 0);
    }

    #[test]
    fn backspace_removes_char_before_cursor() {
        let mut p = Prompt::new();
        type_(&mut p, "abc");
        p.move_left();
        p.backspace();
        assert_eq!(p.buffer, "ac");
        assert_eq!(p.cursor, 1);
    }

    #[test]
    fn move_clamps_at_buffer_bounds() {
        let mut p = Prompt::new();
        type_(&mut p, "ab");
        p.move_right();
        assert_eq!(p.cursor, 2);
        p.move_left();
        p.move_left();
        p.move_left();
        assert_eq!(p.cursor, 0);
    }

    #[test]
    fn unicode_buffer_indexing() {
        let mut p = Prompt::new();
        type_(&mut p, "héllo");
        assert_eq!(p.cursor, 5);
        p.backspace();
        assert_eq!(p.buffer, "héll");
        assert_eq!(p.cursor, 4);
        p.move_left();
        p.move_left();
        p.delete_at_cursor();
        assert_eq!(p.buffer, "hél");
    }

    #[test]
    fn shell_split_basic() {
        assert_eq!(shell_split("git status"), vec!["git", "status"]);
        assert_eq!(
            shell_split("git add -- src/a.rs"),
            vec!["git", "add", "--", "src/a.rs"]
        );
    }

    #[test]
    fn shell_split_quoted_args() {
        assert_eq!(
            shell_split(r#"git commit -m "wip change""#),
            vec!["git", "commit", "-m", "wip change"]
        );
    }

    #[test]
    fn shell_split_empty_quoted_string() {
        assert_eq!(
            shell_split(r#"git commit -m """#),
            vec!["git", "commit", "-m", ""]
        );
    }

    #[test]
    fn shell_split_backslash_escape_inside_quotes() {
        assert_eq!(
            shell_split(r#""a\"b" c"#),
            vec![r#"a"b"#.to_string(), "c".to_string()]
        );
    }

    #[test]
    fn shell_split_blank_is_empty() {
        assert!(shell_split("   ").is_empty());
        assert!(shell_split("").is_empty());
    }
}
