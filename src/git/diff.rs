//! Unified-diff parsing for hunk-level staging.
//!
//! We never ask `git` to stage a hunk for us interactively (`git add -p` is a
//! REPL we can't drive from the TUI). Instead we parse the plain `git diff`
//! output into [`Hunk`]s, let the user pick one, then reconstruct a *minimal
//! one-hunk patch* and feed it back to `git apply --cached -` (or
//! `--reverse`). This is the same trick lazygit / tig use, and it keeps every
//! git call flowing through `GitCmd` for the teaching bar.
//!
//! The parser is deliberately single-file: gitgud only ever diffs one path at
//! a time (`git diff -- <path>`), so a [`FileDiff`] is a header + its hunks.
//! Binary diffs and renames-without-content simply produce zero hunks, which
//! the caller treats as "nothing to stage by hunk here".

/// One `@@ ... @@` block: the header line plus its body (context / `+` / `-`
/// lines, including any trailing `\ No newline at end of file` marker).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    /// The verbatim `@@ -a,b +c,d @@ optional section` line.
    pub header: String,
    /// Body lines, verbatim (each without its trailing newline).
    pub lines: Vec<String>,
}

/// A single file's diff: everything before the first `@@` (the `diff --git`,
/// `index`, `---`, `+++`, mode/rename lines) plus the hunks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDiff {
    /// Lines preceding the first hunk, verbatim. Required to reconstruct a
    /// patch `git apply` will accept (it needs the `---`/`+++` headers).
    pub header_lines: Vec<String>,
    pub hunks: Vec<Hunk>,
}

impl FileDiff {
    pub fn is_empty(&self) -> bool {
        self.hunks.is_empty()
    }

    /// Reconstruct a patch containing only hunk `idx`, suitable for
    /// `git apply [--cached] [--reverse] -`. Returns `None` if the index is
    /// out of range. The result ends with a newline (git apply is picky).
    pub fn single_hunk_patch(&self, idx: usize) -> Option<String> {
        let hunk = self.hunks.get(idx)?;
        let mut s = String::new();
        for h in &self.header_lines {
            s.push_str(h);
            s.push('\n');
        }
        s.push_str(&hunk.header);
        s.push('\n');
        for l in &hunk.lines {
            s.push_str(l);
            s.push('\n');
        }
        Some(s)
    }
}

/// Parse the output of `git diff [--cached] -- <path>` for a single file.
///
/// Everything up to the first line starting with `@@` is the header; each
/// subsequent `@@` opens a new hunk that runs until the next `@@` or EOF.
/// `\ No newline at end of file` lines stay attached to the hunk they follow.
pub fn parse(text: &str) -> FileDiff {
    let mut header_lines = Vec::new();
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut seen_hunk = false;

    for line in text.lines() {
        if line.starts_with("@@") {
            seen_hunk = true;
            hunks.push(Hunk {
                header: line.to_string(),
                lines: Vec::new(),
            });
        } else if !seen_hunk {
            header_lines.push(line.to_string());
        } else if let Some(h) = hunks.last_mut() {
            h.lines.push(line.to_string());
        }
    }

    FileDiff {
        header_lines,
        hunks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Built by joining so leading-space context lines survive (a `\`-line
    // continuation in a string literal would eat them).
    fn sample() -> String {
        [
            "diff --git a/foo.rs b/foo.rs",
            "index 1111111..2222222 100644",
            "--- a/foo.rs",
            "+++ b/foo.rs",
            "@@ -1,3 +1,4 @@ fn main() {",
            " let a = 1;",
            "-let b = 2;",
            "+let b = 3;",
            "+let c = 4;",
            "@@ -10,2 +11,2 @@",
            " ctx",
            "-old",
            "+new",
        ]
        .join("\n")
            + "\n"
    }

    #[test]
    fn splits_header_and_hunks() {
        let d = parse(&sample());
        assert_eq!(d.header_lines.len(), 4);
        assert_eq!(d.header_lines[0], "diff --git a/foo.rs b/foo.rs");
        assert_eq!(d.hunks.len(), 2);
        assert_eq!(d.hunks[0].header, "@@ -1,3 +1,4 @@ fn main() {");
        assert_eq!(d.hunks[0].lines.len(), 4);
        assert_eq!(d.hunks[1].header, "@@ -10,2 +11,2 @@");
        assert_eq!(d.hunks[1].lines, vec![" ctx", "-old", "+new"]);
    }

    #[test]
    fn single_hunk_patch_roundtrips_header() {
        let d = parse(&sample());
        let p = d.single_hunk_patch(1).unwrap();
        assert!(p.starts_with("diff --git a/foo.rs b/foo.rs\n"));
        assert!(p.contains("--- a/foo.rs\n"));
        assert!(p.contains("+++ b/foo.rs\n"));
        assert!(p.contains("@@ -10,2 +11,2 @@\n"));
        assert!(p.ends_with("+new\n"));
        // Must not bleed the other hunk in.
        assert!(!p.contains("let b = 3;"));
    }

    #[test]
    fn out_of_range_patch_is_none() {
        let d = parse(&sample());
        assert!(d.single_hunk_patch(2).is_none());
    }

    #[test]
    fn no_newline_marker_stays_with_hunk() {
        let t = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n\\ No newline at end of file\n";
        let d = parse(t);
        assert_eq!(d.hunks.len(), 1);
        assert_eq!(
            d.hunks[0].lines,
            vec!["-a", "+b", "\\ No newline at end of file"]
        );
    }

    #[test]
    fn binary_or_empty_diff_has_no_hunks() {
        let d = parse("diff --git a/x b/x\nBinary files a/x and b/x differ\n");
        assert!(d.is_empty());
        assert!(d.single_hunk_patch(0).is_none());
        assert!(parse("").is_empty());
    }
}
