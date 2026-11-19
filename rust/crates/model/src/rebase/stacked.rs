//! The other local branches on the commits a rebase makes again, the branches of a stack, and the version of git that
//! decides whether git can move them with the rebase (`rebase.read-stacked-branches`, `repo.read-git-version`).

use std::path::PathBuf;

use crate::CommitId;

/// The version of git that runs the operations, as `git --version` said it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitVersion {
    /// The first number: `2` of `2.53.0`.
    pub major: u32,
    /// The second number: `53` of `2.53.0`.
    pub minor: u32,
    /// The third number: `0` of `2.53.0`.
    pub patch: u32,
    /// What git said after `git version `, kept as said and never compared: `2.53.0.windows.1`.
    pub said: String,
}

impl GitVersion {
    /// Whether it is this version or a newer one.
    #[must_use]
    pub fn is_at_least(&self, (major, minor): (u32, u32)) -> bool {
        (self.major, self.minor) >= (major, minor)
    }
}

/// A version of git a feature needs, with why (rule R9.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeededVersion {
    /// The major and the minor numbers.
    pub version: (u32, u32),
    /// What it brought that the feature needs.
    pub reason: &'static str,
}

/// `--update-refs` and the `update-ref` lines of a todo list came with git 2.38.
pub const UPDATE_REFS_VERSION: NeededVersion = NeededVersion {
    version: (2, 38),
    reason: "git moves the other branches of a stack with a rebase from 2.38 on",
};

/// A local branch whose tip is one of the commits a rebase makes again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackedBranch {
    /// Its name, without `refs/heads/`.
    pub name: String,
    /// The commit it is on.
    pub commit: CommitId,
    /// The folder of the worktree that has it checked out, if one does.
    pub worktree: Option<PathBuf>,
}

/// The branches of a stack over the commits a rebase makes again, and git's own `rebase.updateRefs`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StackedBranches {
    /// Each branch, other than the one rewritten, whose tip is among the commits made again, in the order of
    /// `git for-each-ref`.
    pub branches: Vec<StackedBranch>,
    /// `rebase.updateRefs` as git has it: `None` when it is not set.
    pub update_refs: Option<bool>,
}
