//! The words of the branch rail (`refs.tree`).

/// The rail, for the screen reader.
pub const RAIL: &str = "Branches";
/// The title of the local branches.
pub const LOCAL: &str = "LOCAL";
/// The title of what is kept aside.
pub const KEPT_ASIDE: &str = "KEPT ASIDE";
/// The list of the stashes.
pub const STASHES: &str = "Stashes";
/// The list of the worktrees.
pub const WORKTREES: &str = "Worktrees";
/// The list of the tags.
pub const TAGS: &str = "Tags";
/// A local branch whose upstream was deleted.
pub const GONE: &str = "gone";
/// A worktree with nothing checked out on a branch.
pub const DETACHED: &str = "detached";
/// A worktree whose folder is gone.
pub const MISSING: &str = "missing";

/// The title of the branches of a remote: its name, as **ORIGIN**.
#[must_use]
pub fn remote_title(remote: &str) -> String {
    remote.to_uppercase()
}
