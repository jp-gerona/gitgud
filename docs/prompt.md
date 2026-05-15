# `prompt`

Source: [`src/prompt.rs`](../src/prompt.rs)

## Purpose

State for the slash-Command prompt — gitgud's primary teaching surface. Pressing `/` in the Status view opens a one-line buffer at the bottom of the screen where the user types a literal `git ...` command, hits Enter, and watches it run. The keymap shortcuts (`s`, `u`, `c`, …) remain available; the prompt is the "type it yourself" alternative.

The module owns only the *state*: the buffer, cursor, and a bounded ring of past submissions. Parsing and dispatch live in [`app::dispatch_prompt`](app.md); rendering lives in [`ui::prompt_bar`](ui.md).

## Why a persistent mode, not vim's one-shot `:`

In vim, `:` opens a transient command-line for one command. We considered that model and rejected it: the goal here is to *practice typing git commands for a stretch*, not to fire one off. So `/` enters a persistent mode (closer to vim's Insert mode), Enter runs and stays in the mode for rapid-fire input, and `Esc` returns to Normal.

`:` is intentionally **reserved** — it's already used inside the modal commit editor for `:w`/`:wq`/`:q`, and we want to keep that surface available for future vim-flavored one-shots (e.g. `:q` from Status to quit gitgud).

## API

```rust
pub struct Prompt {
    pub buffer: String,        // chars typed after the leading "/"
    pub cursor: usize,         // char index into buffer (Unicode-safe)
    history: VecDeque<String>, // submitted commands, capped at 64
    history_idx: Option<usize>,
    saved: Option<String>,
}

impl Prompt {
    pub fn new() -> Self;
    pub fn submit(&mut self) -> String; // pops buffer, records into history, resets
    pub fn insert_char(&mut self, c: char);
    pub fn backspace(&mut self);
    pub fn delete_at_cursor(&mut self);
    pub fn move_left/right/home/end(&mut self);
    pub fn recall_prev(&mut self);      // ↑
    pub fn recall_next(&mut self);      // ↓
}

pub fn shell_split(s: &str) -> Vec<String>;
```

The leading `/` is **not** stored in `buffer` — it's a render-time prefix on the prompt bar. This keeps parsing trivial: whatever is in `buffer` is parsed as if typed at a shell.

## `submit`

```rust
pub fn submit(&mut self) -> String;
```

Returns the **raw** buffer (not trimmed), resets the prompt to empty state, and records the buffer into history when non-blank. The caller (`app::dispatch_prompt`) decides what to do with it — typically `trim()` and route. Consecutive duplicate submissions are collapsed so `↑` doesn't repeat the same entry.

History is capped at 64 entries; the oldest is dropped when full. Distinct from [`history::History`](history.md) (which logs *every* executed `GitCmd.display()` including auto-refreshes) — this ring stores only what the user typed.

## History recall semantics

Closely modeled on bash/zsh:

| State | `↑` (recall_prev) | `↓` (recall_next) |
|---|---|---|
| `idx == None`, history empty | no-op | no-op |
| `idx == None`, history non-empty | save current buffer to `saved`, jump to newest entry | no-op |
| `idx == Some(0)` (oldest) | stay at oldest | move to next-newer |
| `idx == Some(n)` mid-history | move to older (`n-1`) | move to newer (`n+1`) |
| `idx == Some(last)` (newest) | stay | restore `saved`, clear `idx` |

Editing (`insert_char`, `backspace`, `delete_at_cursor`) calls `detach_from_history`, which clears both `idx` and `saved`. So after recalling an entry and modifying it, `↑` from there treats the edit as a fresh baseline (saves it, jumps to newest) and `↓` is a no-op — the edit isn't lost to history scrubbing.

This is slightly friendlier than strict bash, which would clobber the edit on `↑`.

## `shell_split`

```rust
pub fn shell_split(s: &str) -> Vec<String>;
```

A small subset of POSIX shell tokenizing — enough for things like `git commit -m "wip change"`:

- Splits on unquoted whitespace.
- Double quotes group a run as one token; backslash inside quotes escapes the next char (so `\"` and `\\` work).
- Single quotes are **not** special — they're literal characters. (We could add them later; in practice `git commit -m "..."` is what users type.)
- Variables, globs, redirection, and pipes are not interpreted (this is gitgud, not a shell).

The dispatcher uses the first token to gate execution:

- empty → no-op (stay in Command mode)
- not `git` → `unknown command: /... (commands must start with `git`)` surfaced via `app.error`
- `git` + no further tokens → `missing git subcommand`
- `git <sub>` + args → built into a `GitCmd` and run via `app::run_action`

## Editor-takeover detection

Some git commands normally spawn `$EDITOR`. We're in raw mode on an alternate screen — letting them run would fight the terminal. Dispatch intercepts:

| Input | Behavior |
|---|---|
| `git commit` (no `-m`/`-F`/`--message`/`--file`/`-mMSG`/`-Fpath`/`--message=...`/`--file=...`) | open the modal commit editor (same as `c`) |
| `git rebase -i` / `--interactive` | reject — `interactive rebase is not yet supported` |
| `git add -p` / `--patch` | reject — `interactive add -p is not yet supported` |
| everything else | run via `git::runner::run`, refresh status |

The commit interception is the most useful — it makes `/git commit` a discoverable path to the same modal editor that `c` opens.

## Why the prompt is decoupled from rendering

`Prompt` is a plain struct. No `Frame`, no `Ratatui` type. This means:

- Unit tests cover the editing model thoroughly (15 cases) without spinning up a UI.
- The renderer (`ui::prompt_bar`) is a pure function of `&Prompt`.
- If we later swap the input UX (e.g. multi-line prompt, completion popup), the state contract is stable.

## Related

- [`app`](app.md) — owns the `Option<Prompt>`, dispatches submissions
- [`ui::prompt_bar`](ui.md) — renders `/` + buffer + terminal cursor
- [`git::GitCmd`](git-cmd.md) — every dispatched command becomes one of these
- [`commit_editor`](commit-editor.md) — receives `/git commit` (no `-m`) via the editor-takeover intercept
