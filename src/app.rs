use crate::action::Action;
use crate::commit_editor::{CommitEditor, EditorMode, SubmitIntent};
use crate::event::{self, AppEvent};
use crate::git::{self, FileStatus};
use crate::history::History;
use crate::keymap;
use crate::prompt::{self, Prompt};
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
    Log,
    CommitEditor,
}

impl View {
    /// True for "tabbed" views — anything other than the modal commit editor.
    /// Tab bar, command bar, status hints render on tabbed views; the commit
    /// editor takes over the full content area.
    pub fn is_tabbed(self) -> bool {
        matches!(self, View::Status | View::Log)
    }
}

pub struct App {
    pub status: git::StatusList,
    pub focused: Pane,
    pub unstaged_selected: usize,
    pub staged_selected: usize,
    pub diff: String,
    pub log: git::LogList,
    pub log_selected: usize,
    pub log_detail: String,
    pub history: History,
    pub should_quit: bool,
    pub error: Option<String>,
    pub view: View,
    pub commit_editor: CommitEditor,
    /// `Some` when the user is in slash-Command mode in a tabbed view.
    pub prompt: Option<Prompt>,
    /// `Some` when a destructive op is awaiting `y/N` confirmation. While set,
    /// all key input routes to the confirm handler.
    pub confirm: Option<PendingConfirm>,
}

/// A pending destructive operation. The `prompt` is what the user sees; the
/// `cmd` is what runs on `y`. Reusable beyond discard — same pattern will host
/// branch-delete, stash-drop, etc. when those land.
pub struct PendingConfirm {
    pub prompt: String,
    pub cmd: git::GitCmd,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut app = Self {
            status: git::StatusList::default(),
            focused: Pane::Unstaged,
            unstaged_selected: 0,
            staged_selected: 0,
            diff: String::new(),
            log: git::LogList::default(),
            log_selected: 0,
            log_detail: String::new(),
            history: History::default(),
            should_quit: false,
            error: None,
            view: View::Status,
            commit_editor: CommitEditor::new(),
            prompt: None,
            confirm: None,
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
        // Ctrl-C always quits, from any view including the prompt and editor.
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // A pending confirmation swallows all input until resolved. Reusable
        // across views — destructive ops set `confirm` and trust this gate.
        if self.confirm.is_some() {
            self.handle_confirm_key(k);
            return;
        }

        if matches!(self.view, View::CommitEditor) {
            self.handle_commit_editor_key(k);
            return;
        }

        // Tabbed views (Status, Log). Prompt mode swallows all input.
        if self.prompt.is_some() {
            self.handle_prompt_key(k);
            return;
        }

        // `/` enters slash-Command mode in any tabbed view.
        if k.modifiers.is_empty() && k.code == KeyCode::Char('/') {
            self.prompt = Some(Prompt::new());
            self.error = None;
            return;
        }

        // Tab-bar navigation (works on any tabbed view, Normal mode).
        if self.try_handle_tab_key(k) {
            return;
        }

        match self.view {
            View::Status => self.handle_status_normal_key(k),
            View::Log => self.handle_log_normal_key(k),
            View::CommitEditor => {}
        }
    }

