//! The words of the operations run from the window: their questions, the commands shown before they run
//! (`preview.confirm`) and what is said once they ended.

/// The button that closes a question or a plan without running anything.
pub const CANCEL: &str = "Cancel";
/// The button that runs the plan shown (`preview.confirm`).
pub const RUN: &str = "Run";
/// What the plan shown does, over its commands, when it is not destructive.
pub const THESE_COMMANDS_RUN: &str = "These commands will run:";
/// The button of the message that follows an operation, which takes it back (`undo.last`).
pub const UNDO: &str = "Undo";
/// The button that puts away what was said.
pub const DISMISS: &str = "Dismiss";

/// What is said when an operation could not finish: what it was, and what git or the system said.
#[must_use]
pub fn failed(title: &str, said: &str) -> String {
    let said = said.trim();
    if said.is_empty() {
        format!("{title} failed.")
    } else {
        format!("{title} failed: {said}")
    }
}

/// What is said once an operation ran and said nothing of its own.
#[must_use]
pub fn done(title: &str) -> String {
    format!("{title}: done.")
}

/// The reason a plan gives for being shown first, over its commands, as a sentence.
#[must_use]
pub fn warning(reason: &str) -> String {
    let reason = reason.trim_end();
    if reason.ends_with(['.', '!', '?']) {
        format!("{reason} {THESE_COMMANDS_RUN}")
    } else {
        format!("{reason}. {THESE_COMMANDS_RUN}")
    }
}
