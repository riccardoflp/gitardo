//! The words of the branch rail (`refs.tree`, `refs.filter`, `refs.scope-line`, `refs.sort`, `refs.recent`,
//! `refs.favorite`).

/// The rail, for the screen reader.
pub const RAIL: &str = "Branches";
/// The title of the favorites.
pub const FAVORITES: &str = "FAVORITES";
/// The title of the branches checked out last.
pub const RECENT: &str = "RECENT";
/// The title of the local branches.
pub const LOCAL: &str = "LOCAL";
/// The title of the one list of the matches while something is typed in the filter box.
pub const MATCHES: &str = "MATCHES";
/// The title of what is kept aside.
pub const KEPT_ASIDE: &str = "KEPT ASIDE";
/// The list of the stashes.
pub const STASHES: &str = "Stashes";
/// The list of the worktrees.
pub const WORKTREES: &str = "Worktrees";
/// The list of the tags.
pub const TAGS: &str = "Tags";
/// The list of the submodules.
pub const SUBMODULES: &str = "Submodules";
/// A local branch whose upstream was deleted.
pub const GONE: &str = "gone";
/// A worktree with nothing checked out on a branch.
pub const DETACHED: &str = "detached";
/// A worktree whose folder is gone.
pub const MISSING: &str = "missing";
/// What the filter box at the top of the rail says while it is empty.
pub const FILTER: &str = "Filter branches and tags";
/// The button at the end of the filter box, which empties it.
pub const EMPTY_FILTER: &str = "Empty the filter";
/// The tooltip of a reference the timeline leaves out, while the rail lists them all.
pub const NOT_SHOWN: &str = "Not shown in the timeline";
/// The eye at the end of such a reference, which adds its history to the timeline.
pub const SHOW_TOO: &str = "Show this history too";
/// The button of the line at the top of the rail while it lists only the branches the timeline shows.
pub const LIST_ALL: &str = "List every branch, the others dimmed";
/// The same button while the rail lists every branch.
pub const LIST_SHOWN: &str = "List only the branches the timeline shows";
/// **Show all** on the line at the top of the rail.
pub const SHOW_ALL: &str = "Show all";
/// The **…** on the title of a list, for the screen reader and its tooltip.
pub const LIST_MENU: &str = "More about this list";
/// **Add to favorites** in the menu of a branch.
pub const ADD_TO_FAVORITES: &str = "Add to favorites";
/// **Remove from favorites** in the menu of a branch.
pub const REMOVE_FROM_FAVORITES: &str = "Remove from favorites";
/// **Sort by name** in the menu of a list.
pub const SORT_BY_NAME: &str = "Sort by name";
/// **Sort by date** in the menu of a list.
pub const SORT_BY_DATE: &str = "Sort by date";
/// **Create worktree…** on the title of the Worktrees list.
pub const CREATE_WORKTREE: &str = "Create worktree\u{2026}";
/// **Add submodule…** on the title of the Submodules list.
pub const ADD_SUBMODULE: &str = "Add submodule\u{2026}";
/// **Rename…** in the menu of a local branch.
pub const RENAME: &str = "Rename\u{2026}";
/// **Delete** in the menu of a local branch or a tag.
pub const DELETE: &str = "Delete";
/// **Delete from remote** in the menu of a remote branch (`remote.delete-branch`).
pub const DELETE_FROM_REMOTE: &str = "Delete from remote";
/// **Check out as local branch** in the menu of a remote branch.
pub const CHECK_OUT_AS_LOCAL: &str = "Check out as local branch";
/// **Show in the timeline** in the menu of a reference.
pub const SHOW_IN_TIMELINE: &str = "Show in the timeline";

/// The title of the branches of a remote: its name, as **ORIGIN**.
#[must_use]
pub fn remote_title(remote: &str) -> String {
    remote.to_uppercase()
}

/// **Show all `<n>`…** under the first 100 entries of a list.
#[must_use]
pub fn show_all_of(count: usize) -> String {
    format!("Show all {count}\u{2026}")
}

/// The count of a list while something is typed in the filter box: `3 of 40`.
#[must_use]
pub fn matches_of(matches: usize, all: usize) -> String {
    format!("{matches} of {all}")
}

/// **Check out `<branch>`** in the menu of a local branch.
#[must_use]
pub fn check_out(branch: &str) -> String {
    format!("Check out {branch}")
}

/// The tooltip of a local branch, about its upstream: `main → origin/main`, or `main (no upstream)`.
#[must_use]
pub fn upstream_of(branch: &str, upstream: Option<&str>) -> String {
    match upstream {
        Some(upstream) => format!("{branch} \u{2192} {upstream}"),
        None => format!("{branch} (no upstream)"),
    }
}

/// The line of the tooltip of a branch another worktree has: `In the worktree app-feature, 2 files not committed`.
#[must_use]
pub fn in_worktree(folder: &str, uncommitted: Option<usize>) -> String {
    match uncommitted {
        Some(0) | None => format!("In the worktree {folder}"),
        Some(1) => format!("In the worktree {folder}, 1 file not committed"),
        Some(files) => format!("In the worktree {folder}, {files} files not committed"),
    }
}

/// The tooltip of a stash: `stash@{0}: On main: before the merge`.
#[must_use]
pub fn stash_tooltip(index: usize, message: &str) -> String {
    format!("stash@{{{index}}}: {message}")
}
