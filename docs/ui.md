# `ui`

Source: [`src/ui/`](../src/ui/)

## Purpose

The render layer. Pure functions of `&App`: no state, no side effects beyond writing to the [`Frame`](https://docs.rs/ratatui/0.29/ratatui/struct.Frame.html). One entry point — `ui::draw(&mut Frame, &App)` — is called from `App::run` each tick.

## Top-level layout

```
ui::draw splits the frame vertically:
┌──────────────────────────────────────────┐
│  1.Status (3)  2.Log (52)   (1 row)      │   — tab_bar::draw, ONLY on tabbed views
├──────────────────────────────────────────┤
│                                          │
│        active view (Min(0))              │   — views::status / views::log
│                                          │     or views::commit (full-screen modal)
│                                          │
├──────────────────────────────────────────┤
│  $ git ...                  (1 row)      │   — command_bar::draw
├──────────────────────────────────────────┤
│  /git █                     (1 row)      │   — prompt_bar::draw, ONLY in Command mode
├──────────────────────────────────────────┤
│  [1/2] tab ...              (1 row)      │   — status_line (in ui::mod)
└──────────────────────────────────────────┘
```

Two rows are conditional:

- **Tab bar** — drawn only when `app.view.is_tabbed()` (i.e. Status or Log; not CommitEditor).
- **Prompt bar** — drawn only when `app.prompt.is_some()` in a tabbed view.

The status line is view- and mode-aware. Each branch returns a `Line`. The precedence inside a tabbed view is **confirm > prompt > error > hints** — a pending confirmation always wins, because the user has to resolve it before doing anything else:

- **Confirm pending** — yellow `confirm` chip + the prompt text + `[y]` (red) yes / `[N]` (green) no.
- **Status view (Normal)** — `[1/2] tab  [Tab] pane  [j/k] move  [s] stage  [u] unstage  [X] discard  [c] commit  [/] cmd  [r] refresh  [q] quit`.
- **Status view (Diff pane focused)** — `[Tab] pane  [j/k] hunk  [s] stage hunk  [u] unstage hunk  [X] discard hunk  [Esc] back  [/] cmd  [q] quit`. Checked after `error`, before the normal Status hints.
- **Log view (Normal)** — `[1/2] tab  [j/k] move  [g/G] top/bottom  [/] cmd  [r] refresh  [q] quit`.
- **Command mode** (either tabbed view) — `[Esc] back  [↑/↓] history  [Enter] run`.
- **Error** — red `error` chip + message + `[Esc] dismiss`.
- **CommitEditor view** — `[Ctrl+C] quit gitgud`. The editor draws its own mode label and hints panel inside the view area, so the global status line stays minimal.

## `ui::tab_bar`

Renders the top-row tab strip. Each tab is `<N>.<Label> (<count>)`:

- `<N>` is the keystroke to switch (1-indexed).
- `<count>` is `app.status.entries.len()` for Status (total distinct files) and `app.log.len()` for Log. Omitted when the count is unknown / not yet loaded.
- Active tab style: `FOCUS_BORDER` (cyan) + bold + reversed background. Inactive: dark gray. The reversed-bg trick gives a "raised chip" look without a border.

Tab order is hard-coded in `TABS` constant — currently `[Status, Log]`. To add a tab, append here and add a number key in `App::try_handle_tab_key`.

## `ui::prompt_bar`

Renders the slash-Command prompt: a cyan-bold `/` followed by the buffer chars from `app.prompt`. Positions the terminal cursor at the buffer's char-cursor offset (Unicode-safe; `cursor` is a char index). Pure function of `&App` — no state of its own.

The widget is only invoked when `app.prompt.is_some()` and the active view is `Status`; `ui::draw` allocates an extra 1-row slot above the status line for it in that case.

## `ui::command_bar`

Renders the most recent entry in `History` (the `.display()` of the last `GitCmd`) prefixed with `$ ` in cyan. If the history is empty, just `$ ` is shown. This is the teaching surface — by the contract that every git call flows through `GitCmd`, **whatever appears here is exactly the command that was just executed.**

## `ui::theme`

Colour constants only, no styles. The current palette:

| Constant | Color | Used for |
|---|---|---|
| `STAGED` | Green | added / renamed / copied entries |
| `UNSTAGED` | Yellow | modified / type-change entries |
| `UNTRACKED` | Red | deleted / untracked / unmerged entries |
| `FOCUS_BORDER` | Cyan | active pane border, commit editor border |
| `DIM_BORDER` | DarkGray | inactive pane / diff / hints border |
| `COMMAND_BAR_FG` | Cyan | command bar text |

## `ui::views::status`

Renders the Status view as a horizontal 40/60 split:

```
┌────────────┬─────────────────────────────────┐
│ Unstaged   │                                 │
│ ─────────  │           Diff                  │
│ Staged     │                                 │
└────────────┴─────────────────────────────────┘
```

Each file row shows a status symbol (`M`, `A`, `?`, …) colored by the pane's perspective (`worktree` for Unstaged, `index` for Staged). The selected row uses a reversed style; only the **focused** pane sets its `ListState` selection, so the unfocused pane shows no highlight. (Earlier iterations highlighted both panes simultaneously, which was visually confusing.)

When the diff parsed into hunks (`app.diff_parsed`), the panel is
hunk-aware: the title shows ` Diff (hunk h/n) `, every body line gets a
2-col gutter, and the **selected hunk**'s lines get a cyan-bold `▌ ` gutter
instead of blanks. The border turns `FOCUS_BORDER` (cyan) when
`app.diff_focused`; while the Diff pane has focus neither file pane shows a
selection highlight (both borders dim). Binary / empty diffs fall back to
the plain line rendering with a ` Diff ` title.

The diff panel colorizes lines by leading character:

| Prefix | Color |
|---|---|
| `+++` / `---` | Cyan bold |
| `+` | Green |
| `-` | Red |
| `@@` | Magenta |
| `diff ` | Yellow bold |
| else | default |

## `ui::views::commit`

Renders the modal commit editor as three stacked widgets:

```
┌─ Commit message  (subject ≤50, body ≤72) ─┐
│ buffer rendering, with column-limit       │
│ overflow in yellow (50→72) and red+bold   │
│ (>72)                                      │
└────────────────────────────────────────────┘
 -- NORMAL --                                     ← vim status row (1 row, no border)
┌─ NORMAL mode ──────────────────────────────┐
│ context-specific hints, swaps per mode    │
└────────────────────────────────────────────┘
```

The vim status row shows one of:

- A yellow-bold `status_message` (vim `E…` errors, commit failures) when set.
- `-- NORMAL --` (cyan) / `-- INSERT --` (green) when no message.
- `:typed_command` plus a terminal-cursor in Command mode.

The hints panel content switches per mode — Normal shows the full motion/insert/delete/command reference; Insert and Command modes show their narrower sets. See [`commit_editor.md`](commit-editor.md) for the command set.

### Cursor placement

Only one terminal cursor exists per frame; `Frame::set_cursor_position` is called in `place_cursor` once, choosing where based on `mode`:

- **Command mode** — at the end of the typed `:command` in the vim status row.
- **Normal / Insert** — at `(inner.x + col, inner.y + row)` inside the editor block, clipped if out of view (no scrolling yet — long messages can put the cursor off-screen).

## Adding a view

1. Add a variant to `app::View`.
2. Add a module under `src/ui/views/`.
3. Branch in `ui::draw` (and `ui::status_line` if the view wants different hints).
4. The view function is `pub fn draw(f: &mut Frame, area: Rect, app: &App)` — pure.

## Related

- [`app`](app.md) — owns the state being rendered
- [`commit_editor`](commit-editor.md) — state behind `views::commit`
- [`prompt`](prompt.md) — state behind `prompt_bar`
- [`git::status`](git-status.md) — data behind `views::status`
- [`git::log`](git-log.md) — data behind `views::log`
- [`git::diff`](git-diff.md) — hunk data behind the Diff pane
- [Log view deep dive](log-view.md) — layout, row rendering, ref chip colors
