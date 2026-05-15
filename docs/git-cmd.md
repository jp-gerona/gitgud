# `git::GitCmd`

Source: [`src/git/mod.rs`](../src/git/mod.rs)

## Purpose

A builder for a `git ...` subprocess invocation. It is the **single chokepoint** through which every git call in gitgud flows — both for actually running the command (via `git::runner`) and for displaying it in the teaching bar.

## API

```rust
pub struct GitCmd { /* args, optional cwd */ }

impl GitCmd {
    pub fn new<S: Into<OsString>>(subcommand: S) -> Self;
    pub fn arg<S: Into<OsString>>(self, arg: S) -> Self;
    pub fn cwd<P: Into<PathBuf>>(self, cwd: P) -> Self;

    pub fn args_ref(&self) -> &[OsString];
    pub fn cwd_ref(&self) -> Option<&PathBuf>;

    /// Copy-pasteable shell-form: `git <sub> <args...>`, with POSIX-style
    /// quoting for args containing whitespace or shell metacharacters.
    pub fn display(&self) -> String;
}
```

The builder is `Clone + Debug`. It does not take an executable path — the binary name is always `"git"` (resolved via `$PATH` by `std::process::Command`).

## Display quoting

`display()` is used in two places: the command bar (visible teaching surface) and the `History` log. It quotes any arg that:

- is empty, or
- contains whitespace, `'`, `"`, `$`, backtick, or backslash

…with single quotes, escaping embedded single quotes the POSIX way (`'\''`). Paths with spaces therefore look like `'my file.txt'` in the bar — exactly what a user could copy and run.

## Why a builder and not a free function

Two reasons:

1. **Display before run.** Several code paths want the `.display()` string both *before* (for destructive commands, future feature) and *after* (for the bar). Holding the args in a value makes this trivial.
2. **One place to add cross-cutting behavior.** When we eventually want to record every git invocation, add env vars (`GIT_PAGER=cat`, locale), or pin to a specific git binary, all of that lives here.

## Usage examples

```rust
// Read-only
let cmd = GitCmd::new("status").arg("--porcelain=v1").arg("-z");

// Mutation
let cmd = GitCmd::new("add").arg("--").arg(path);

// Reading from stdin
let cmd = GitCmd::new("commit").arg("-F").arg("-");
git::runner::run_with_stdin(&cmd, message.as_bytes())?;
```

## Related

- [`git::runner`](git-runner.md) — actually spawns the process
- [`history`](history.md) — stores `.display()` strings for the bar
