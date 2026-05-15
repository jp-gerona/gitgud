# gitgud

A lightweight terminal UI for git, written in Rust.

gitgud helps you *do* git in the terminal while *learning* it — every action surfaces the underlying `git ...` command in a teaching bar, and the commit editor is a modal vi-style buffer so you can practice both at once.

## Features

- **Status view** — staged / unstaged / untracked panes with a live diff preview.
- **One-key stage / unstage** — `s` / `u` show the exact `git add --` and `git restore --staged --` they run.
- **Teaching command bar** — every executed `git ...` rendered copy-pasteable at the bottom.
- **Modal commit editor** — vi-style Normal / Insert / Command modes (`i a I A o O`, `h j k l w b 0 $ gg G`, `x dd dw D`, `:w :wq :x :q :q!`) with a mode-aware hints panel.
- **Live subject/body warnings** — subject line goes yellow at 50 chars and red at 72.

## Stack

- [Ratatui](https://github.com/ratatui-org/ratatui) 0.29 + [Crossterm](https://github.com/crossterm-rs/crossterm) 0.28 — TUI / terminal I/O
- [anyhow](https://github.com/dtolnay/anyhow) — error propagation
- Shells out to the system `git` binary (no libgit2)

## Install

Requires Rust 1.85+ (edition 2024) and a `git` on `$PATH`.

```sh
git clone https://github.com/jp-gerona/gitgud.git
cd gitgud
cargo install --path .
```

Or run from source: `cargo run` inside any git repository.

## Tests

```sh
cargo test                                  # 29 unit tests
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

## Keybindings

### Status view

| Key | Action |
|---|---|
| `Tab` | switch pane |
| `j` `k` (or arrows) | move selection |
| `s` | stage selected file |
| `u` | unstage selected file |
| `c` | open commit editor |
| `r` | refresh status |
| `Esc` | dismiss error |
| `q` / `Ctrl+C` | quit |

### Commit editor

| Mode | Keys |
|---|---|
| Normal | `h j k l` · `0 $` · `w b` · `gg G` · `i a I A o O` · `x dd dw D` · `:` |
| Insert | type / arrows / Backspace / Delete · `Esc` → Normal |
| Command | `:w` `:wq` `:x` commit · `:q` cancel-if-blank · `:q!` force cancel |

## Documentation

Module-by-module deep dives live in [`docs/`](docs/):

- [Architecture](docs/architecture.md) — module map, data flow, design choices
- [GitCmd builder](docs/git-cmd.md)
- [Process runner](docs/git-runner.md)
- [Status parser](docs/git-status.md)
- [Commit editor](docs/commit-editor.md)
- [App state & dispatch](docs/app.md)
- [UI layer](docs/ui.md)
- [Keymap & actions](docs/keymap-action.md)
- [Event loop](docs/event.md)
- [Command history](docs/history.md)

## License

MIT — see [LICENSE](LICENSE).
