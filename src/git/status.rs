use super::{GitCmd, runner};
use anyhow::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileStatus {
    Unmodified,
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    TypeChange,
    Unmerged,
    Unknown(char),
}

impl FileStatus {
    pub fn from_char(c: char) -> Self {
        match c {
            ' ' => Self::Unmodified,
            'A' => Self::Added,
            'M' => Self::Modified,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            '?' => Self::Untracked,
            '!' => Self::Ignored,
            'T' => Self::TypeChange,
            'U' => Self::Unmerged,
            other => Self::Unknown(other),
        }
    }

    pub fn symbol(&self) -> char {
        match self {
            Self::Unmodified => ' ',
            Self::Added => 'A',
            Self::Modified => 'M',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Copied => 'C',
            Self::Untracked => '?',
            Self::Ignored => '!',
            Self::TypeChange => 'T',
            Self::Unmerged => 'U',
            Self::Unknown(c) => *c,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    #[allow(dead_code)] // surfaced once the rename view exists
    pub orig_path: Option<String>,
    /// Index (staged) status.
    pub index: FileStatus,
    /// Worktree (unstaged) status.
    pub worktree: FileStatus,
}

impl FileEntry {
    pub fn is_staged(&self) -> bool {
        !matches!(
            self.index,
            FileStatus::Unmodified | FileStatus::Untracked | FileStatus::Ignored
        )
    }

    pub fn is_unstaged(&self) -> bool {
        !matches!(self.worktree, FileStatus::Unmodified)
    }
}

#[derive(Clone, Debug, Default)]
pub struct StatusList {
    pub entries: Vec<FileEntry>,
}

impl StatusList {
    pub fn staged(&self) -> impl Iterator<Item = &FileEntry> {
        self.entries.iter().filter(|e| e.is_staged())
    }

    pub fn unstaged(&self) -> impl Iterator<Item = &FileEntry> {
        self.entries.iter().filter(|e| e.is_unstaged())
    }
}

pub fn cmd() -> GitCmd {
    GitCmd::new("status").arg("--porcelain=v1").arg("-z")
}

pub fn load() -> Result<StatusList> {
    let out = runner::run(&cmd())?;
    parse(&out.stdout)
}

/// Parse `git status --porcelain=v1 -z` output.
///
/// Each record is `XY SP path NUL`, except renames/copies which append
/// `origPath NUL`. Untracked entries use `?? path NUL`.
pub fn parse(bytes: &[u8]) -> Result<StatusList> {
    let mut entries = Vec::new();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let x = bytes[i] as char;
        let y = bytes[i + 1] as char;
        // bytes[i+2] is the space separator
        i += 3;

        let path_start = i;
        while i < bytes.len() && bytes[i] != 0 {
            i += 1;
        }
        let path = String::from_utf8_lossy(&bytes[path_start..i]).into_owned();
        if i < bytes.len() {
            i += 1; // consume NUL
        }

        let mut orig_path = None;
        if x == 'R' || x == 'C' || y == 'R' || y == 'C' {
            let orig_start = i;
            while i < bytes.len() && bytes[i] != 0 {
                i += 1;
            }
            orig_path = Some(String::from_utf8_lossy(&bytes[orig_start..i]).into_owned());
            if i < bytes.len() {
                i += 1;
            }
        }

        let (index, worktree) = if x == '?' && y == '?' {
            (FileStatus::Untracked, FileStatus::Untracked)
        } else if x == '!' && y == '!' {
            (FileStatus::Ignored, FileStatus::Ignored)
        } else {
            (FileStatus::from_char(x), FileStatus::from_char(y))
        };

        entries.push(FileEntry {
            path,
            orig_path,
            index,
            worktree,
        });
    }
    Ok(StatusList { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modified_unstaged() {
        let s = parse(b" M src/foo.rs\0").unwrap();
        assert_eq!(s.entries.len(), 1);
        let e = &s.entries[0];
        assert_eq!(e.path, "src/foo.rs");
        assert_eq!(e.index, FileStatus::Unmodified);
        assert_eq!(e.worktree, FileStatus::Modified);
        assert!(e.is_unstaged() && !e.is_staged());
    }

    #[test]
    fn parses_modified_staged() {
        let s = parse(b"M  src/foo.rs\0").unwrap();
        let e = &s.entries[0];
        assert_eq!(e.index, FileStatus::Modified);
        assert_eq!(e.worktree, FileStatus::Unmodified);
        assert!(e.is_staged() && !e.is_unstaged());
    }

    #[test]
    fn parses_modified_both() {
        let s = parse(b"MM src/foo.rs\0").unwrap();
        let e = &s.entries[0];
        assert!(e.is_staged() && e.is_unstaged());
    }

    #[test]
    fn parses_untracked() {
        let s = parse(b"?? new.txt\0").unwrap();
        let e = &s.entries[0];
        assert_eq!(e.path, "new.txt");
        assert_eq!(e.index, FileStatus::Untracked);
        assert!(!e.is_staged());
        assert!(e.is_unstaged());
    }

    #[test]
    fn parses_multiple_entries() {
        let s = parse(b"M  staged.rs\0 M unstaged.rs\0?? new.txt\0").unwrap();
        assert_eq!(s.entries.len(), 3);
        assert_eq!(s.staged().count(), 1);
        assert_eq!(s.unstaged().count(), 2);
    }

    #[test]
    fn parses_rename() {
        // `R  new\0old\0` — renamed and staged
        let s = parse(b"R  new.rs\0old.rs\0").unwrap();
        assert_eq!(s.entries.len(), 1);
        assert_eq!(s.entries[0].path, "new.rs");
        assert_eq!(s.entries[0].orig_path.as_deref(), Some("old.rs"));
    }
}
