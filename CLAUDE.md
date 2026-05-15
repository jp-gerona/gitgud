# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What gitgud is

A lightweight TUI for git, written in Rust. Inspired by lazygit and the Claude Code TUI/TUX experience. Two audiences:

- **Beginners**: surface the actual `git ...` commands being run so users learn them, not hide them.
- **Intermediate users** (incl. the author): smoother staging/unstaging, rebase, and git's vim-driven editors (commit message, interactive rebase todo).

## Architectural decisions

- **TUI**: Ratatui + Crossterm.
- **Git interface**: shell out to the `git` CLI (not libgit2). Every git call goes through `git::GitCmd`, which is the choke point. This gives us (a) behaviorally identical results to what a user would type, (b) the exact command string for the teaching UI, (c) a free upgrade path for new git features.
- **Async**: synchronous for now. Git commands typically return in <100ms. Add a worker thread + `mpsc` to `event` only when something concrete stutters.

## Module layout

Current (✓ implemented) and planned (○ not yet) modules. Don't pre-create planned modules — add them when an actual feature needs them.

```
src/
├── main.rs              ✓ terminal setup/teardown, run app
├── app.rs               ✓ App state, event dispatch, refresh logic
├── event.rs             ✓ crossterm event polling → AppEvent
├── history.rs           ✓ ring buffer of executed git commands
├── action.rs            ✓ Action enum (Stage, Unstage, Commit, ...)
├── keymap.rs            ✓ (view, key) → Action table
├── commit_editor.rs     ✓ in-TUI multiline editor state for commit messages
├── prompt.rs            ✓ slash-Command prompt state (buffer, cursor, history) + shell_split
├── editor.rs            ○ suspend TUI to run commands that take over the terminal ($EDITOR) — for future rebase
├── git/
│   ├── mod.rs           ✓ GitCmd builder
│   ├── runner.rs        ✓ spawn + capture stdout/stderr/status
│   ├── status.rs        ✓ parse `git status --porcelain=v1 -z`
│   ├── log.rs           ✓ parse `git log --pretty=format:...` + `git show --stat`
│   ├── diff.rs          ✓ hunk-level parsing + one-hunk patch reconstruction
│   ├── branch.rs        ○ `git branch -vv`, current, upstream
│   └── rebase.rs        ○ interactive rebase via GIT_SEQUENCE_EDITOR
└── ui/
    ├── mod.rs           ✓ top-level draw, tab bar inclusion, status line
    ├── theme.rs         ✓ colors & symbols
    ├── command_bar.rs   ✓ shows last/current `git ...` (the teaching surface)
    ├── prompt_bar.rs    ✓ one-row slash prompt + terminal cursor
    ├── tab_bar.rs       ✓ top-row tab strip with live counts
    ├── help.rs          ○ contextual keybind overlay
    └── views/
        ├── mod.rs       ✓
        ├── status.rs    ✓ staged/unstaged/diff
        ├── log.rs       ✓ commit list + `git show --stat` detail
        ├── commit.rs    ✓ modal commit editor
        ├── branches.rs  ○
        └── stash.rs     ○
```

## How to add a new git operation

1. Add a constructor on `git::GitCmd` or a free function in the relevant `git::*` module that returns a `GitCmd`. **Don't bypass `GitCmd`** — the command bar and history depend on every git call flowing through it.
2. (When `action.rs` exists) add an `Action` variant; map keys in `keymap.rs`.
3. In `app.rs` dispatch: render the command in the command bar **before** executing destructive ops (so users see what's about to happen), run via `git::runner::run`, refresh state, surface errors via `App::error`.

## Commands

- Build: `cargo build` (release: `cargo build --release`)
- Run: `cargo run`
- Test: `cargo test` — single test: `cargo test <test_name>` — single file's tests: `cargo test --test <file_stem>`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt` (check only: `cargo fmt -- --check`)
