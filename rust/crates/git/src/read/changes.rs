//! What is not committed, as the Changes pane lists it: staged, not staged, untracked and in conflict.

mod skipped;

use std::ffi::OsString;

use model::state::{ChangeKind, ConflictKind, ConflictedFile, FileChange, WorkingTree};

use crate::read::fields::{arguments, unexpected};
use crate::{Git, GitError};

pub use skipped::{
    SkippedRead, SkippedRequest, parse_skipped, read_skipped, skipped_arguments, sparse_checkout_arguments,
};

/// What is being read, for the errors.
const WHAT: &str = "what is not committed";

/// Which worktree to read: the one git runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorkingTreeRequest;

/// What git says of each changed file of the worktree it runs in: the files in conflict, the changes staged with their
/// old path for a rename or a copy, the changes not staged, and every untracked file, not its folder. A file changed
/// both in the index and in the working tree is in both lists. The read takes no lock.
///
/// Feature: changes.read-working-tree
///
/// # Errors
///
/// When git cannot be started, fails, or prints a status this does not understand.
pub fn read_working_tree(git: &Git, request: &WorkingTreeRequest) -> Result<WorkingTree, GitError> {
    parse_working_tree(&git.run_read(&working_tree_arguments(request))?)
}

/// `git status --porcelain=v1 -z --untracked-files=all`.
#[must_use]
pub fn working_tree_arguments(_request: &WorkingTreeRequest) -> Vec<OsString> {
    arguments(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])
}

/// What `git status --porcelain=v1 -z` printed. Each record is the letter of the index, the letter of the working
/// tree, a space and the path; a rename or a copy is followed by a record with its old path. Ignored files are left
/// out.
///
/// # Errors
///
/// When a record is too short to hold its two letters and a path, or a letter is not one git documents.
pub fn parse_working_tree(output: &[u8]) -> Result<WorkingTree, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut working_tree = WorkingTree::default();
    let mut records = text.split('\0').filter(|record| !record.is_empty());
    while let Some(record) = records.next() {
        let (Some(index), Some(tree), Some(path)) = (
            record.chars().next(),
            record.chars().nth(1),
            record.get(3..).filter(|path| !path.is_empty()),
        ) else {
            return Err(unexpected(WHAT, record));
        };
        let original = if [index, tree].iter().any(|letter| matches!(letter, 'R' | 'C')) {
            Some(records.next().ok_or_else(|| unexpected(WHAT, record))?)
        } else {
            None
        };
        let letters = Letters { index, tree };
        add_record(&mut working_tree, letters, path, original)?;
    }
    Ok(working_tree)
}

/// The two letters of a record.
#[derive(Clone, Copy)]
struct Letters {
    /// What the index did, a space for nothing.
    index: char,
    /// What the working tree did, a space for nothing.
    tree: char,
}

/// Adds a record to the lists it belongs in.
fn add_record(
    working_tree: &mut WorkingTree,
    letters: Letters,
    path: &str,
    original: Option<&str>,
) -> Result<(), GitError> {
    if let Some(kind) = conflict_kind(&format!("{}{}", letters.index, letters.tree)) {
        working_tree.conflicted.push(ConflictedFile {
            path: path.to_owned(),
            kind,
        });
        return Ok(());
    }
    match (letters.index, letters.tree) {
        ('?', '?') => working_tree.untracked.push(path.to_owned()),
        ('!', '!') => {}
        (index, tree) => {
            let file = |kind: ChangeKind| FileChange {
                path: path.to_owned(),
                original_path: matches!(kind, ChangeKind::Renamed | ChangeKind::Copied)
                    .then(|| original.map(str::to_owned))
                    .flatten(),
                kind,
            };
            if let Some(kind) = change_kind(blank_as_dot(index))? {
                working_tree.staged.push(file(kind));
            }
            if let Some(kind) = change_kind(blank_as_dot(tree))? {
                working_tree.unstaged.push(file(kind));
            }
        }
    }
    Ok(())
}

/// The letter of a side that did nothing is a space in `--porcelain=v1` and a dot in `--porcelain=v2`.
fn blank_as_dot(letter: char) -> char {
    if letter == ' ' { '.' } else { letter }
}

/// What the letter of a side says of a file, none for `.`: that side did not change it.
pub(crate) fn change_kind(letter: char) -> Result<Option<ChangeKind>, GitError> {
    Ok(Some(match letter {
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

/// What each side did to a file in conflict, from the two letters of `git status`; none when they are not a conflict.
pub(crate) fn conflict_kind(letters: &str) -> Option<ConflictKind> {
    Some(match letters {
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
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::state::{ChangeKind, ConflictKind, FileChange};

    use super::parse_working_tree;

    #[test]
    fn changes_read_working_tree__a_rename_is_one_staged_file_with_its_old_path() {
        let read = parse_working_tree(b"R  new.txt\0old.txt\0").unwrap();
        let renamed = FileChange {
            path: String::from("new.txt"),
            original_path: Some(String::from("old.txt")),
            kind: ChangeKind::Renamed,
        };
        assert_eq!(read.staged, [renamed]);
        assert!(read.unstaged.is_empty() && read.untracked.is_empty() && read.conflicted.is_empty());
    }

    #[test]
    fn changes_read_working_tree__a_file_changed_in_both_is_in_both_lists() {
        let read = parse_working_tree(b"MM a.txt\0").unwrap();
        let changed = FileChange {
            path: String::from("a.txt"),
            original_path: None,
            kind: ChangeKind::Modified,
        };
        assert_eq!(read.staged, std::slice::from_ref(&changed));
        assert_eq!(read.unstaged, [changed]);
    }

    #[test]
    fn changes_read_working_tree__conflicts_are_only_conflicts() {
        let read = parse_working_tree(b"UU a.txt\0AA b.txt\0").unwrap();
        let kinds: Vec<(&str, ConflictKind)> = read
            .conflicted
            .iter()
            .map(|file| (file.path.as_str(), file.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("a.txt", ConflictKind::BothModified),
                ("b.txt", ConflictKind::BothAdded)
            ]
        );
        assert!(read.staged.is_empty() && read.unstaged.is_empty());
    }

    #[test]
    fn changes_read_working_tree__an_untracked_file_is_not_staged_as_untracked() {
        let read = parse_working_tree(b"?? dir/new.txt\0").unwrap();
        assert_eq!(read.untracked, ["dir/new.txt"]);
        assert!(read.staged.is_empty() && read.unstaged.is_empty());
    }

    #[test]
    fn changes_read_working_tree__refuses_a_record_it_does_not_understand() {
        assert!(parse_working_tree(b"X  a.txt\0").is_err());
        assert!(parse_working_tree(b"M\0").is_err());
        assert!(parse_working_tree(b"R  new.txt\0").is_err());
        assert_eq!(
            parse_working_tree(b"!! target/\0").unwrap(),
            model::state::WorkingTree::default()
        );
    }
}
