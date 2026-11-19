//! The words of the timeline (`graph.view`) and of the inspector beside it.

/// The timeline, for the screen reader.
pub const TIMELINE: &str = "Timeline";
/// The list of the commits, for the screen reader.
pub const HISTORY: &str = "History";
/// What the timeline says in a repository with no commits.
pub const NO_COMMITS_YET: &str = "No commits yet";
/// What the timeline says when it lists no commits otherwise.
pub const NO_COMMITS_TO_SHOW: &str = "No commits to show";
/// What the timeline says while the first commits are read.
pub const LOADING_COMMITS: &str = "Loading commits\u{2026}";
/// The inspector, for the screen reader.
pub const INSPECTOR: &str = "Selected commit";

/// The second line of a row: who wrote the commit and when.
#[must_use]
pub fn author_and_date(author: &str, date: &str) -> String {
    format!("{author} \u{b7} {date}")
}
/// The last row of the timeline while more commits are to be read (`graph.load-more`).
pub const LOADING_MORE: &str = "Loading more commits\u{2026}";
/// The last row of the timeline of a shallow clone (`graph.shallow-end`).
pub const SHALLOW_END: &str = "The history ends here: this is a shallow clone.";
/// The button that fetches more of the history of a shallow clone (`sync.deepen`).
pub const LOAD_MORE_HISTORY: &str = "Load more history";
/// The button that fetches all of the history of a shallow clone (`sync.unshallow`).
pub const LOAD_WHOLE_HISTORY: &str = "Load the whole history";
/// The tooltip of the dot of a commit whose parents were not fetched (`graph.shallow-end`).
pub const NOT_FETCHED_BEFORE: &str =
    "The commits before this one were not fetched: this repository is a shallow clone.";
/// The draft, for the screen reader and at the top of the inspector (`graph.uncommitted-row`).
pub const UNCOMMITTED_CHANGES: &str = "Uncommitted changes";
/// What the message of the draft says while it is empty.
pub const DESCRIBE_NEXT_COMMIT: &str = "Describe the next commit";
/// The button of the draft that opens the Changes page.
pub const REVIEW: &str = "Review";
/// The button of the draft that commits.
pub const COMMIT: &str = "Commit";
/// The tooltip of the buttons of the draft, until committing comes (milestone M4).
pub const NOT_YET: &str = "Committing from the timeline comes with the Changes page";
/// **Stash changes…** in the menu of the draft.
pub const STASH_CHANGES: &str = "Stash changes\u{2026}";
/// **Stash staged changes…** in the menu of the draft.
pub const STASH_STAGED: &str = "Stash staged changes\u{2026}";
/// **Stage all changes** in the menu of the draft.
pub const STAGE_ALL: &str = "Stage all changes";
/// **Unstage all changes** in the menu of the draft.
pub const UNSTAGE_ALL: &str = "Unstage all changes";
/// **Discard all changes…** in the menu of the draft.
pub const DISCARD_ALL: &str = "Discard all changes\u{2026}";
/// **Apply** in the menu of a stash.
pub const APPLY: &str = "Apply";
/// **Pop** in the menu of a stash.
pub const POP: &str = "Pop";
/// **Create branch from stash…** in the menu of a stash.
pub const CREATE_BRANCH_FROM_STASH: &str = "Create branch from stash\u{2026}";
/// **Drop** in the menu of a stash.
pub const DROP: &str = "Drop";
/// The entry and the arrow that fold a merge (`graph.fold-merge`).
pub const FOLD_MERGE: &str = "Fold the commits it merged into one row";
/// The entry and the arrow that unfold a merge.
pub const UNFOLD_MERGE: &str = "Show the commits it merged again";
/// The tooltip of a folded row.
pub const CLICK_TO_SHOW: &str = "Click to show them again";
/// The edge between the graph and the messages, for the screen reader (`graph.resize-graph`).
pub const GRAPH_EDGE: &str = "Width of the graph";
/// The line between the branch rail and the commits (`graph.resize-panes`).
pub const RAIL_LINE: &str = "Width of the branch rail";
/// The line between the commits and the inspector.
pub const INSPECTOR_LINE: &str = "Size of the inspector";
/// The marks beside the scroll bar, for the screen reader (`graph.scroll-marks`).
pub const SCROLL_MARKS: &str = "Marks of the commit checked out and the one selected";

/// What the draft says it changes: `1 file changed`, `3 files changed · 2 staged`.
#[must_use]
pub fn files_changed(files: usize, staged: usize) -> String {
    let changed = if files == 1 {
        String::from("1 file changed")
    } else {
        format!("{files} files changed")
    };
    if staged == 0 {
        changed
    } else {
        format!("{changed} \u{b7} {staged} staged")
    }
}

/// The tooltip of a sticky branch name (`graph.sticky-branch-names`).
#[must_use]
pub fn go_up_to(name: &str) -> String {
    format!("Go up to {name}")
}

/// The labels of a commit, on its row and in the list of all of them (`graph.label-overflow`), for the screen reader.
pub const LABELS: &str = "Labels";

/// The tooltip of the ring of a commit not pushed (`graph.unpushed`).
#[must_use]
pub fn not_pushed_to(upstream: &str) -> String {
    format!("Not pushed to {upstream}")
}

/// The tooltip of a label of a branch checked out in another worktree.
#[must_use]
pub fn checked_out_in(folder: &str) -> String {
    format!("Checked out in {folder}")
}

/// **Copy `<name>`** in the menu of a label a branch shares with its upstream: the upstream's name on the clipboard.
#[must_use]
pub fn copy_name(name: &str) -> String {
    format!("Copy {name}")
}
