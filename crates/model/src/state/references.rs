//! The references of a repository, its stashes and its worktrees, as read.

use std::path::PathBuf;

use crate::{CommitId, Timestamp};

/// A local branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// Its name without `refs/heads/`: `feature/login`.
    pub name: String,
    /// The commit it is at.
    pub commit: CommitId,
    /// The branch it follows on a remote, if it has one.
    pub upstream: Option<Upstream>,
    /// Whether it is the branch checked out in this worktree.
    pub is_checked_out: bool,
    /// The date of its commit.
    pub date: Timestamp,
}

/// The branch a local branch follows on a remote, and how far apart they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upstream {
    /// Its full name: `refs/remotes/origin/main`.
    pub reference: String,
    /// The remote: `origin`.
    pub remote: String,
    /// The branch on the remote, in full: `refs/heads/main`.
    pub remote_branch: String,
    /// How many commits the branch has that its upstream has not.
    pub ahead: u32,
    /// How many commits the upstream has that the branch has not.
    pub behind: u32,
    /// Whether the upstream was deleted on the remote and pruned here.
    pub is_gone: bool,
}

/// A remote branch: a remote-tracking branch of git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBranch {
    /// Its name without `refs/remotes/`: `origin/main`.
    pub name: String,
    /// The remote: `origin`.
    pub remote: String,
    /// The commit it is at.
    pub commit: CommitId,
    /// The date of its commit.
    pub date: Timestamp,
}

/// A tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// Its name without `refs/tags/`: `v1.0.0`.
    pub name: String,
    /// The commit it points at; an annotated tag, the commit it peels to.
    pub commit: CommitId,
    /// The date of its commit, or the date an annotated tag was made.
    pub date: Timestamp,
}

/// A stash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stash {
    /// The commit git keeps it as.
    pub commit: CommitId,
    /// Its place in the list: 0 for `stash@{0}`, the newest.
    pub index: usize,
    /// Its message: `On main: before the merge`.
    pub message: String,
    /// When it was made.
    pub date: Timestamp,
}

/// A worktree of the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    /// Its folder.
    pub folder: PathBuf,
    /// The commit checked out there, none before the first commit.
    pub commit: Option<CommitId>,
    /// The branch checked out there, by its full name, none when detached.
    pub branch: Option<String>,
    /// Whether it is the main worktree, the one the repository was made in.
    pub is_main: bool,
    /// Whether it is the worktree that was read.
    pub is_current: bool,
    /// Whether its folder is gone: git would prune it.
    pub is_missing: bool,
    /// How many files are not committed there; none when it is missing.
    pub uncommitted: Option<usize>,
}
