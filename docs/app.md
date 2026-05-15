# `app`

Source: [`src/app.rs`](../src/app.rs)

## Purpose

Owns all mutable state, runs the main loop, and dispatches input to the correct view-specific handler. Every git side effect is launched from here.

## Structs and enums

```rust
pub enum Pane    { Unstaged, Staged }
pub enum View    { Status, CommitEditor }

pub struct App {
    pub status: git::StatusList,         // last loaded `git status` snapshot
    pub focused: Pane,                   // which list pane has the cursor (Status view)
    pub unstaged_selected: usize,
    pub staged_selected: usize,
    pub diff: String,                    // diff for the currently-selected file
    pub history: History,                // ring buffer feeding the command bar
    pub should_quit: bool,
    pub error: Option<String>,           // status-view errors only
    pub view: View,
    pub commit_editor: CommitEditor,
}
```

`App::new()` constructs the default state and immediately calls `refresh_status()` so the first frame has populated panes.

## The run loop

```rust
pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()>;
```

Per tick:

1. `terminal.draw(|f| ui::draw(f, self))` — render the current view.
2. `event::poll(200 ms)` — wait for a key/resize or time out.
3. If something arrived, `handle_event` → `handle_key`.
4. Loop until `should_quit`.

A 200 ms poll keeps the UI responsive without busy-waiting; nothing time-based depends on the tick rate.

## Input dispatch

```
handle_key(k)
├── view == Status         → handle_status_key
│                             └── keymap::key_to_action → match Action
└── view == CommitEditor   → handle_commit_editor_key
                              ├── Ctrl-C → quit gitgud
                              └── match commit_editor.mode
                                  ├── Normal  → handle_normal_mode_key
                                  ├── Insert  → handle_insert_mode_key
                                  └── Command → handle_command_mode_key
```

The Status view uses the [keymap/action](keymap-action.md) indirection. The commit editor handlers are inline because the modal context (and `pending_op`) make a generic Action-based table awkward.

## Status-view actions

| Action | Behavior |
|---|---|
| `Quit` | `should_quit = true` |
| `MoveSelection(±1)` | shift the focused pane's cursor; clamp; trigger `refresh_diff` |
| `SwitchPane` | toggle `focused`; `refresh_diff` |
| `Refresh` | `refresh_status` |
| `StageSelected` | (Unstaged pane only) `git add -- <path>` via `run_action` |
| `UnstageSelected` | (Staged pane only) `git restore --staged -- <path>` via `run_action` |
| `Commit` | `open_commit_editor` |
| `Dismiss` | clear `error` |

`run_action(cmd)` is the shared mutator: runs `cmd`, sets `error` on failure / clears on success, `refresh_status()`, and records the displayed command into history last (so the command bar reflects the user's action, not the implicit reload).

## Commit editor lifecycle

```
open_commit_editor()
  ├── nothing staged → set app.error, stay in Status view
  └── else            → clear app.error, reset commit_editor, view = CommitEditor

submit_commit()                        (from `:w/:wq/:x` Enter)
  ├── is_blank → status_message "Aborting commit due to empty commit message"
  └── else
      ├── run git commit -F - (piping the message)
      ├── success → reset editor, view = Status, clear error
      └── failure → status_message = stderr / exit code

cancel_commit()                        (from `:q` on blank / `:q!`)
  └── reset editor, view = Status, clear error
```

Errors during commit attempts land in `commit_editor.status_message`, **not** `app.error` — the user stays in the editor with their message intact.

## Why `Terminal` is on `run` but not deeper

`App::run` takes the terminal because `terminal.draw` needs it. Once that closure returns, the terminal is not threaded any further; handlers operate on `&mut self` only. The earlier $EDITOR-suspend prototype required threading the terminal into `commit()`; the modal in-TUI editor doesn't, which keeps the signatures clean.

If interactive rebase eventually lands, the suspend path will need to thread `terminal` into one specific handler — but only that handler.

## Related

- [`event`](event.md) — source of input
- [`keymap` / `action`](keymap-action.md) — Status-view dispatch table
- [`commit_editor`](commit-editor.md) — state owned by `App`
- [`ui`](ui.md) — pure render of `App` per frame
