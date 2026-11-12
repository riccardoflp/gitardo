//! The words of the operations on branches, tags and commits in the menus of the branch rail and of the timeline
//! (`branch.`, `tag.`).

/// **Open in worktree** in the menu of a branch (`worktree.open-in-worktree`).
pub const OPEN_IN_WORKTREE: &str = "Open in worktree";
/// **Create branch here…** in the menu of a commit, a branch or a tag (`branch.create`).
pub const CREATE_BRANCH_HERE: &str = "Create branch here\u{2026}";
/// **Rename branch…** in the menu of a branch another worktree has checked out (`branch.rename`).
pub const RENAME_BRANCH: &str = "Rename branch\u{2026}";
/// **Open worktree in new window** in the menu of a branch another worktree has checked out.
pub const OPEN_WORKTREE_IN_NEW_WINDOW: &str = "Open worktree in new window";
/// **Remove worktree…** in the menu of a branch another worktree has checked out.
pub const REMOVE_WORKTREE: &str = "Remove worktree\u{2026}";
/// **Remove worktree and delete branch…** in the menu of a branch another worktree has checked out.
pub const REMOVE_WORKTREE_AND_DELETE_BRANCH: &str = "Remove worktree and delete branch\u{2026}";
/// **Check out tag (detached)** in the menu of a tag (`tag.checkout`).
pub const CHECK_OUT_TAG: &str = "Check out tag (detached)";
/// **Push tag** in the menu of a tag (`tag.push`).
pub const PUSH_TAG: &str = "Push tag";
/// **Create tag here…** in the menu of a commit (`tag.create`).
pub const CREATE_TAG_HERE: &str = "Create tag here\u{2026}";
/// **Check out commit (detached)** in the menu of a commit (`branch.checkout-detached`).
pub const CHECK_OUT_COMMIT: &str = "Check out commit (detached)";
/// **Clean up branches…** in the **…** menu of the header (`branch.clean-up`).
pub const CLEAN_UP_BRANCHES: &str = "Clean up branches\u{2026}";

/// **Fast-forward to `<upstream>`** in the menu of a branch behind its upstream (`branch.fast-forward`).
#[must_use]
pub fn fast_forward_to(upstream: &str) -> String {
    format!("Fast-forward to {upstream}")
}

/// **Check out `<name>`** in the menu of a remote branch whose name a local branch has (`branch.checkout-remote`).
#[must_use]
pub fn check_out_local(name: &str) -> String {
    format!("Check out {name}")
}

/// **Check out `<name>` and rebase it onto this** in the menu of a remote branch
/// (`branch.checkout-remote-and-rebase`).
#[must_use]
pub fn check_out_and_rebase(name: &str) -> String {
    format!("Check out {name} and rebase it onto this")
}
