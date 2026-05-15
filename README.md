# gitgud

A lightweight terminal UI for git, written in Rust.

gitgud helps you *do* git in the terminal while *learning* it — every action surfaces the underlying `git ...` command in a teaching bar, and the commit editor is a modal vi-style buffer so you can practice both at once.

<p align="center">
<img src="https://img.shields.io/badge/Built_with-Rust-000?logo=rust&amp;logoColor=fff&amp;labelColor=201a16&amp;color=dea584" alt="Built with Rust badge">
<img src="https://img.shields.io/badge/Built_with-Ratatui-000?logo=ratatui&amp;logoColor=fff&amp;labelColor=201a16&amp;color=dea584" alt="Built with Ratatui badge">
</p>

## Features

- **Tabbed views** — `1` Status, `2` Log (more to come). `[` / `]` cycle. Live counts in the tab labels.
- **Status view** — staged / unstaged / untracked panes with a live diff preview.
- **Log view** — last 200 commits with author + relative time + ref chips (HEAD / branches / tags). Right pane shows `git show --stat` for the selected commit.
- **One-key stage / unstage / discard** — `s` / `u` / `X` show the exact `git add`, `git restore`, `git clean` commands they run. Discard has a `[y/N]` safety prompt.
- **Hunk-level staging** — `Tab` into the Diff pane, `j` / `k` between hunks, then `s` / `u` / `X` to stage / unstage / discard just that hunk via `git apply [--cached] [--reverse] -`.
- **Slash Command mode** — press `/` to drop into a prompt and type real git commands (`/git status`, `/git add foo`, `/git commit -m "wip"`), with ↑/↓ history recall. `/git log` and `/git status` auto-switch to the matching tab. `/exit` and `/quit` close gitgud.
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
cargo test                                  # 51 unit tests
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

## Keybindings

### Tab navigation (any tabbed view)

| Key | Action |
|---|---|
| `1` | switch to Status |
| `2` | switch to Log |
| `[` / `]` | cycle previous / next tab |

### Status view

| Key | Action |
|---|---|
| `Tab` | cycle focus: Unstaged → its Diff → Staged → its Diff → … |
| `j` `k` (or arrows) | move selection (file pane) / move between hunks (Diff pane) |
| `s` | stage selected file / stage selected hunk (in Diff pane) |
| `u` | unstage selected file / unstage selected hunk (in Diff pane) |
| `X` | discard / reset selected file or hunk (with `[y/N]` confirmation) |
| `c` | open commit editor |
| `/` | enter slash-Command mode |
| `r` | refresh status |
| `Esc` | step out of the Diff pane, or dismiss error |
| `q` / `Ctrl+C` | quit |

In the **Diff pane** (border turns cyan, title shows `hunk h/n`, a `▌` gutter marks the active hunk): `s` stages just that hunk, `u` unstages it, `X` discards it. `s` works from the Unstaged side, `u` from the Staged side.

### Log view

| Key | Action |
|---|---|
| `j` `k` (or arrows) | move selection |
| `g` / `G` | jump to first / last commit |
| `/` | enter slash-Command mode |
| `r` | refresh log |
| `Esc` | dismiss error |
| `q` / `Ctrl+C` | quit |

### Command mode (slash prompt)

| Key | Action |
|---|---|
| `Enter` | run the typed command (must start with `git`, or `/exit`/`/quit`) |
| `Esc` | leave Command mode |
| `↑` / `↓` | recall previous / next submitted command |
| `←` `→` `Home` `End` | move cursor inside the buffer |
| `Backspace` / `Delete` | edit the buffer |
| `Ctrl+C` | quit gitgud |

`/git commit` with no `-m`/`-F` routes to the modal editor. `/git log` and `/git status` switch to the matching tab. `/exit` and `/quit` close gitgud. `/git rebase -i` and `/git add -p` are rejected until those views ship.

### Commit editor

| Mode | Keys |
|---|---|
| Normal | `h j k l` · `0 $` · `w b` · `gg G` · `i a I A o O` · `x dd dw D` · `:` |
| Insert | type / arrows / Backspace / Delete · `Esc` → Normal |
| Command | `:w` `:wq` `:x` commit · `:q` cancel-if-blank · `:q!` force cancel |

## Documentation

Module-by-module deep dives live in [`docs/`](docs/):

- [Architecture](docs/architecture.md)
- [GitCmd builder](docs/git-cmd.md)
- [Process runner](docs/git-runner.md)
- [Status parser](docs/git-status.md)
- [Log parser](docs/git-log.md)
- [Diff parser & hunk staging](docs/git-diff.md)
- [Commit editor](docs/commit-editor.md)
- [Slash prompt](docs/prompt.md)
- [Log view](docs/log-view.md)
- [App state & dispatch](docs/app.md)
- [UI layer](docs/ui.md)
- [Keymap & actions](docs/keymap-action.md)
- [Event loop](docs/event.md)
- [Command history](docs/history.md)

## License

MIT — see [LICENSE](LICENSE).
