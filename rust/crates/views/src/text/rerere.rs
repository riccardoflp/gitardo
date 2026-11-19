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
