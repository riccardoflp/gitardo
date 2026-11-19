//! The words of rebasing and rewriting commits: the entries of the menu of a commit, the banner of a rebase stopped to
//! edit or split a commit, and the interactive rebase page (`rebase.`, `squash.`, `split.`, `fixup.`).

/// **Squash into the commit before it…** in the menu of a commit (`squash.into-previous`).
pub const SQUASH_INTO_PREVIOUS: &str = "Squash into the commit before it\u{2026}";
/// **Split this commit…** in the menu of a commit (`split.commit`).
pub const SPLIT_THIS_COMMIT: &str = "Split this commit\u{2026}";
/// **Add staged changes to this commit (fixup)** in the menu of a commit (`fixup.commit`).
pub const FIXUP_STAGED: &str = "Add staged changes to this commit (fixup)";
/// **Add all changes to this commit (fixup)** in the menu of a commit, when nothing is staged (`fixup.commit`).
pub const FIXUP_ALL: &str = "Add all changes to this commit (fixup)";
/// **Edit this commit (interactive rebase)…** in the menu of the commit checked out (`rebase.todo-page`).
pub const EDIT_THIS_COMMIT: &str = "Edit this commit (interactive rebase)\u{2026}";

/// **Rebase `<current>` onto this** in the menu of a commit, a local branch and a remote branch (`rebase.onto`).
#[must_use]
pub fn rebase_onto_this(current: &str) -> String {
    format!("Rebase {current} onto this")
}

/// **Interactive rebase of N commits…** in the menu of a commit (`rebase.todo-page`).
#[must_use]
pub fn interactive_rebase_of(count: usize) -> String {
    if count == 1 {
        String::from("Interactive rebase of 1 commit\u{2026}")
    } else {
        format!("Interactive rebase of {count} commits\u{2026}")
    }
}

/// What the banner of a rebase stopped to edit a commit says (`rebase.edit-stop`).
#[must_use]
pub fn stopped_to_edit(subject: &str) -> String {
    format!("Stopped at \"{subject}\" to edit it. Amend it or commit more, then Continue.")
}

/// What the banner of a rebase stopped to split a commit says (`split.commit`).
#[must_use]
pub fn splitting(subject: &str) -> String {
    format!("Splitting \"{subject}\": commit its changes in as many commits as you like, then Continue.")
}

/// **Continue** in the banner of a stop.
pub const CONTINUE: &str = "Continue";
/// The tooltip of **Continue**.
pub const CONTINUE_TOOLTIP: &str = "Go on with the rebase: the commits after it are made again on top";
/// The tooltip of **Continue** while files are in conflict.
pub const RESOLVE_FIRST: &str = "Resolve the conflicts and stage the files first";
/// **Abort** in the banner of a stop.
pub const ABORT: &str = "Abort";
/// The tooltip of **Abort** in the banner of a stop (`conflicts.abort-operation`).
pub const ABORT_TOOLTIP: &str = "Take the whole rebase back";

/// The name of the interactive rebase page in the trail.
pub const PAGE: &str = "Interactive rebase";

/// The header of the interactive rebase page: *Interactive rebase of `<branch>`: N commits*.
#[must_use]
pub fn page_header(branch: &str, count: usize) -> String {
    if count == 1 {
        format!("Interactive rebase of {branch}: 1 commit")
    } else {
        format!("Interactive rebase of {branch}: {count} commits")
    }
}

/// Under the header of the page.
pub const NEWEST_FIRST: &str = "Newest first. Drag to reorder.";
/// The label of the box of a reworded commit.
pub const NEW_SUBJECT: &str = "New subject";
/// The tooltip of a branch on a commit of the page that moves with the rebase.
#[must_use]
pub fn moves_with_it(branch: &str) -> String {
    format!("{branch} moves with the rebase")
}

/// The tooltip of a branch on a commit of the page that another worktree has checked out.
#[must_use]
pub fn stays_in_another_worktree(branch: &str) -> String {
    format!("{branch} is checked out in another worktree and stays on the old commit")
}

/// **Move up** on a row.
pub const MOVE_UP: &str = "Move up";
/// **Move down** on a row.
pub const MOVE_DOWN: &str = "Move down";
/// **Cancel** in the footer.
pub const CANCEL: &str = "Cancel";
/// **Start rebase** in the footer.
pub const START_REBASE: &str = "Start rebase";
/// Said in the footer when every row is dropped.
pub const EVERY_COMMIT_DROPPED: &str = "Every commit is dropped. To remove them all, reset the branch instead.";
/// Said in the footer when the oldest commit kept melds into nothing.
pub const NOTHING_TO_MELD_INTO: &str = "The oldest commit that is kept has nothing below it to meld into.";
/// Said in the footer when a reworded commit has an empty box.
pub const ENTER_A_MESSAGE: &str = "Enter a message for each reworded commit.";

/// The names of the actions, as the rows show them, with their help as tooltip.
pub const ACTIONS: [(&str, &str); 6] = [
    ("Pick", "Keep the commit as it is"),
    ("Reword", "Keep the changes, change the message"),
    (
        "Edit",
        "Stop right after this commit, to amend it or commit more, then continue",
    ),
    ("Squash", "Meld into the commit below, keeping both messages"),
    ("Fixup", "Meld into the commit below, discarding this message"),
    ("Drop", "Remove the commit"),
];
