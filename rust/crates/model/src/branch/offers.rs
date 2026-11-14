//! When the operations on a branch are offered: the conditions their cards give, read by the operations, which refuse
//! when they no longer hold, and by the menus, which leave them out.

use crate::state::{Branch, RepositoryState};

use super::other_worktree;

/// The local branch that follows a remote branch and can be checked out and rebased onto it
/// (`branch.checkout-remote-and-rebase`): its upstream is not gone, each has commits the other has not as of the last
/// fetch, it is not checked out here and no other worktree has it.
#[must_use]
pub fn rebase_onto<'a>(state: &'a RepositoryState, remote_branch: &str) -> Option<&'a Branch> {
    let reference = format!("refs/remotes/{remote_branch}");
    state.branches.iter().find(|branch| {
        branch.upstream.as_ref().is_some_and(|upstream| {
            upstream.reference == reference && !upstream.is_gone && upstream.ahead > 0 && upstream.behind > 0
        }) && !branch.is_checked_out
            && other_worktree(state, &branch.name).is_none()
    })
}

/// Whether a local branch can be fast-forwarded to its upstream without being checked out (`branch.fast-forward`): it
/// is not checked out here, it has an upstream that is not gone and no commits of its own as of the last fetch, and,
/// when another worktree has it, that worktree's folder is there with no file not committed.
#[must_use]
pub fn can_fast_forward(state: &RepositoryState, branch: &Branch) -> bool {
    let follows = branch
        .upstream
        .as_ref()
        .is_some_and(|upstream| !upstream.is_gone && upstream.ahead == 0);
    let worktree_is_clean = other_worktree(state, &branch.name)
        .is_none_or(|worktree| !worktree.is_missing && worktree.uncommitted == Some(0));
    !branch.is_checked_out && follows && worktree_is_clean
}
