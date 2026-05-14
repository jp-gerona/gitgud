use super::GitCmd;
use anyhow::{Context, Result};
use std::borrow::Cow;
use std::process::{Command, ExitStatus};

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
