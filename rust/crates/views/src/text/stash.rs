//! The words of the Stashes list of the branch rail (`stash.pane`) and of the warning a pull leaves with its autostash
//! (`stash.autostash-left`).

/// The button on the title of the list, which stashes all changes (`stash.push`).
pub const STASH_ALL: &str = "Stash all changes\u{2026}";
/// What the list says while there is no stash.
pub const NO_STASHES: &str = "No stashes. Put your changes aside with the button above, or from the Changes page: one \
                              file, what is staged, or everything.";
/// The button on a stash that brings its changes back and keeps it (`stash.apply`).
pub const APPLY: &str = "Apply: bring the changes back and keep the stash";
/// The button on a stash that brings its changes back and drops it (`stash.pop`).
pub const POP: &str = "Pop: bring the changes back and drop the stash";
/// The button on a stash that drops it, once asked (`stash.drop`).
pub const DROP: &str = "Drop\u{2026}";
/// The arrow of a stash folded, which lists its files.
pub const UNFOLD: &str = "Show the files of the stash";
/// The arrow of a stash unfolded, which hides its files.
pub const FOLD: &str = "Hide the files of the stash";
/// Under a stash unfolded while its files are read.
pub const LOADING: &str = "Loading\u{2026}";
/// Under a stash unfolded that has no file.
pub const NO_FILES: &str = "No files";
/// Under a stash unfolded whose files git could not read.
pub const NOT_READ: &str = "Git could not read this stash.";
/// The tooltip of an untracked file a stash took.
pub const UNTRACKED: &str = "Untracked: it was not in the commit the stash was made on";
/// **Show Stashes**, beside the warning of a pull that left its autostash.
pub const SHOW_STASHES: &str = "Show Stashes";
