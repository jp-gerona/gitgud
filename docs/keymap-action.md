# `action` & `keymap`

Sources: [`src/action.rs`](../src/action.rs), [`src/keymap.rs`](../src/keymap.rs)

## Purpose

A typed-intent layer between raw `KeyEvent`s and the App's state mutations. Keys go through `keymap::key_to_action` to produce an `Action`; the App matches on the `Action`. This indirection makes intent explicit and gives a single place to remap keys later (per-user config, per-view bindings) without touching the handlers.

**Scope:** the keymap/action layer only handles the Status view in **Normal mode**. Two things bypass it:

- **The commit editor** — its modal context (Normal/Insert/Command + pending operators like `gg`/`dd`) makes a generic Action table awkward, so keys are dispatched inline in [`app.rs`](app.md).
- **The slash prompt** — when `app.prompt.is_some()`, every keystroke is character input into the [`prompt`](prompt.md) buffer (or a control key like `Esc`/`Enter`/`↑`). The keymap is bypassed entirely. `/` is also checked in `handle_status_key` *before* the keymap call, so it isn't a regular `Action` — the keymap table stays clean of mode-entry keys.

## `Action`

```rust
pub enum Action {
    Quit,
    MoveSelection(i32),
    SwitchPane,
    Refresh,
    StageSelected,
    UnstageSelected,
    DiscardSelected,
    Commit,
    Dismiss,
}
```

`Copy + Debug`. Variants are coarse intents, not key names — `MoveSelection(1)` could be triggered by `j`, `Down`, or eventually a mouse wheel.

## `keymap::key_to_action`

```rust
pub fn key_to_action(k: KeyEvent) -> Option<Action>;
```

A flat match. Returns `None` for unbound keys so the dispatcher can ignore them silently.

| Key | Action |
|---|---|
| `Ctrl+C` | `Quit` |
| `q` | `Quit` |
| `j` / `↓` | `MoveSelection(1)` |
| `k` / `↑` | `MoveSelection(-1)` |
| `Tab` | `SwitchPane` |
| `r` | `Refresh` |
| `s` | `StageSelected` |
| `u` | `UnstageSelected` |
| `X` | `DiscardSelected` |
| `c` | `Commit` |
| `Esc` | `Dismiss` |
| (anything else) | `None` |

## How the App consumes Actions

In `App::handle_status_key`:

```rust
let Some(action) = keymap::key_to_action(k) else { return; };
match action {
    Action::Quit              => self.should_quit = true,
    Action::MoveSelection(d)  => self.move_selection(d),
    Action::SwitchPane        => { /* toggle Pane, refresh_diff */ }
    Action::Refresh           => self.refresh_status(),
    Action::StageSelected     => self.stage_selected(),
    Action::UnstageSelected   => self.unstage_selected(),
    Action::Commit            => self.open_commit_editor(),
    Action::Dismiss           => self.error = None,
}
```

Notice that `StageSelected` / `UnstageSelected` are pane-gated inside the handler — `s` on the Staged pane is a no-op, not an error. The keymap doesn't know about panes; that's the handler's job.

`DiscardSelected` is pane-aware in a richer sense: the handler picks a different `GitCmd` per pane and per file status (untracked → `git clean -fd`, modified → `git restore`, staged → `git restore --staged --worktree --source=HEAD`). It also doesn't execute directly — it stages a [`PendingConfirm`](app.md#destructive-ops-pendingconfirm) which the next key (`y` or anything else) resolves.

## Growing the keymap

When more Status-view bindings appear (discard, stage-all, stash, etc.):

1. Add an `Action` variant.
2. Add a `KeyCode` → `Action` row in `keymap::key_to_action`.
3. Handle the variant in `App::handle_status_key` (build a `GitCmd`, run it via `run_action`).

When new **views** appear (log, branches, …) and need different bindings for the same keys (e.g. `j` means "next commit" in log, "next file" in status), `keymap::key_to_action` will need a `(View, KeyEvent) → Option<Action>` shape. That refactor is deferred until the second view actually needs it.

## Related

- [`app`](app.md) — consumes Actions
- [`event`](event.md) — produces the KeyEvents
