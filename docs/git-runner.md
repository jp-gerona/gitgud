# `git::runner`

Source: [`src/git/runner.rs`](../src/git/runner.rs)

## Purpose

Spawns a [`GitCmd`](git-cmd.md) as a subprocess and captures its output. Two entry points: a vanilla `run` and a `run_with_stdin` that pipes bytes into the child's stdin.

## API

```rust
pub struct CommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: std::process::ExitStatus,
}

impl CommandOutput {
    pub fn success(&self) -> bool;
    pub fn stdout_str(&self) -> Cow<'_, str>;
    pub fn stderr_str(&self) -> Cow<'_, str>;
}

pub fn run(cmd: &GitCmd) -> Result<CommandOutput>;
pub fn run_with_stdin(cmd: &GitCmd, stdin_bytes: &[u8]) -> Result<CommandOutput>;
```

Both functions return `anyhow::Result<CommandOutput>`. **Non-zero exit is not an error** — `run` returns `Ok(output)` and lets the caller inspect `status.success()`. This matters for diff commands: `git diff --no-index /dev/null new.txt` exits with 1 when files differ, but the diff text in stdout is what we want.

## `run`

Standard "spawn, wait for output." The child inherits no stdio — everything is captured. Used by every read-only and staging operation (`status`, `diff`, `add`, `restore`).

## `run_with_stdin`

Used for `git commit -F -` (read message from stdin). The function:

1. Configures the child with `Stdio::piped()` for stdin/stdout/stderr.
2. `spawn()`s the child.
3. Writes `stdin_bytes` to the child's stdin and lets the writer drop (closes the pipe, signalling EOF to git).
4. `wait_with_output()` collects the result.

This avoids the alternatives:

- A temp file in `/tmp` — leaves cleanup state, more code paths to fail.
- `git commit -m <message>` — awkward for multi-line messages and needs careful arg escaping.
- Re-running with `git commit --file=$(mktemp)` — same temp-file problems.

## Why not `git2`?

A deliberate choice — see [architecture.md](architecture.md). Briefly: shelling out matches what a user would type, the teaching bar can show real commands, and there's no library-versus-CLI behavioral drift to track.

## Error model

`anyhow::Result` carries context via `with_context` at each failure point (spawn, write-to-stdin, wait). A typical error looks like:

```
failed to spawn: git commit -F -
caused by: No such file or directory (os error 2)
```

The app surfaces these in `app.error` (status view) or `commit_editor.status_message` (commit editor).

## Related

- [`git::GitCmd`](git-cmd.md) — the builder fed into `run`
- [`git::status`](git-status.md) — calls `run` to fetch the porcelain v1 output
