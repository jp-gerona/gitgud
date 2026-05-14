use crate::action::Action;
use crate::commit_editor::{CommitEditor, EditorMode, SubmitIntent};
use crate::event::{self, AppEvent};
use crate::git::{self, FileStatus};
use crate::history::History;
use crate::keymap;
use crate::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::Stdout;
use std::time::Duration;

type Term = Terminal<CrosstermBackend<Stdout>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Unstaged,
    Staged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Status,
    CommitEditor,
}

pub struct App {
    pub status: git::StatusList,
    pub focused: Pane,
    pub unstaged_selected: usize,
    pub staged_selected: usize,
    pub diff: String,
    pub history: History,
    pub should_quit: bool,
    pub error: Option<String>,
    pub view: View,
    pub commit_editor: CommitEditor,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut app = Self {
            status: git::StatusList::default(),
            focused: Pane::Unstaged,
            unstaged_selected: 0,
            staged_selected: 0,
            diff: String::new(),
            history: History::default(),
            should_quit: false,
            error: None,
            view: View::Status,
            commit_editor: CommitEditor::new(),
        };
        app.refresh_status();
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut Term) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| ui::draw(f, self))?;
            if let Some(ev) = event::poll(Duration::from_millis(200))? {
                self.handle_event(ev);
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, ev: AppEvent) {
        if let AppEvent::Key(k) = ev
            && k.kind != KeyEventKind::Release
        {
            self.handle_key(k);
        }
    }

    fn handle_key(&mut self, k: KeyEvent) {
        match self.view {
            View::Status => self.handle_status_key(k),
            View::CommitEditor => self.handle_commit_editor_key(k),
        }
    }

    fn handle_status_key(&mut self, k: KeyEvent) {
        let Some(action) = keymap::key_to_action(k) else {
            return;
        };
        match action {
            Action::Quit => self.should_quit = true,
            Action::MoveSelection(d) => self.move_selection(d),
            Action::SwitchPane => {
                self.focused = match self.focused {
                    Pane::Unstaged => Pane::Staged,
                    Pane::Staged => Pane::Unstaged,
                };
                self.refresh_diff();
            }
            Action::Refresh => self.refresh_status(),
            Action::StageSelected => self.stage_selected(),
            Action::UnstageSelected => self.unstage_selected(),
            Action::Commit => self.open_commit_editor(),
            Action::Dismiss => self.error = None,
        }
    }

    // --- commit editor ---------------------------------------------------

    fn handle_commit_editor_key(&mut self, k: KeyEvent) {
        // Ctrl-C always quits gitgud, even from inside the editor. Esc is the
        // vim-style "back to normal mode" / "cancel command" key.
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        let in_normal = matches!(self.commit_editor.mode, EditorMode::Normal);
        let in_insert = matches!(self.commit_editor.mode, EditorMode::Insert);
        if in_normal {
            self.handle_normal_mode_key(k);
        } else if in_insert {
            self.handle_insert_mode_key(k);
        } else {
            self.handle_command_mode_key(k);
        }
    }

    fn handle_normal_mode_key(&mut self, k: KeyEvent) {
        if let KeyCode::Char(c) = k.code {
            // Resolve pending operator (gg, dd, dw). On miss, swallow the keypress.
            if let Some(prev) = self.commit_editor.pending_op.take() {
                match (prev, c) {
                    ('g', 'g') => self.commit_editor.goto_top(),
                    ('d', 'd') => self.commit_editor.delete_line(),
                    ('d', 'w') => self.commit_editor.delete_word_forward(),
                    _ => {}
                }
                return;
            }
            match c {
                // Movement
                'h' => self.commit_editor.move_left(),
                'j' => self.commit_editor.move_down(),
                'k' => self.commit_editor.move_up(),
                'l' => self.commit_editor.move_right(),
                '0' => self.commit_editor.move_line_start(),
                '$' => self.commit_editor.move_line_end(),
                'w' => self.commit_editor.move_word_forward(),
                'b' => self.commit_editor.move_word_back(),
                'G' => self.commit_editor.goto_bottom(),
                // Two-key operators — set pending and wait for the next key.
                'g' => self.commit_editor.pending_op = Some('g'),
                'd' => self.commit_editor.pending_op = Some('d'),
                // Insert-mode entries
                'i' => self.commit_editor.enter_insert(),
                'a' => self.commit_editor.enter_insert_after(),
                'I' => self.commit_editor.enter_insert_line_start(),
                'A' => self.commit_editor.enter_insert_line_end(),
                'o' => self.commit_editor.open_line_below(),
                'O' => self.commit_editor.open_line_above(),
                // Normal-mode deletes
                'x' => self.commit_editor.delete_at_cursor(),
                'D' => self.commit_editor.delete_to_end_of_line(),
                // Enter command mode
                ':' => self.commit_editor.enter_command(),
                _ => {}
            }
            return;
        }
        match k.code {
            KeyCode::Esc => self.commit_editor.clear_status(),
            // Arrow keys also work in normal mode for newcomers.
            KeyCode::Left => self.commit_editor.move_left(),
            KeyCode::Right => self.commit_editor.move_right(),
            KeyCode::Up => self.commit_editor.move_up(),
            KeyCode::Down => self.commit_editor.move_down(),
            KeyCode::Home => self.commit_editor.move_line_start(),
            KeyCode::End => self.commit_editor.move_line_end(),
            _ => {}
        }
    }

    fn handle_insert_mode_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc => self.commit_editor.enter_normal(),
            KeyCode::Enter => {
                self.commit_editor.insert_newline();
                self.commit_editor.clear_status();
            }
            KeyCode::Backspace => {
                self.commit_editor.backspace();
                self.commit_editor.clear_status();
            }
            KeyCode::Delete => {
                self.commit_editor.delete_at_cursor();
                self.commit_editor.clear_status();
            }
            KeyCode::Left => self.commit_editor.move_left(),
            KeyCode::Right => self.commit_editor.move_right(),
            KeyCode::Up => self.commit_editor.move_up(),
            KeyCode::Down => self.commit_editor.move_down(),
            KeyCode::Home => self.commit_editor.move_line_start(),
            KeyCode::End => self.commit_editor.move_line_end(),
            KeyCode::Char(c) => {
                self.commit_editor.insert_char(c);
                self.commit_editor.clear_status();
            }
            _ => {}
        }
    }

    fn handle_command_mode_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc => self.commit_editor.cancel_command(),
            KeyCode::Enter => {
                let intent = self.commit_editor.execute_command();
                match intent {
                    SubmitIntent::Commit => self.submit_commit(),
                    SubmitIntent::Cancel => self.cancel_commit(),
                    SubmitIntent::None => {}
                }
            }
            KeyCode::Backspace => self.commit_editor.command_backspace(),
            KeyCode::Char(c) => self.commit_editor.command_append(c),
            _ => {}
        }
    }

    fn open_commit_editor(&mut self) {
        if self.status.staged().count() == 0 {
            self.error = Some(
                "nothing staged to commit (press 's' on an unstaged file, [Esc] to dismiss)"
                    .into(),
            );
            return;
        }
        self.error = None;
        self.commit_editor = CommitEditor::new();
        self.view = View::CommitEditor;
    }

    fn cancel_commit(&mut self) {
        self.commit_editor = CommitEditor::new();
        self.view = View::Status;
        self.error = None;
    }

    fn submit_commit(&mut self) {
        if self.commit_editor.is_blank() {
            self.commit_editor.status_message =
                Some("Aborting commit due to empty commit message (use :q! to discard)".into());
            return;
        }
        let message = self.commit_editor.message();
        let cmd = git::GitCmd::new("commit").arg("-F").arg("-");
        let display = cmd.display();
        match git::runner::run_with_stdin(&cmd, message.as_bytes()) {
            Ok(out) if out.success() => {
                // Success: drop the editor and return to status.
                self.commit_editor = CommitEditor::new();
                self.view = View::Status;
                self.error = None;
            }
            Ok(out) => {
                let stderr = out.stderr_str().trim().to_string();
                self.commit_editor.status_message = Some(if stderr.is_empty() {
                    let code = out
                        .status
                        .code()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "signal".into());
                    format!("commit failed (exit {code})")
                } else {
                    format!("commit failed: {stderr}")
                });
            }
            Err(e) => {
                self.commit_editor.status_message = Some(format!("{display}: {e}"));
            }
        }
        self.refresh_status();
        self.history.record(&display);
    }

    // --- status-view actions --------------------------------------------

    fn stage_selected(&mut self) {
        if self.focused != Pane::Unstaged {
            return;
        }
        let Some(path) = self.selected_entry().map(|e| e.path.clone()) else {
            return;
        };
        self.run_action(git::GitCmd::new("add").arg("--").arg(&path));
    }

    fn unstage_selected(&mut self) {
        if self.focused != Pane::Staged {
            return;
        }
        let Some(path) = self.selected_entry().map(|e| e.path.clone()) else {
            return;
        };
        self.run_action(
            git::GitCmd::new("restore")
                .arg("--staged")
                .arg("--")
                .arg(&path),
        );
    }

    fn run_action(&mut self, cmd: git::GitCmd) {
        let display = cmd.display();
        match git::runner::run(&cmd) {
            Ok(out) if !out.success() => {
                self.error = Some(format!("{}: {}", display, out.stderr_str().trim()));
            }
            Ok(_) => self.error = None,
            Err(e) => self.error = Some(format!("{}: {}", display, e)),
        }
        self.refresh_status();
        self.history.record(&display);
    }

    fn move_selection(&mut self, delta: i32) {
        let len = match self.focused {
            Pane::Unstaged => self.status.unstaged().count(),
            Pane::Staged => self.status.staged().count(),
        };
        let sel = match self.focused {
            Pane::Unstaged => &mut self.unstaged_selected,
            Pane::Staged => &mut self.staged_selected,
        };
        if len == 0 {
            *sel = 0;
        } else {
            let new = (*sel as i32 + delta).clamp(0, (len - 1) as i32);
            *sel = new as usize;
        }
        self.refresh_diff();
    }

    pub fn refresh_status(&mut self) {
        let cmd = git::status::cmd();
        self.history.record(&cmd.display());
        match git::status::load() {
            Ok(s) => {
                self.status = s;
                let u = self.status.unstaged().count();
                let st = self.status.staged().count();
                self.unstaged_selected = clamp_sel(self.unstaged_selected, u);
                self.staged_selected = clamp_sel(self.staged_selected, st);
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        self.refresh_diff();
    }

    pub fn selected_entry(&self) -> Option<&git::FileEntry> {
        match self.focused {
            Pane::Unstaged => self.status.unstaged().nth(self.unstaged_selected),
            Pane::Staged => self.status.staged().nth(self.staged_selected),
        }
    }

    pub fn refresh_diff(&mut self) {
        let info = self
            .selected_entry()
            .map(|e| (e.path.clone(), matches!(e.index, FileStatus::Untracked)));
        let Some((path, is_untracked)) = info else {
            self.diff = String::new();
            return;
        };

        let cached = matches!(self.focused, Pane::Staged);
        let cmd = if is_untracked {
            git::GitCmd::new("diff")
                .arg("--no-index")
                .arg("--")
                .arg("/dev/null")
                .arg(&path)
        } else if cached {
            git::GitCmd::new("diff").arg("--cached").arg("--").arg(&path)
        } else {
            git::GitCmd::new("diff").arg("--").arg(&path)
        };

        self.history.record(&cmd.display());
        match git::runner::run(&cmd) {
            Ok(out) => {
                if !out.stdout.is_empty() {
                    self.diff = out.stdout_str().into_owned();
                } else if !out.success() {
                    self.diff = format!("(git diff exited {})\n{}", out.status, out.stderr_str());
                } else {
                    self.diff = String::new();
                }
            }
            Err(e) => self.diff = format!("(error: {})", e),
        }
    }

    pub fn last_command(&self) -> Option<&str> {
        self.history.last()
    }
}

fn clamp_sel(sel: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else if sel >= len {
        len - 1
    } else {
        sel
    }
}
