//! The words of the branch rail (`refs.tree`, `refs.filter`, `refs.scope-line`, `refs.sort`, `refs.recent`,
//! `refs.favorite`).

/// The rail, for the screen reader.
pub const RAIL: &str = "Branches";
/// The button at the top of the rail that makes a branch at `HEAD` (`branch.pane`).
pub const CREATE_BRANCH: &str = "Create branch";
/// The button at the top of the rail that chooses what to compare (`branch.pane`).
pub const COMPARE: &str = "Compare";
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
/// **Push tags…** in the **…** of the Tags list (`tag.push-all`).
pub const PUSH_TAGS: &str = "Push tags\u{2026}";
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
/// The tooltip of a reference the timeline leaves out, dimmed in the rail.
pub const NOT_SHOWN: &str = "Not shown in the timeline";
/// **Show all** on the line at the top of the rail.
pub const SHOW_ALL: &str = "Show all";
/// The list of the filter of the rail, for the screen reader (`refs.list-filter`).
pub const LIST_FILTER: &str = "Branches listed";
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

/// The entry of the menu of a remote that fetches its notes (`note.fetch`).
pub const FETCH_NOTES: &str = "Fetch notes";

/// The entry of the menu of a remote that pushes the notes to it (`note.push`).
pub const PUSH_NOTES: &str = "Push notes";
/// **Rename…** in the menu of a local branch.
pub const RENAME: &str = "Rename\u{2026}";
/// The entry that makes a local branch with no upstream follow a remote branch (`branch.set-upstream`).
pub const SET_UPSTREAM: &str = "Set upstream\u{2026}";
/// The entry that makes a local branch follow another remote branch than it does.
pub const CHANGE_UPSTREAM: &str = "Change upstream\u{2026}";

/// The entry that makes a local branch follow nothing (`branch.unset-upstream`): **Stop tracking origin/feature**.
#[must_use]
pub fn stop_tracking(upstream: &str) -> String {
    format!("Stop tracking {upstream}")
}
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

/// **Check tags on `<remote>`** in the **…** of the Tags list (`tag.remote-marks`).
#[must_use]
pub fn check_tags_on(remote: &str) -> String {
    format!("Check tags on {remote}")
}

/// The tooltip of the mark of a tag a remote does not have: `Not on origin`.
#[must_use]
pub fn not_on(remote: &str) -> String {
    format!("Not on {remote}")
}

/// The tooltip of the mark of a tag a remote has under another object.
#[must_use]
pub fn other_tag_on(remote: &str) -> String {
    format!("{remote} has another tag of this name")
}

/// What follows the count of the Tags list while some tags are not on the remote checked: `· 2 not on origin`.
#[must_use]
pub fn not_on_count(count: usize, remote: &str) -> String {
    format!("\u{b7} {count} not on {remote}")
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

/// The tooltip of the button of the filter of the rail: `Branches listed in the branch rail: Local branches`.
#[must_use]
pub fn list_filter_tooltip(what: &str) -> String {
    format!("Branches listed in the branch rail: {what}")
}
