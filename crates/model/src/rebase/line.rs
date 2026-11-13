//! The commits a rewrite makes again: a commit and those after it up to `HEAD`, as read before the rewrite.

use crate::CommitId;

/// A commit of the line, with its message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineCommit {
    /// The commit, in full.
    pub id: CommitId,
    /// Its whole message.
    pub message: String,
}

/// What is read of a commit before it is rewritten: its parents, and the line from it up to `HEAD`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LineToHead {
    /// The parents of the commit: none for the first commit, two or more for a merge.
    pub parents: Vec<CommitId>,
    /// The commit and the commits after it up to `HEAD`, oldest first, each the first parent of the next; none when
    /// the commit is not an ancestor of `HEAD`, or when it or a commit after it is a merge.
    pub commits: Option<Vec<LineCommit>>,
}

impl LineToHead {
    /// The commit the rebase starts after: the parent of the commit, none for the first commit.
    #[must_use]
    pub fn base(&self) -> Option<&CommitId> {
        self.parents.first()
    }
}

/// The subject of a message: its first line, trimmed.
#[must_use]
pub fn subject_of(message: &str) -> &str {
    message.trim().lines().next().unwrap_or_default().trim()
}

/// How the note of a rewrite counts the commits after those it changes, which are made again on top:
/// `the commit after it is` or `the 3 commits after it are`.
#[must_use]
pub fn commits_after(count: usize, after_what: &str) -> String {
    if count == 1 {
        format!("the commit after {after_what} is")
    } else {
        format!("the {count} commits after {after_what} are")
    }
}
