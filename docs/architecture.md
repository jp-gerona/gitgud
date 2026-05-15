# Architecture

gitgud is a small Rust TUI built on [Ratatui](https://github.com/ratatui-org/ratatui) and [Crossterm](https://github.com/crossterm-rs/crossterm). It does **not** link `libgit2`; every git operation is a subprocess call to the system `git` binary. This is the central design choice — it means:

1. gitgud's behavior matches what a user would see at the shell.
2. Every command can be surfaced verbatim in the teaching bar.
3. New git features are available the moment the user's `git` supports them.

## Module map

```
src/
├── main.rs           — terminal setup, run loop, restore on exit
├── app.rs            — App state, view dispatch, action handlers
├── event.rs          — Crossterm event polling → AppEvent
├── action.rs         — Action enum (typed user intents)
├── keymap.rs         — key → Action mapping (Status view only)
├── history.rs        — bounded ring buffer of executed git commands
├── commit_editor.rs  — modal vi-style editor state
├── prompt.rs         — slash-Command prompt state + shell tokenizer
├── git/
│   ├── mod.rs        — GitCmd builder (the choke point for git calls)
│   ├── runner.rs     — process spawning: run / run_with_stdin
│   ├── status.rs     — `git status --porcelain=v1 -z` parser
│   └── log.rs        — `git log --pretty=format:...` parser + `git show --stat` builder
└── ui/
    ├── mod.rs        — top-level draw + bottom hint/error line
    ├── theme.rs      — color constants
    ├── command_bar.rs— renders the most recent `git ...` from history
    ├── prompt_bar.rs — slash-Command prompt strip + terminal cursor
    ├── tab_bar.rs    — top-row tab strip with live counts and numbered hints
    └── views/
        ├── status.rs — staged / unstaged / diff panes
        ├── log.rs    — commit list + `git show --stat` detail (two-pane)
        └── commit.rs — modal vi editor + status row + hints
```

## Data flow

```
KeyEvent
  → event::poll
    → App::handle_key
        ├── Ctrl+C anywhere    → quit
        ├── View::CommitEditor → match commit_editor.mode
        │                         ├── Normal  → handle_normal_mode_key
        │                         ├── Insert  → handle_insert_mode_key
        │                         └── Command → handle_command_mode_key
        └── tabbed view (Status | Log)
              ├── prompt.is_some()  → handle_prompt_key → (Enter) dispatch_prompt
              │                                            → build GitCmd → run_action
              │                                              or switch_view (log/status)
              │                                              or quit (/exit, /quit)
              ├── key == '/'        → open prompt
              ├── tab key (1/2/[/]) → switch_view
              └── per-view dispatch → handle_status_normal_key
                                      handle_log_normal_key
  → handler builds GitCmd → git::runner::run → updates App state
  → ui::draw repaints the active view + command bar + status line
```

## The "every git call goes through GitCmd" invariant

`git::GitCmd` is a small builder that holds args and an optional `cwd`. Its `.display()` method produces a copy-pasteable shell-style command string with the right quoting. Both the teaching command bar (`ui::command_bar`) and the history log read from this string.

The contract: **no code spawns `git` directly**; it always constructs a `GitCmd` and runs it through `git::runner::run` or `run_with_stdin`. This is what makes the teaching surface trustworthy.

When adding a new git operation, follow the pattern:

1. Build a `GitCmd` (in the relevant `git::*` module or inline).
2. Run via `git::runner::run` (or `run_with_stdin` for piped input).
3. Surface the command via the history (or directly via `app.error` / `commit_editor.status_message` on failure).

## State separation: `app.error` vs `commit_editor.status_message`

| Field | Scope | Cleared by |
|---|---|---|
| `app.error` | status-view problems (stage/unstage/refresh fail) | next successful action, `Esc` (Status view), `Action::Dismiss` |
| `commit_editor.status_message` | editor problems (vim `E32`/`E37`/`E492`, failed commit) | entering insert or command mode, `Esc` in normal mode |

This split is deliberate. A failed `git commit` does **not** drop the user back to the status view discarding their typed message — it surfaces inside the editor so they can fix and retry. The two fields never share a frame; the bottom line is `app.error`-only and lives in the Status view.

## Synchronous, single-threaded

Every git invocation blocks the UI thread for the duration of the subprocess. Most calls return in well under 100 ms (status, diff, add, restore), so this is invisible. If a slow command (large `git log`, `git fetch`) starts to stutter the UI, the right move is to add a worker thread + `mpsc` channel into `event` rather than adopting Tokio.

## What's not done yet

- Hunk-level staging — needs `git diff` parsing and `git apply --cached`
- Honoring slash-command args on view-defining commands (`--oneline`, `-n`, `--graph`, `--author=`) — tracked as a follow-up issue
- Branch view
- Stash view
- Interactive rebase — would resurrect a small `editor.rs` for `$EDITOR` suspend
- Push / pull / fetch
