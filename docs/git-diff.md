# `git::diff`

Source: [`src/git/diff.rs`](../src/git/diff.rs)

## Purpose

Parse a single file's `git diff` into structured hunks so the user can stage,
unstage, or discard one hunk at a time — without driving the interactive
`git add -p` REPL (which the TUI can't sit in front of).

The trick: parse the plain `git diff [--cached] -- <path>` output, let the
user pick a hunk, then **reconstruct a minimal one-hunk patch** and feed it
back to `git apply [--cached] [--reverse] -`. Same approach lazygit / tig
use. Every git call still flows through `GitCmd`, so the command bar keeps
teaching.

## Types

```rust
pub struct Hunk     { pub header: String, pub lines: Vec<String> }
pub struct FileDiff { pub header_lines: Vec<String>, pub hunks: Vec<Hunk> }
```

- `header_lines` — everything before the first `@@` (`diff --git`, `index`,
  `---`, `+++`, mode/rename lines), verbatim. Required so the reconstructed
  patch has the headers `git apply` needs.
- Each `Hunk` keeps its `@@ ... @@` header (including any trailing section
  context) and body lines verbatim, including a trailing
  `\ No newline at end of file` marker if present.

## API

- `parse(text: &str) -> FileDiff` — split on `@@`; lines before the first
  hunk are header, the rest attach to the hunk they follow. Binary/empty
  diffs yield zero hunks.
- `FileDiff::is_empty()` — no hunks (binary, rename-only, empty).
- `FileDiff::single_hunk_patch(idx) -> Option<String>` — `header_lines` +
  hunk `idx`, newline-terminated. `None` if `idx` is out of range. This is
  the exact byte stream piped to `git apply`.

## Why single-file

gitgud only ever diffs one path at a time (`git diff -- <path>`), so a
`FileDiff` is a header plus its hunks — no multi-file demux needed. If a
future view diffs a whole tree, this parser grows a file-splitting layer
above it; the hunk shape stays.

## Consumed by

- [`app`](app.md) — `refresh_diff` parses into `App.diff_parsed`; the diff
  pane's `s` / `u` / `X` build a one-hunk patch and run
  `git apply --cached -` / `--cached --reverse -` / `--reverse -`.
- [`ui::views::status`](ui.md) — renders hunks with a focus gutter.

## Related

- [`git::status`](git-status.md) — picks the file whose diff this is
- [`app`](app.md) — hunk-staging dispatch & `PendingConfirm.stdin`
