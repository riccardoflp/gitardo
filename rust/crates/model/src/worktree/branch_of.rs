//! `worktree.branch-in-another-worktree`: a local branch that another worktree has checked out, which is worked on
//! there.

use std::path::Path;

use crate::state::{RepositoryState, Worktree};

use super::name::is_listed;

/// A branch, and where the views are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtherWorktreeInput<'a> {
    /// The repository shown, as last read, with its worktrees.
    pub state: &'a RepositoryState,
    /// The branch, by its short name.
    pub branch: &'a str,
    /// The folder of the window, when the views show another worktree; none when they show the window's own.
    pub window_folder: Option<&'a Path>,
}

/// The worktree that has a branch, and what can be done with it from here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorktreeBranch<'a> {
    /// The worktree.
    pub worktree: &'a Worktree,
    /// Whether it can be renamed and removed from here: it is neither the main worktree nor the folder of the window.
    pub can_rename_or_remove: bool,
}

impl WorktreeBranch<'_> {
    /// Whether its folder is gone: a double click does nothing, and a checkout is refused until it is pruned.
    #[must_use]
    pub fn is_missing(&self) -> bool {
        self.worktree.is_missing
    }
}

/// The worktree other than the one the views show that has a local branch checked out, if any: the branch has the
/// worktree's icon and its menu opens the worktree rather than checking the branch out, which git refuses. Seen from
/// another worktree, the main worktree's branch is the same, but the main worktree can be neither renamed nor removed,
/// and neither can the folder of the window.
///
/// Feature: worktree.branch-in-another-worktree
#[must_use]
pub fn branch_in_another_worktree<'a>(input: &OtherWorktreeInput<'a>) -> Option<WorktreeBranch<'a>> {
    let worktree = crate::branch::other_worktree(input.state, input.branch)?;
    let is_window = input
        .window_folder
        .is_some_and(|window| is_listed(&[window.to_path_buf()], &worktree.folder));
    Some(WorktreeBranch {
        worktree,
        can_rename_or_remove: !worktree.is_main && !is_window,
    })
}
