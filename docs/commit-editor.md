# `commit_editor`

Source: [`src/commit_editor.rs`](../src/commit_editor.rs)

## Purpose

A small modal text editor for composing git commit messages, modeled on vi. It exists for two reasons:

1. **Make commits feel native.** No suspending the TUI, no `$EDITOR` handoff, no fiddly terminal-state dance.
2. **Practice vi.** gitgud's audience is partly users learning neovim. The commit editor is a small, low-stakes context to drill the modal model and the common motion/edit commands.

## State

```rust
pub enum EditorMode {
    Normal,
    Insert,
    Command(String),   // buffer holds chars typed after the leading ':'
}

pub enum SubmitIntent {
    None, Commit, Cancel,
}

pub struct CommitEditor {
    pub lines: Vec<String>,         // one entry per line, no '\n' inside
    pub row: usize,                 // cursor row
    pub col: usize,                 // cursor column in CHARS (Unicode-safe)
    pub mode: EditorMode,
    pub pending_op: Option<char>,   // first half of gg / dd / dw
    pub status_message: Option<String>,
}
```

The cursor is tracked as `(row, char-col)` rather than a byte offset. Mutations translate to byte indices via `byte_index(line, char_col)` only at the point of insertion/deletion — so multi-byte characters (Unicode names, emoji, etc.) Just Work.

## Modes & transitions

```
                Esc                Esc
   ┌─────────────────┐  ┌─────────────────┐
   │                 │  │                 │
   ▼     i a I A     │  ▼      Enter      │
NORMAL ─────────► INSERT     COMMAND ──► (intent → app)
   │     o O            │
   │                    │
   │   :                │
   ▼                    │
COMMAND ◄───────────────┘
   │  Esc → NORMAL
   │  Backspace on empty → NORMAL
```

Mode-change helpers:

| Method | Behavior |
|---|---|
| `enter_insert()` | mode = Insert, clear status |
| `enter_insert_after()` | advance col (if not at line end), enter insert |
| `enter_insert_line_start()` | col = 0, enter insert |
| `enter_insert_line_end()` | col = line_len, enter insert |
| `open_line_below()` | insert blank line at row+1, move to it, enter insert |
| `open_line_above()` | insert blank line at row, leave row pointer (pushed onto new line), enter insert |
| `enter_command()` | mode = Command(""), clear status |
| `cancel_command()` | mode = Normal (status preserved) |
| `enter_normal()` | mode = Normal, clear pending_op |

## Command parser (`execute_command`)

Always exits command mode. Returns:

| Input | Returns | Side effect |
|---|---|---|
| `""` | `None` | none |
| `"w"` `"wq"` `"x"` | `Commit` | none |
| `"q!"` | `Cancel` | none |
| `"q"` (blank buffer) | `Cancel` | none |
| `"q"` (non-blank) | `None` | sets `E37: No write since last change ...` |
| anything else | `None` | sets `E492: Not an editor command: <cmd>` |

The app layer interprets the intent: `Commit` → `submit_commit()`, `Cancel` → `cancel_commit()`, `None` → stay in editor.

## Normal-mode command set

| Command | Binding | Implementation |
|---|---|---|
| Left/down/up/right | `h j k l` | `move_left/down/up/right` |
| Line ends | `0` `$` | `move_line_start / _end` |
| Word forward / back | `w` `b` | `move_word_forward / _back` |
| Top / bottom | `gg` `G` | `goto_top / _bottom` |
| Enter insert | `i a I A` | `enter_insert*` family |
| Open line | `o` `O` | `open_line_below / _above` |
| Delete char | `x` | `delete_at_cursor` |
| Delete line | `dd` | `delete_line` |
| Delete word | `dw` | `delete_word_forward` |
| Delete to EOL | `D` | `delete_to_end_of_line` |
| Enter command | `:` | `enter_command` |

Two-key commands (`gg`, `dd`, `dw`) park the first key in `pending_op` and complete on the next key. Any non-matching second key abandons the operator (matches vim's behavior).

## Word boundaries

`w`/`b`/`dw` use a three-class character classification:

```rust
enum CharClass { Word, Punct, Space }

// Word    = alphanumeric or '_'
// Punct   = anything non-whitespace that isn't a Word char
// Space   = whitespace
```

`w` from inside a Word/Punct run advances past the run, then skips whitespace, landing on the next non-whitespace char. `b` is the mirror. Word boundaries thus respect punctuation runs (e.g. `foo::bar` is `foo`, `::`, `bar`).

This is a simplification of vim's full word-motion machinery (no `W`/`B` for WORD-on-whitespace) but enough to feel right for commit messages and to teach the concept.

## Status messages

`status_message: Option<String>` is the in-editor "command line" — it's where vim-style errors land (`E32` empty message, `E37` unsaved changes, `E492` unknown command, plus git-commit failure strings). Rendered on the vim status row, in yellow bold, overriding the mode label when set.

Cleared by:

- Entering insert mode (any variant)
- Entering command mode
- `Esc` in normal mode (`clear_status`)

Survives motions and the `:cancel_command` path — a vim user's instinct is "the error sticks until I do something about it."

## Why decoupled from rendering

`CommitEditor` knows nothing about Ratatui. The view in [`ui/views/commit.rs`](ui.md) is a pure render of this state; the app's key handlers in [`app.rs`](app.md) are the only mutators. This is why the editor has 24 unit tests — none of them need a terminal.

## Related

- [`app`](app.md) — owns the editor and routes keys to its methods per mode
- [`ui/views/commit`](ui.md) — renders the editor, status row, and hints panel
