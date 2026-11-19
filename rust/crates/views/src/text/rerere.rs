//! The words of git's rerere, which remembers how conflicts are resolved (`rerere.turn-on`, `rerere.turn-off`,
//! `rerere.resolved-as-before`, `rerere.forget`, `review.recorded`).

/// The entry of the menu of the row of a repository while git does not remember resolutions.
pub const REMEMBER: &str = "Remember how conflicts are resolved\u{2026}";
/// The same entry, ticked, while it does.
pub const REMEMBERING: &str = "Remember how conflicts are resolved";
/// The command of Quick search that turns it on.
pub const TURN_ON_COMMAND: &str = "Remember How Conflicts Are Resolved\u{2026}";
/// The command of Quick search that turns it off.
pub const TURN_OFF_COMMAND: &str = "Stop Remembering How Conflicts Are Resolved";
/// The small text of the row of a file git resolved as before.
pub const RESOLVED_AS_BEFORE: &str = "Resolved as before";
/// The entry of its menu that shows what git wrote.
pub const SHOW_WHAT_GIT_WROTE: &str = "Show what git wrote";
/// The entry of its menu that has git forget the resolution.
pub const FORGET: &str = "Forget this resolution\u{2026}";
/// The name of the column of the Diff page with the file as git marked it.
pub const AS_GIT_MARKED_IT: &str = "As git marked it";

/// What the banner adds after the count of the files in conflict: ** · git resolved 2 as before**.
#[must_use]
pub fn banner(count: usize) -> String {
    format!(" \u{b7} git resolved {count} as before")
}

/// What the message of an operation that stopped adds: *Git resolved 2 files as it did before: look at them before you
/// continue.*
#[must_use]
pub fn stopped(count: usize) -> String {
    let files = if count == 1 {
        "1 file"
    } else {
        &format!("{count} files")
    };
    format!("Git resolved {files} as it did before: look at them before you continue.")
}

/// The heading of the card of a file git resolved as before, in the review.
pub const RESOLVED_BY_GIT: &str = "Resolved by git as before";
/// What the card says under it.
pub const RECORDED_NOTE: &str =
    "Git recorded how this conflict was resolved the last time, and wrote the same resolution.";
/// The tooltip of **Accept** on it.
pub const ACCEPT_TOOLTIP: &str = "Stage the file as git wrote it when the accepted resolutions are written";
/// The button of the card that has git forget the resolution.
pub const FORGET_IT: &str = "Forget it\u{2026}";
/// Its tooltip.
pub const FORGET_IT_TOOLTIP: &str = "Forget the resolution git recorded, and mark the conflicts of the file again";
/// What git wrote is the file as it was marked: nothing changed.
pub const WROTE_NOTHING: &str = "Git wrote nothing in place of the markers.";

/// What the head of the review adds: *; 1 file resolved by git as before*.
#[must_use]
pub fn summary(count: usize) -> String {
    format!("; {} resolved by git as before", super::review::counted(count, "file"))
}
