# `git::log`

Source: [`src/git/log.rs`](../src/git/log.rs)

## Purpose

Parser for `git log`. Asks git for a NUL-separated record format so that subjects, author names, and ref decorations can contain commas, colons, or other delimiters without ambiguity. Mirrors the pattern from [`git::status`](git-status.md), which uses the same NUL trick on `--porcelain=v1 -z`.

## Output format

```
git log --pretty=format:'%H%x00%h%x00%an%x00%ar%x00%s%x00%D' -n 200
```

| Token | Field | Maps to |
|---|---|---|
| `%H` | full SHA (40 hex) | `LogEntry::sha` |
| `%h` | short SHA | `LogEntry::short_sha` |
| `%an` | author name | `LogEntry::author` |
| `%ar` | relative author date ("2 hours ago") | `LogEntry::when` |
| `%s` | subject (first line of message) | `LogEntry::subject` |
| `%D` | decoration (ref names, comma-separated) | `LogEntry::refs` (split on `, `) |

`%x00` is a literal NUL byte; `\n` between records is the implicit trailing newline after each `--pretty=format:...` line.

200 is the default fetch cap. Big enough to cover any normal session, small enough that even huge repos render instantly. Lazy-load / pagination is deferred until someone actually needs the 201st commit.

## API

```rust
pub struct LogEntry {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub when: String,
    pub subject: String,
    pub refs: Vec<String>,   // [], or ["HEAD -> main", "origin/main", "tag: v1.0"]
}

pub struct LogList {
    pub entries: Vec<LogEntry>,
}

pub fn cmd() -> GitCmd;
pub fn load() -> Result<LogList>;
pub fn show_stat_cmd(sha: &str) -> GitCmd;
pub fn parse(bytes: &[u8]) -> LogList;
```

`load` returns an empty `LogList` (not an error) for empty repos — detects the "does not have any commits" / "bad default revision" stderr signature. UI then renders "Log (empty)".

`show_stat_cmd(sha)` builds `git show --stat --no-color <sha>` for the right-pane detail. We deliberately omit `--patch` so a 5000-line commit doesn't drown the screen; the full diff body is a follow-up.

## Parser shape

Line-oriented: split bytes on `\n`, skip empties, then `splitn(6, '\0')` each line. Lines that don't yield 6 fields are silently skipped (defensive — git format never produces these, but malformed input shouldn't crash the view).

`parse_refs` splits `%D` on the literal `", "` (comma-space) per git's `log-format(7)` spec. `HEAD -> main` stays one entry; `HEAD -> main, origin/main` becomes two. Tags keep their `tag: ` prefix so the renderer can style them differently.

## Why `String::from_utf8_lossy`

Subject lines and author names come from user input and may not be valid UTF-8 on weirdly-configured repos. Lossy decode keeps the parser infallible — invalid bytes render as `U+FFFD`, which is fine for our display surface.

## Tests

Six unit tests on the byte-level parser cover: ref decoration with multiple entries, multiple commits in one fixture, empty input, malformed lines (skipped silently), tag prefix preservation, and subjects containing the delimiter chars (`,` and `:`).

## Related

- [`git::GitCmd`](git-cmd.md) — every dispatched git call goes through this
- [`git::status`](git-status.md) — sibling parser, same NUL-separated trick
- [`ui::views::log`](log-view.md) — renders the parsed data
- [`app`](app.md) — owns the `LogList`, drives `refresh_log` and `refresh_log_detail`