    fn handle_status_normal_key(&mut self, k: KeyEvent) {
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
            Action::DiscardSelected => self.discard_selected(),
            Action::Commit => self.open_commit_editor(),
            Action::Dismiss => self.error = None,
        }
    }

    fn handle_log_normal_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_log_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_log_selection(-1),
            KeyCode::Char('g') => self.move_log_selection(i32::MIN),
            KeyCode::Char('G') => self.move_log_selection(i32::MAX),
            KeyCode::Char('r') => self.refresh_log(),
            KeyCode::Esc => self.error = None,
            _ => {}
        }
    }

    // --- confirmation ---------------------------------------------------

    fn handle_confirm_key(&mut self, k: KeyEvent) {
        // Only lowercase/uppercase `y` confirms — anything else cancels. This
        // mirrors the bash `[y/N]` convention where the capital letter is the
        // safe default.
        let confirmed = matches!(k.code, KeyCode::Char('y') | KeyCode::Char('Y'));
        let Some(pending) = self.confirm.take() else {
            return;
        };
        if confirmed {
            self.run_action(pending.cmd);
        }
        // Either way we clear; on cancel we keep `app.error` as-is.
    }

    // --- tab switching --------------------------------------------------

    fn try_handle_tab_key(&mut self, k: KeyEvent) -> bool {
        if !k.modifiers.is_empty() {
            return false;
        }
        let target = match k.code {
            KeyCode::Char('1') => Some(View::Status),
            KeyCode::Char('2') => Some(View::Log),
            KeyCode::Char(']') => Some(self.next_tab()),
            KeyCode::Char('[') => Some(self.prev_tab()),
            _ => None,
        };
        if let Some(t) = target {
            self.switch_view(t);
            return true;
        }
        false
    }

    fn next_tab(&self) -> View {
        match self.view {
            View::Status => View::Log,
            View::Log => View::Status,
            View::CommitEditor => self.view,
        }
    }

    fn prev_tab(&self) -> View {
        // Two tabs, so prev == next.
        self.next_tab()
    }

    /// Switch the active tab. Refreshes the target view's data so counts and
    /// content reflect the latest repo state on every entry.
    pub fn switch_view(&mut self, target: View) {
        if self.view == target {
            return;
        }
        self.view = target;
        match target {
            View::Status => self.refresh_status(),
            View::Log => self.refresh_log(),
            View::CommitEditor => {}
        }
    }

    // --- command-mode prompt --------------------------------------------

    fn handle_prompt_key(&mut self, k: KeyEvent) {
        let Some(p) = self.prompt.as_mut() else {
            return;
        };
        match k.code {
            KeyCode::Esc => {
                self.prompt = None;
            }
            KeyCode::Enter => {
                let raw = p.submit();
                self.dispatch_prompt(raw);
            }
            KeyCode::Backspace => p.backspace(),
            KeyCode::Delete => p.delete_at_cursor(),
            KeyCode::Left => p.move_left(),
            KeyCode::Right => p.move_right(),
            KeyCode::Home => p.move_home(),
            KeyCode::End => p.move_end(),
            KeyCode::Up => p.recall_prev(),
            KeyCode::Down => p.recall_next(),
            KeyCode::Char(c) => p.insert_char(c),
            _ => {}
        }
    }

    /// Parse and route a submitted prompt buffer (without the leading `/`).
    ///
    /// Rules:
    /// - blank → no-op
    /// - `exit` / `quit` → quit gitgud (gitgud-internal slash commands)
    /// - first token must otherwise be `git` — anything else is unknown
    /// - `git log` / `git status` switch to the matching tab (canonical query
    ///   runs; user's args show in the bar but don't shape the view — tracked
    ///   in the GH issue for arg-honoring)
    /// - `git commit` with no `-m`/`-F`/`--message`/`--file` opens the modal
    ///   commit editor (would otherwise spawn `$EDITOR`)
    /// - `git rebase -i` / `git add -p` are rejected until those views ship
    /// - everything else is built into a `GitCmd` and run via `run_action`
    fn dispatch_prompt(&mut self, raw: String) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return;
        }
        let args = prompt::shell_split(trimmed);
        if args.is_empty() {
            return;
        }

        // Built-in slash commands (no `git` prefix). Future: /help, /config, ...
        match args[0].as_str() {
            "exit" | "quit" => {
                self.should_quit = true;
                return;
            }
            _ => {}
        }

        if args[0] != "git" {
            self.error = Some(format!(
                "unknown command: /{trimmed} (try /git ..., /exit, or /quit)"
            ));
            return;
        }
        let rest = &args[1..];
        if rest.is_empty() {
            self.error = Some("missing git subcommand (e.g. /git status)".into());
            return;
        }
        let sub = rest[0].as_str();
        let tail = &rest[1..];

        // Editor-takeover intercept: commit-without-message routes to the modal.
        if sub == "commit" && !has_commit_message_flag(tail) {
            self.prompt = None;
            self.open_commit_editor();
            return;
        }
        if sub == "rebase" && tail.iter().any(|a| a == "-i" || a == "--interactive") {
            self.error = Some("interactive rebase is not yet supported".into());
            return;
        }
        if sub == "add" && tail.iter().any(|a| a == "-p" || a == "--patch") {
            self.error = Some("interactive `add -p` is not yet supported".into());
            return;
        }

        // View-defining commands switch tabs. Per v1 design, the canonical
        // query runs — the user's args are visible only in the prompt history
        // (↑). Arg-honoring is a separate feature.
        if sub == "log" {
            self.switch_view(View::Log);
            return;
        }
        if sub == "status" {
            self.switch_view(View::Status);
            return;
        }

        // Default: run the literal command.
        let mut cmd = git::GitCmd::new(sub);
        for a in tail {
            cmd = cmd.arg(a.as_str());
        }
        self.run_action(cmd);
    }

    // --- commit editor ---------------------------------------------------

    fn handle_commit_editor_key(&mut self, k: KeyEvent) {
        // Ctrl-C is handled in `handle_key`; here `Esc` is the vim-style
        // "back to normal mode" / "cancel command" key.
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
                "nothing staged to commit (press 's' on an unstaged file, [Esc] to dismiss)".into(),
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

    /// `X` in Status view — queues a destructive op for `y/N` confirmation.
    /// The behavior depends on which pane is focused and the file's status:
    ///
    /// | Pane | File status | Command |
    /// |---|---|---|
    /// | Unstaged | Untracked          | `git clean -fd -- <path>` (deletes the file from disk) |
    /// | Unstaged | Modified / Deleted | `git restore -- <path>` (drops worktree edits) |
    /// | Staged   | (any)              | `git restore --staged --worktree --source=HEAD -- <path>` (full reset for the file) |
    fn discard_selected(&mut self) {
        let Some(entry) = self.selected_entry() else {
            return;
        };
        let path = entry.path.clone();

        let (prompt, cmd) = match self.focused {
            Pane::Unstaged => {
                if matches!(entry.worktree, FileStatus::Untracked) {
                    (
                        format!("Delete untracked file '{path}'?"),
                        git::GitCmd::new("clean").arg("-fd").arg("--").arg(&path),
                    )
                } else {
                    (
                        format!("Discard worktree changes to '{path}'?"),
                        git::GitCmd::new("restore").arg("--").arg(&path),
                    )
                }
            }
            Pane::Staged => (
                format!("Reset '{path}' to HEAD (drops staged + worktree changes)?"),
                git::GitCmd::new("restore")
                    .arg("--staged")
                    .arg("--worktree")
                    .arg("--source=HEAD")
                    .arg("--")
                    .arg(&path),
            ),
        };
        self.confirm = Some(PendingConfirm { prompt, cmd });
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

    // --- log-view actions -----------------------------------------------

    fn move_log_selection(&mut self, delta: i32) {
        let len = self.log.len();
        if len == 0 {
            self.log_selected = 0;
            self.log_detail = String::new();
            return;
        }
        let new = match delta {
            i32::MIN => 0,
            i32::MAX => len - 1,
            d => (self.log_selected as i32 + d).clamp(0, (len - 1) as i32) as usize,
        };
        if new != self.log_selected {
            self.log_selected = new;
        }
        self.refresh_log_detail();
    }

    pub fn refresh_log(&mut self) {
        let cmd = git::log::cmd();
        self.history.record(&cmd.display());
        match git::log::load() {
            Ok(l) => {
                self.log = l;
                if self.log.is_empty() {
                    self.log_selected = 0;
                } else if self.log_selected >= self.log.len() {
                    self.log_selected = self.log.len() - 1;
                }
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        self.refresh_log_detail();
    }

    pub fn selected_commit(&self) -> Option<&git::LogEntry> {
        self.log.entries.get(self.log_selected)
    }

    pub fn refresh_log_detail(&mut self) {
        let Some(sha) = self.selected_commit().map(|c| c.sha.clone()) else {
            self.log_detail = String::new();
            return;
        };
        let cmd = git::log::show_stat_cmd(&sha);
        self.history.record(&cmd.display());
        match git::runner::run(&cmd) {
            Ok(out) if out.success() => {
                self.log_detail = out.stdout_str().into_owned();
            }
            Ok(out) => {
                self.log_detail = format!("(git show exited {})\n{}", out.status, out.stderr_str());
            }
            Err(e) => self.log_detail = format!("(error: {})", e),
        }
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
            git::GitCmd::new("diff")
                .arg("--cached")
                .arg("--")
                .arg(&path)
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

/// Whether `git commit` args include a flag that supplies the message inline.
/// Anything without one would normally drop git into `$EDITOR`, which would
/// fight our raw-mode terminal — so we route those to the modal editor.
fn has_commit_message_flag(args: &[String]) -> bool {
    args.iter().any(|a| {
        a == "-m"
            || a == "-F"
            || a == "--message"
            || a == "--file"
            || a.starts_with("--message=")
            || a.starts_with("--file=")
            // Short clustered forms like `-mfoo` or `-Ffoo`.
            || (a.len() > 2 && (a.starts_with("-m") || a.starts_with("-F")))
    })
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
