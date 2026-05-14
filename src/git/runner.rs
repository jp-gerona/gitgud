use super::GitCmd;
use anyhow::{Context, Result};
use std::borrow::Cow;
use std::io::Write;
use std::process::{Command, ExitStatus, Stdio};

pub struct CommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: ExitStatus,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.status.success()
    }

    pub fn stdout_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    pub fn stderr_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.stderr)
    }
}

pub fn run(cmd: &GitCmd) -> Result<CommandOutput> {
    let mut c = Command::new("git");
    c.args(cmd.args_ref());
    if let Some(cwd) = cmd.cwd_ref() {
        c.current_dir(cwd);
    }
    let out = c
        .output()
        .with_context(|| format!("failed to spawn: {}", cmd.display()))?;
    Ok(CommandOutput {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status,
    })
}

/// Run a git command, piping `stdin_bytes` to its stdin (used to feed a commit
/// message via `git commit -F -` without a temp file).
pub fn run_with_stdin(cmd: &GitCmd, stdin_bytes: &[u8]) -> Result<CommandOutput> {
    let mut c = Command::new("git");
    c.args(cmd.args_ref());
    if let Some(cwd) = cmd.cwd_ref() {
        c.current_dir(cwd);
    }
    c.stdin(Stdio::piped());
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());

    let mut child = c
        .spawn()
        .with_context(|| format!("failed to spawn: {}", cmd.display()))?;
    if let Some(mut sin) = child.stdin.take() {
        sin.write_all(stdin_bytes)
            .with_context(|| format!("failed to write stdin to: {}", cmd.display()))?;
    }
    let out = child
        .wait_with_output()
        .with_context(|| format!("wait failed: {}", cmd.display()))?;
    Ok(CommandOutput {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status,
    })
}
