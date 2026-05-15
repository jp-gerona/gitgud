# `app`

Source: [`src/app.rs`](../src/app.rs)

## Purpose

Owns all mutable state, runs the main loop, and dispatches input to the correct view-specific handler. Every git side effect is launched from here.

## Structs and enums

```rust
pub enum Pane { Unstaged, Staged }
pub enum View {
    Status,
    Log,
    CommitEditor,    // modal — bypasses tab bar and per-tab dispatch
}

pub struct App {
    pub status: git::StatusList,         // last loaded `git status` snapshot
    pub focused: Pane,                   // which list pane has the cursor (Status view)
    pub unstaged_selected: usize,
    pub staged_selected: usize,
    pub diff: String,                    // diff for the currently-selected file
    pub log: git::LogList,               // last loaded `git log` snapshot
    pub log_selected: usize,
    pub log_detail: String,              // `git show --stat <sha>` for the selected commit
    pub history: History,                // ring buffer feeding the command bar
    pub should_quit: bool,
    pub error: Option<String>,           // status/log error surface (not commit editor)
    pub view: View,
    pub commit_editor: CommitEditor,
    pub prompt: Option<Prompt>,          // Some while in slash-Command mode
}
```

`View::is_tabbed()` returns true for `Status | Log`. The UI uses this to decide whether to render the tab bar and the bottom hint row vs. the full-screen commit editor.

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
├── Ctrl+C anywhere                   → should_quit = true
├── view == CommitEditor              → handle_commit_editor_key
│                                        └── match commit_editor.mode
│                                            ├── Normal  → handle_normal_mode_key
│                                            ├── Insert  → handle_insert_mode_key
│                                            └── Command → handle_command_mode_key
└── tabbed view (Status | Log)
      ├── prompt.is_some()             → handle_prompt_key
      │                                  └── Enter → dispatch_prompt
      │                                              ├── /exit, /quit
      │                                              ├── /git log / status   → switch_view
      │                                              ├── /git commit no -m   → open_commit_editor
      │                                              └── /git ...            → run_action
      ├── key == '/'                   → open prompt
      ├── tab key (1, 2, [, ])         → switch_view
      └── per-view dispatch
            ├── View::Status → handle_status_normal_key → keymap → match Action
            └── View::Log    → handle_log_normal_key (inline)
```

The Status view uses the [keymap/action](keymap-action.md) indirection in Normal mode. Log is inline because its handler is small (≤ 7 keys) and Status-specific actions like `SwitchPane` / `StageSelected` don't translate. When the slash prompt is open the keymap is bypassed entirely — every keypress is character input until `Esc`. The commit editor handlers are inline because the modal context (and `pending_op`) make a generic Action-based table awkward.

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

## Log-view actions

| Key | Behavior |
|---|---|
| `j` / `↓` | `move_log_selection(1)` → clamp + `refresh_log_detail` |
| `k` / `↑` | `move_log_selection(-1)` |
| `g` | jump to first commit (`i32::MIN` sentinel) |
| `G` | jump to last commit (`i32::MAX` sentinel) |
| `r` | `refresh_log` |
| `q` / `Ctrl+C` | quit |
| `Esc` | clear `error` |

`refresh_log` runs `git log --pretty=format:... -n 200`, parses, and re-runs `refresh_log_detail` for the new selection. `refresh_log_detail` runs `git show --stat <sha>` for the currently selected commit.

## Tab switching

```
switch_view(target: View)
├── if target == self.view → no-op
└── else
    ├── self.view = target
    └── refresh the target view's data:
        ├── Status → refresh_status
        └── Log    → refresh_log
```

Refresh-on-switch keeps tab labels accurate (counts) and content current. The cost is small — a `git status` or `git log -n 200` typically returns in under 100 ms.

## Slash-Command prompt lifecycle

```
Status (Normal)
  ├── '/'                → prompt = Some(Prompt::new())
  └── handle_prompt_key
        ├── Esc          → prompt = None
        ├── Ctrl-C       → quit gitgud
        ├── Enter        → dispatch_prompt(raw)
        ├── ↑/↓          → recall_prev / recall_next
        └── chars/edits  → mutate the buffer

dispatch_prompt(raw)
  ├── empty                                  → no-op (stay in Command mode)
  ├── first token != "git"                   → app.error = "unknown command: /…"
  ├── "git" with no subcommand               → app.error = "missing git subcommand"
  ├── "git commit" (no -m/-F/--message/--file) → prompt = None, open_commit_editor()
  ├── "git rebase -i" / "--interactive"      → app.error = "interactive rebase not yet supported"
  ├── "git add -p" / "--patch"               → app.error = "interactive `add -p` not yet supported"
  └── else                                   → build GitCmd from rest, run_action(cmd)
```

After a normal run the prompt stays open with an empty buffer (rapid-fire). Errors land in `app.error`, which renders **above** the prompt row in the status line — the user can keep typing or `↑` to recall and fix. Only the editor-takeover branch closes the prompt (because it's switching to a different view).

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
- [`prompt`](prompt.md) — state behind the slash-Command mode
- [`git::log`](git-log.md) — parser feeding `App.log`
- [Log view](log-view.md) — render layer for the Log tab
- [`ui`](ui.md) — pure render of `App` per frame
