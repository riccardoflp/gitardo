//! A file as a list shows it, which `details.compare-file-with` compares with a branch, a tag, a commit or the working
//! tree: the side it is taken from, and the two versions the Diff page then compares.

use super::sides::CompareSide;
use super::{NamedCommit, Side};
use crate::CommitId;
use crate::state::{ChangeKind, FileChange};

/// A file of a list, as a menu is opened on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedFile {
    /// The file, as the list has it.
    pub file: FileChange,
    /// The list it is in.
    pub list: FileList,
}

/// The lists a file can be compared from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileList {
    /// The files of a commit, in the inspector or in its page.
    Commit {
        /// The commit, with the name the list gives it.
        commit: NamedCommit,
        /// Its first parent; none for a root commit.
        first_parent: Option<CommitId>,
    },
    /// The files of a comparison, from its first side to its second.
    Comparison {
        /// The side it starts from.
        first: CompareSide,
        /// The side it goes to.
        second: CompareSide,
    },
    /// The uncommitted changes.
    Changes,
}

/// What is checked out, which names the side of a file the uncommitted changes deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedOut<'a> {
    /// The commit checked out; none before the first commit.
    pub head: Option<&'a CommitId>,
    /// The branch checked out; none when `HEAD` is detached.
    pub branch: Option<&'a str>,
}

/// The two versions of a file the Diff page compares, from a side to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSides {
    /// The side on the left.
    pub left: CompareSide,
    /// The side on the right.
    pub right: CompareSide,
    /// The file, by its path from the top of the repository.
    pub path: String,
}

impl ListedFile {
    /// The side the file is taken from: the commit, or its first parent when it deleted the file; the second side of a
    /// comparison, or the side it starts from when the second deleted it; the working tree, or `HEAD` named by the
    /// branch checked out for a file deleted there. None for a file deleted by a root commit, or before the first
    /// commit, which has no version to compare.
    #[must_use]
    pub fn own_side(&self, checked_out: CheckedOut<'_>) -> Option<CompareSide> {
        let is_deleted = self.file.kind == ChangeKind::Deleted;
        match &self.list {
            FileList::Commit { first_parent, .. } if is_deleted => first_parent.as_ref().map(|parent| CompareSide {
                name: parent.short().to_owned(),
                side: Side::Commit(parent.clone()),
                is_followed: false,
                is_gone: false,
            }),
            FileList::Commit { commit, .. } => Some(CompareSide {
                name: commit.name.clone(),
                side: Side::Commit(commit.commit.clone()),
                is_followed: false,
                is_gone: false,
            }),
            FileList::Comparison { first, .. } if is_deleted => Some(first.clone()),
            FileList::Comparison { second, .. } => Some(second.clone()),
            FileList::Changes if is_deleted => checked_out.head.map(|head| CompareSide {
                name: checked_out.branch.unwrap_or("HEAD").to_owned(),
                side: Side::Commit(head.clone()),
                is_followed: true,
                is_gone: false,
            }),
            FileList::Changes => Some(CompareSide::working_tree()),
        }
    }

    /// The name of the file, the last part of its path.
    #[must_use]
    pub fn name(&self) -> &str {
        self.file.path.rsplit('/').next().unwrap_or(&self.file.path)
    }

    /// The versions the Diff page compares once the other side is chosen: the working tree always on the right, where
    /// its file can be edited; otherwise the side chosen on the left and the file's own on the right.
    #[must_use]
    pub fn sides_with(&self, own: CompareSide, chosen: CompareSide) -> FileSides {
        let (left, right) = if own.is_working_tree() || !chosen.is_working_tree() {
            (chosen, own)
        } else {
            (own, chosen)
        };
        FileSides {
            left,
            right,
            path: self.file.path.clone(),
        }
    }
}
