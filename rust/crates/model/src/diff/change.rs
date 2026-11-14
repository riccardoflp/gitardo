//! A change of a Diff page as the buttons of its gutter give it to the operations that stage, unstage or revert it
//! (`diff.stage-change`, `diff.unstage-change`, `diff.revert-change`), and the versions of its file those read again.

use std::ops::Range;

use crate::CommitId;

/// A version of a file a side of a Diff page shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiffSide {
    /// What is checked out.
    Head,
    /// The index.
    Index,
    /// The working tree.
    WorkingTree,
    /// A commit.
    Commit(CommitId),
    /// No version: every line of the other side is added, or removed.
    Empty,
}

/// A change of a Diff page: its file, the two versions it is between, its lines on each side counted from 0 (an empty
/// range is the place meant, as [`super::LineChange`] has it), and the text of those lines as the page showed them, so
/// that an operation can tell whether the file changed since.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffChange {
    /// The path of the file from the top of the working tree, with `/`.
    pub path: String,
    /// The version on the left.
    pub left: DiffSide,
    /// The version on the right.
    pub right: DiffSide,
    /// Its lines on the left.
    pub left_lines: Range<usize>,
    /// Its lines on the right.
    pub right_lines: Range<usize>,
    /// The text of its lines on the left, without their line breaks.
    pub left_text: Vec<String>,
    /// The text of its lines on the right.
    pub right_text: Vec<String>,
}

/// The versions of a file an operation on a change reads again: a commit's (`HEAD`'s, or the one the page compares
/// with), the index's and the working tree's, each as bytes, none where the file is not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileVersions {
    /// In the commit asked for; none when none was asked for, or the commit has no such file.
    pub commit: Option<Vec<u8>>,
    /// In the index; none for a file git does not track.
    pub index: Option<Vec<u8>>,
    /// On disk; none for a file deleted.
    pub working_tree: Option<Vec<u8>>,
}
