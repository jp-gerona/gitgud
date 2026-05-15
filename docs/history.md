# `history`

Source: [`src/history.rs`](../src/history.rs)

## Purpose

A bounded ring buffer of executed `git` command strings. Two consumers today:

1. **Command bar** — `ui::command_bar` renders `history.last()`, so the most-recent git invocation is always visible at the bottom of the screen. This is the teaching surface.
2. **Future "command log" view** — keeping the full ring around makes a "what just ran?" panel a one-screen feature.

## API

```rust
pub struct History { /* VecDeque<String>, cap 64 */ }

impl History {
    pub fn record(&mut self, cmd: &str);
    pub fn last(&self) -> Option<&str>;
}
```

`record` pushes onto the back; if the deque is at capacity, the front is dropped (oldest discarded).

## Capacity

64 entries. A typical session generates: 1 status + 1 diff per navigation step + occasional stage/unstage/commit. 64 covers a few minutes of active use; older entries can be dropped without harming UX. If a full command-log view ships, we may bump the cap or persist to disk.

## What gets recorded

Every git invocation logs its `GitCmd::display()` string. This includes:

- `git status --porcelain=v1 -z` (every `refresh_status`)
- `git diff [--cached] -- <path>` (every selection change in Status view)
- `git add -- <path>`, `git restore --staged -- <path>` (stage / unstage)
- `git commit -F -` (commit)

The order matters: in `App::run_action` and `submit_commit`, the user-initiated command is recorded **after** the implicit `refresh_status`, so the bar reflects the user's intent rather than the reload that followed.

## What's intentionally *not* recorded

The piped stdin to `git commit -F -` is not stored — only the command line. The actual message lives in git's own log after a successful commit; capturing it here would duplicate that and risk leaking sensitive content into in-memory state longer than necessary.

## Related

- [`git::GitCmd::display`](git-cmd.md) — produces the strings stored here
- [`ui::command_bar`](ui.md) — only consumer today
