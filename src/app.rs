use crate::event::{self, AppEvent};
use crate::git::{self, FileStatus};
use crate::history::History;
use crate::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::Backend};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Unstaged,
    Staged,
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
        };
        app.refresh_status();
        Ok(app)
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
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
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        match k.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Tab => {
                self.focused = match self.focused {
                    Pane::Unstaged => Pane::Staged,
                    Pane::Staged => Pane::Unstaged,
                };
                self.refresh_diff();
            }
            KeyCode::Char('r') => self.refresh_status(),
            _ => {}
        }
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
                // `git diff --no-index` exits 1 on differences; treat stdout as
                // authoritative regardless of exit code.
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
