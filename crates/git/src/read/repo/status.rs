//! What is not committed: the files staged, changed, untracked and in conflict.

use std::ffi::OsString;

use model::state::{ChangeKind, ConflictKind, ConflictedFile, FileChange, WorkingTree};

use crate::read::repo::fields::{arguments, unexpected};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "what is not committed";

/// How many fields, separated by spaces, the line of a file changed has, the path last: `1 XY sub mH mI mW hH hI path`.
const ORDINARY_FIELDS: usize = 9;
/// The same for a file renamed or copied, which has the score before the path: `2 XY … hI Xscore path`.
const RENAMED_FIELDS: usize = 10;
/// The same for a file in conflict, which has the three stages: `u XY sub m1 m2 m3 mW h1 h2 h3 path`.
const UNMERGED_FIELDS: usize = 11;

/// `git status --porcelain=v2 --branch -z --untracked-files=all`: every untracked file, not its folder.
#[must_use]
pub fn status_arguments() -> Vec<OsString> {
    arguments(&["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=all"])
}

/// `git status --porcelain=v2 -z --untracked-files=all`, run in another worktree to count what is not committed there.
#[must_use]
pub fn worktree_status_arguments() -> Vec<OsString> {
    arguments(&["status", "--porcelain=v2", "-z", "--untracked-files=all"])
}

/// What `git status --porcelain=v2 -z` printed: a file changed in both the index and the working tree is in both
/// lists, a rename keeps its old name, and a file in conflict is only in conflict. The headers of `--branch` and the
/// ignored files are left out.
///
/// # Errors
///
/// When a line is of no kind git documents, or its codes are not.
pub fn parse_status(output: &[u8]) -> Result<WorkingTree, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut working_tree = WorkingTree::default();
    let mut fields = text.split('\0').filter(|field| !field.is_empty());
    while let Some(field) = fields.next() {
        let (kind, _) = field.split_once(' ').unwrap_or((field, ""));
        match kind {
            "1" => add_change(&mut working_tree, &change(field, ORDINARY_FIELDS)?, None)?,
            "2" => {
                // The path before the rename or the copy is the next field.
                let original = fields.next().ok_or_else(|| unexpected(WHAT, field))?;
                add_change(&mut working_tree, &change(field, RENAMED_FIELDS)?, Some(original))?;
            }
            "u" => {
                let line = change(field, UNMERGED_FIELDS)?;
                working_tree.conflicted.push(ConflictedFile {
                    path: line.path.to_owned(),
                    kind: conflict_kind(line.codes).ok_or_else(|| unexpected(WHAT, field))?,
                });
            }
            "?" => working_tree
                .untracked
                .push(field.get(2..).unwrap_or_default().to_owned()),
            // The headers of `--branch`, and the ignored files, which are not asked for.
            "#" | "!" => {}
            _ => return Err(unexpected(WHAT, field)),
        }
    }
    Ok(working_tree)
}

/// What is not committed in the worktree git runs in.
pub(super) fn read_status(git: &Git) -> Result<WorkingTree, GitError> {
    parse_status(&git.run_read(&status_arguments())?)
}

/// The two codes and the path of a line.
struct Line<'a> {
    codes: &'a str,
    path: &'a str,
}

/// The codes and the path of a line of a kind with so many fields. The path is the rest of the line, spaces and all.
fn change(field: &str, count: usize) -> Result<Line<'_>, GitError> {
    let parts: Vec<&str> = field.splitn(count, ' ').collect();
    match (parts.len() == count, parts.get(1), parts.last()) {
        (true, Some(codes), Some(path)) => Ok(Line { codes, path }),
        _ => Err(unexpected(WHAT, field)),
    }
}

/// Adds a file changed or renamed to the lists of the sides it changed on: the index (the first code) and the working
/// tree (the second). The side that renamed or copied it keeps its old name.
fn add_change(working_tree: &mut WorkingTree, line: &Line<'_>, original: Option<&str>) -> Result<(), GitError> {
    let mut codes = line.codes.chars();
    let (Some(staged), Some(unstaged), None) = (codes.next(), codes.next(), codes.next()) else {
        return Err(unexpected(WHAT, line.codes));
    };
    let file = |kind: ChangeKind| FileChange {
        path: line.path.to_owned(),
        original_path: matches!(kind, ChangeKind::Renamed | ChangeKind::Copied)
            .then(|| original.map(str::to_owned))
            .flatten(),
        kind,
    };
    if let Some(kind) = change_kind(staged)? {
        working_tree.staged.push(file(kind));
    }
    if let Some(kind) = change_kind(unstaged)? {
        working_tree.unstaged.push(file(kind));
    }
    Ok(())
}

/// What a code of a side says, none for `.`: that side did not change the file.
fn change_kind(code: char) -> Result<Option<ChangeKind>, GitError> {
    Ok(Some(match code {
        '.' => return Ok(None),
        'M' => ChangeKind::Modified,
        'T' => ChangeKind::TypeChanged,
        'A' => ChangeKind::Added,
        'D' => ChangeKind::Deleted,
        'R' => ChangeKind::Renamed,
        'C' => ChangeKind::Copied,
        other => return Err(unexpected(WHAT, &other.to_string())),
    }))
}

/// What each side did to a file in conflict, from the two codes of `git status`.
fn conflict_kind(codes: &str) -> Option<ConflictKind> {
    Some(match codes {
        "DD" => ConflictKind::BothDeleted,
        "AU" => ConflictKind::AddedByUs,
        "UD" => ConflictKind::DeletedByThem,
        "UA" => ConflictKind::AddedByThem,
        "DU" => ConflictKind::DeletedByUs,
        "AA" => ConflictKind::BothAdded,
        "UU" => ConflictKind::BothModified,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_status;
    use model::state::{ChangeKind, ConflictKind};

    #[test]
    fn reads_every_conflict_code() {
        let codes = [
            ("DD", ConflictKind::BothDeleted),
            ("AU", ConflictKind::AddedByUs),
            ("UD", ConflictKind::DeletedByThem),
            ("UA", ConflictKind::AddedByThem),
            ("DU", ConflictKind::DeletedByUs),
            ("AA", ConflictKind::BothAdded),
            ("UU", ConflictKind::BothModified),
        ];
        for (code, kind) in codes {
            let line = format!("u {code} N... 100644 100644 100644 100644 a b c a file.txt\0");
            let read = parse_status(line.as_bytes()).unwrap();
            assert_eq!(read.conflicted.len(), 1, "{code}");
            assert_eq!(read.conflicted[0].kind, kind, "{code}");
            assert_eq!(read.conflicted[0].path, "a file.txt", "{code}");
        }
        assert!(parse_status(b"u MM N... 1 2 3 4 a b c x\0").is_err());
    }

    #[test]
    fn reads_a_type_change_and_refuses_unknown_lines() {
        let read = parse_status(b"1 .T N... 100644 100644 120000 a b link\0").unwrap();
        assert_eq!(read.unstaged[0].kind, ChangeKind::TypeChanged);
        assert_eq!(read.staged, Vec::new());
        assert!(parse_status(b"1 X. N... 100644 100644 100644 a b x\0").is_err());
        assert!(parse_status(b"1 M. N...\0").is_err());
        assert!(parse_status(b"2 R. N... 100644 100644 100644 a b R100 new\0").is_err());
        assert!(parse_status(b"x what\0").is_err());
    }
}
