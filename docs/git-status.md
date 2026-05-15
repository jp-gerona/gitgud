# `git::status`

Source: [`src/git/status.rs`](../src/git/status.rs)

## Purpose

Parse `git status --porcelain=v1 -z` into a `StatusList` of `FileEntry` records. This drives the staged / unstaged / untracked panes in the Status view, the diff preview's choice of `--cached` vs working-tree diff, and the post-stage refresh.

## Why `--porcelain=v1 -z`

| Flag | Reason |
|---|---|
| `--porcelain=v1` | Stable, machine-readable format. v2 is richer (sub-status, mode bits, OIDs) but we don't need it yet. |
| `-z` | NUL terminates records and the rename separator, so newlines in file paths don't confuse the parser. Required for correctness. |

The flags are wired up in `git::status::cmd()` so the teaching bar shows the exact invocation.

## Output format (porcelain v1 -z)

Each record is one of:

```
XY SP path NUL                   (most entries)
XY SP newpath NUL origpath NUL   (renames and copies — when X or Y is 'R' or 'C')
```

Where `X` is the **index** (staged) status and `Y` is the **worktree** (unstaged) status. Special cases: `??` for untracked, `!!` for ignored.

## Data model

```rust
pub enum FileStatus {
    Unmodified, Added, Modified, Deleted, Renamed, Copied,
    Untracked, Ignored, TypeChange, Unmerged,
    Unknown(char),
}

pub struct FileEntry {
    pub path: String,
    pub orig_path: Option<String>,   // for renames/copies
    pub index: FileStatus,
    pub worktree: FileStatus,
}

impl FileEntry {
    pub fn is_staged(&self) -> bool;     // index ≠ Unmodified/Untracked/Ignored
    pub fn is_unstaged(&self) -> bool;   // worktree ≠ Unmodified
}

pub struct StatusList {
    pub entries: Vec<FileEntry>,
}

impl StatusList {
    pub fn staged(&self) -> impl Iterator<Item = &FileEntry>;
    pub fn unstaged(&self) -> impl Iterator<Item = &FileEntry>;
}
```

Untracked entries (`??`) appear only in `unstaged()` (worktree side). Files with changes on both sides (e.g. `MM`) appear in both iterators.

## Parser walkthrough

The parser walks the bytes linearly. For each record:

```
i = 0
while i < len:
    x = bytes[i]; y = bytes[i+1]    # status codes
    # bytes[i+2] is the space separator
    i += 3
    path_start = i
    advance i until bytes[i] == 0   # path runs to NUL
    path = utf8_lossy(bytes[path_start..i])
    i += 1                          # skip NUL

    if x or y is 'R' or 'C':
        orig_start = i
        advance i until bytes[i] == 0
        orig_path = utf8_lossy(...)
        i += 1

    emit FileEntry { ... }
```

Paths are decoded via `String::from_utf8_lossy` — invalid byte sequences become `U+FFFD` rather than producing parse errors. Practical for the TUI; a future enhancement could surface a warning when this triggers.

## Tests

`src/git/status.rs` carries six unit tests covering:

- Modified-unstaged (`" M src/foo.rs\0"`)
- Modified-staged (`"M  src/foo.rs\0"`)
- Modified-both (`"MM src/foo.rs\0"`)
- Untracked (`"?? new.txt\0"`)
- Multiple entries in one record stream
- Rename (`"R  new.rs\0old.rs\0"` with two NUL-separated paths)

These pin the byte-level format so a porcelain change wouldn't slip in undetected.

## Known limitations

- **Ignored entries** (`!!`) are parsed but the Status view doesn't currently show them — they'd appear in `unstaged()` if encountered (since they pass `is_unstaged()`). In practice gitgud doesn't pass `--ignored`, so `!!` doesn't appear.
- **Unmerged** (`U`) is recognized but not specially rendered. A future merge-conflict workflow would handle this.
- **Submodules** are treated as regular files — no recursion into their own status.

## Related

- [`git::runner`](git-runner.md) — executes the porcelain command
- [`app`](app.md) — calls `status::load()` from `App::refresh_status`
- [`ui/views/status`](ui.md) — renders `StatusList` into the two panes
