pub mod log;
pub mod runner;
pub mod status;

use std::ffi::OsString;
use std::path::PathBuf;

pub use log::{LogEntry, LogList};
pub use status::{FileEntry, FileStatus, StatusList};

/// Builder for a `git ...` invocation.
///
/// Every git call in gitgud goes through this so the exact command string is
/// always available for the teaching UI / command bar and the history log.
#[derive(Clone, Debug)]
pub struct GitCmd {
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
}

impl GitCmd {
    pub fn new<S: Into<OsString>>(subcommand: S) -> Self {
        Self {
            args: vec![subcommand.into()],
            cwd: None,
        }
    }

    pub fn arg<S: Into<OsString>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }

    #[allow(dead_code)] // used once we run git from a non-cwd repo (e.g. submodules)
    pub fn cwd<P: Into<PathBuf>>(mut self, cwd: P) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn args_ref(&self) -> &[OsString] {
        &self.args
    }

    pub fn cwd_ref(&self) -> Option<&PathBuf> {
        self.cwd.as_ref()
    }

    /// Human-readable, copy-pasteable command line for the teaching UI.
    pub fn display(&self) -> String {
        let mut out = String::from("git");
        for a in &self.args {
            out.push(' ');
            let s = a.to_string_lossy();
            if needs_quoting(&s) {
                out.push('\'');
                out.push_str(&s.replace('\'', "'\\''"));
                out.push('\'');
            } else {
                out.push_str(&s);
            }
        }
        out
    }
}

fn needs_quoting(s: &str) -> bool {
    s.is_empty()
        || s.chars()
            .any(|c| c.is_whitespace() || matches!(c, '\'' | '"' | '$' | '`' | '\\'))
}
