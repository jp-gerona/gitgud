# Log view

Sources: [`src/ui/views/log.rs`](../src/ui/views/log.rs), [`src/ui/tab_bar.rs`](../src/ui/tab_bar.rs)

## Purpose

The "what happened on this branch" surface — paired with [Status](../src/ui/views/status.rs) to complete the "what's the current state, what got us here" loop. Press `2` (or `[`/`]`) to switch in, or type `/git log` from any tab and the prompt auto-routes you here.

## Layout

```
┌─ Log (52) ─────────────┬─ ab12c34 ───────────────────────────────┐
│ ab12c34 Alice 2h ago … │ commit ab12c34567890...                 │
│ d4e5f67 Bob   1d ago … │ Author: Alice <a@example.com>           │
│ 78901aa Alice 3d ago … │ Date:   Tue Mar 4 14:22:11 2025          │
│ ...                    │                                          │
│                        │     fix(parser): tighten edge case      │
│                        │                                          │
│                        │  src/git/log.rs | 14 ++++++++------     │
│                        │  1 file changed, 8 insertions(+), 6 -    │
└────────────────────────┴──────────────────────────────────────────┘
```

40 / 60 split. Left = commit list with selection; right = `git show --stat` for the selected commit.

## Row rendering

```
 <short_sha>  <author>  <when>  <subject>  <ref chips…>
```

- **`short_sha`** in cyan bold — the visual anchor; same color as the `Status (N)` count and command bar text. Easy scan target.
- **`<author>`** and **`<when>`** in dark gray. Truncated to 12 and 14 chars respectively (with an ellipsis) so long names don't push the subject off-screen.
- **`<subject>`** in default style. Whatever fits.
- **Ref chips** are inline colored badges:

| Ref kind | Background |
|---|---|
| `HEAD -> …` | cyan (bold) |
| `tag: …` | yellow (bold) |
| contains `/` (`origin/main`, `upstream/foo`) | magenta |
| anything else (local branch) | green |

This is the same color logic as lazygit / fugitive — colors function as a type label.

## Detail pane styling

`git show --stat` output is line-classified at render time:

| Line shape | Style |
|---|---|
| `commit <sha>` header | yellow bold |
| `Author:` / `Date:` | dark gray |
| `<file> | N +-…` (the per-file diff bars) | cyan |
| trailing `N files changed, ...` | dark gray |
| else (message body) | default |

This is deliberately gentler than the per-character diff coloring in the Status view's diff pane — `--stat` is structural, not a hunk.

## Tab bar

```
 1.Status (3)   2.Log (52)
```

- Rendered by `src/ui/tab_bar.rs`. 1-row strip at the very top of the frame.
- Active tab: cyan + bold + reversed background. Inactive: dark gray.
- Numbered prefix (`1.`, `2.`) is the keystroke to switch — single-press, like vim's `<C-w>` numbered windows.
- The count (`(3)`, `(52)`) updates live: Status shows total entries (`unstaged + staged + untracked`, distinct files), Log shows fetched commit count. Empty log shows `Log` with no count.

## Navigation

| Key | Action |
|---|---|
| `1` | switch to Status tab |
| `2` | switch to Log tab |
| `[` / `]` | cycle previous / next tab |
| `j` / `↓` | move selection down |
| `k` / `↑` | move selection up |
| `g` | jump to first commit |
| `G` | jump to last commit |
| `r` | refresh log (re-runs `git log`) |
| `/` | enter slash-Command mode |
| `Esc` | dismiss error |
| `q` / `Ctrl+C` | quit gitgud |

The selected commit's detail pane refreshes automatically on selection change — each move runs `git show --stat <sha>`.

## Slash-command auto-switch

The dispatcher in [`app::dispatch_prompt`](app.md) treats `/git log` and `/git status` as **view-defining**: they switch to the matching tab instead of running through the generic `run_action` path.

For v1, args on these commands are not honored — `/git log --oneline -n 5` switches to Log and runs the canonical query (200 entries, full format). The user's literal command lives in prompt history (`↑`) but not the command bar. Tracked as a future feature in [issue #1](https://github.com/jp-gerona/gitgud/issues/1).

## Why a separate handler instead of `keymap::key_to_action`

The keymap module is Status-only by design — `Action::SwitchPane`, `Action::StageSelected`, etc. don't translate to Log. Log's navigation is small enough (≤ 7 keys) that inline matching in `handle_log_normal_key` is clearer than threading a second view into the keymap dispatch.

When a third view ships (Branches, Stash, …), the right move is probably *not* to grow the `Action` enum further but to give each view its own `handle_*_normal_key`. Shared things like `q` for quit and `r` for refresh duplicate cheaply; coupling unrelated views through a single action table would be more drag than DRY.

## Related

- [`git::log`](git-log.md) — the parser feeding this view
- [`app`](app.md) — owns `log`, `log_selected`, `log_detail`, drives refresh
- [`ui`](ui.md) — layout glue, tab bar inclusion
- [`prompt`](prompt.md) — slash dispatcher's `/git log` auto-switch
