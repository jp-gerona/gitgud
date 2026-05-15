//! `git log` parser. We ask git for a NUL-separated record format so that
//! subjects, author names, and ref lists can contain commas, spaces, or other
//! delimiters without ambiguity. Commits are separated by newline (the implicit
//! trailing `\n` after each format line).

use super::{GitCmd, runner};
use anyhow::Result;

const DEFAULT_LIMIT: usize = 200;

/// `%H \0 %h \0 %an \0 %ar \0 %s \0 %D` — full SHA, short SHA, author name,
/// relative author date, subject, decorated refs.
const FORMAT: &str = "%H%x00%h%x00%an%x00%ar%x00%s%x00%D";

#[derive(Clone, Debug, Default)]
pub struct LogEntry {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub when: String,
    pub subject: String,
    /// Each ref as a separate string. `HEAD -> main` stays as one entry; tags
    /// stay prefixed with `tag: `. Empty when the commit has no decorations.
    pub refs: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LogList {
    pub entries: Vec<LogEntry>,
}

impl LogList {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn cmd() -> GitCmd {
    GitCmd::new("log")
        .arg(format!("--pretty=format:{FORMAT}"))
        .arg("-n")
        .arg(DEFAULT_LIMIT.to_string())
}

pub fn load() -> Result<LogList> {
    let out = runner::run(&cmd())?;
    if !out.success() {
        // Empty repo / no commits yet → render empty list, not an error.
        let stderr = out.stderr_str();
        if stderr.contains("does not have any commits") || stderr.contains("bad default revision") {
            return Ok(LogList::default());
        }
        anyhow::bail!("git log failed: {}", stderr.trim());
    }
    Ok(parse(&out.stdout))
}

/// Detail for a single commit — message + per-file change summary. Used for
/// the right pane in the Log view. We use `git show --stat` (no `--patch`) so
/// large diffs don't blow up the screen; full diff body can come later.
pub fn show_stat_cmd(sha: &str) -> GitCmd {
    GitCmd::new("show").arg("--stat").arg("--no-color").arg(sha)
}

/// Parse `git log --pretty=format:...` output produced by [`FORMAT`].
pub fn parse(bytes: &[u8]) -> LogList {
    let s = String::from_utf8_lossy(bytes);
    let mut entries = Vec::new();
    for line in s.split('\n') {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(6, '\0').collect();
        if parts.len() < 6 {
            continue;
        }
        let refs = parse_refs(parts[5]);
        entries.push(LogEntry {
            sha: parts[0].to_string(),
            short_sha: parts[1].to_string(),
            author: parts[2].to_string(),
            when: parts[3].to_string(),
            subject: parts[4].to_string(),
            refs,
        });
    }
    LogList { entries }
}

/// Split git's `%D` decoration field on `, `. Empty input → empty Vec.
fn parse_refs(s: &str) -> Vec<String> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(", ").map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_commit_with_refs() {
        let input = b"abc1234567890abc1234567890abc1234567890ab\0abc1234\0Alice\x002 hours ago\0fix(parser): tighten edge case\0HEAD -> main, origin/main\n";
        let log = parse(input);
        assert_eq!(log.entries.len(), 1);
        let e = &log.entries[0];
        assert!(e.sha.starts_with("abc1234"));
        assert_eq!(e.short_sha, "abc1234");
        assert_eq!(e.author, "Alice");
        assert_eq!(e.when, "2 hours ago");
        assert_eq!(e.subject, "fix(parser): tighten edge case");
        assert_eq!(e.refs, vec!["HEAD -> main", "origin/main"]);
    }

    #[test]
    fn parses_multiple_commits() {
        let input = b"sha1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\0sha1aaa\0Alice\x001 hour ago\0first\0\nsha2bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\0sha2bbb\0Bob\x002 days ago\0second\0HEAD -> main\n";
        let log = parse(input);
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.entries[0].subject, "first");
        assert!(log.entries[0].refs.is_empty());
        assert_eq!(log.entries[1].subject, "second");
        assert_eq!(log.entries[1].refs, vec!["HEAD -> main"]);
    }

    #[test]
    fn empty_input_yields_empty_log() {
        assert!(parse(b"").entries.is_empty());
        assert!(parse(b"\n").entries.is_empty());
    }

    #[test]
    fn malformed_line_is_skipped() {
        // Only 3 NUL-separated fields where 6 are expected.
        let input = b"sha1\0short\0Alice\n";
        assert!(parse(input).entries.is_empty());
    }

    #[test]
    fn refs_with_tag_prefix() {
        let input = b"sha1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\0sha1aaa\0Alice\x003 weeks ago\0release\0tag: v1.0, origin/main\n";
        let log = parse(input);
        assert_eq!(
            log.entries[0].refs,
            vec!["tag: v1.0".to_string(), "origin/main".to_string()]
        );
    }

    #[test]
    fn subject_with_special_chars() {
        let input = b"sha1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\0sha1aaa\0Alice\x005 minutes ago\0fix: handle , and : in subject\0\n";
        let log = parse(input);
        assert_eq!(log.entries[0].subject, "fix: handle , and : in subject");
    }
}
