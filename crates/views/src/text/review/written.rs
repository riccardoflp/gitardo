//! The words of a file deleted on one side (`review.deleted-card`) and of the report of what was written
//! (`review.report`).

use model::conflicts::Side;
use model::state::Operation;

use super::{counted, operation_name};

/// What a card of a file deleted on one side says (`review.deleted-card`): *The branch being merged deleted it, and
/// the branch checked out changed it.*
#[must_use]
pub fn deleted_by(deleter: &Side, keeper: &Side) -> String {
    format!(
        "{} deleted it, and {} changed it.",
        deleter.role.words(),
        keeper.role.words().to_lowercase()
    )
}
/// When the side that kept it changed none of its lines.
pub const CHANGED_NOTHING: &str = "This side changed nothing of the lines of the file.";
/// When its changes are too many.
pub const TOO_MANY_CHANGES: &str = "The changes are too many to show here.";
/// Between the changes shown.
pub const LINES_LEFT_OUT: &str = "Lines left out here";
/// **Keep it, with these changes**.
pub const KEEP_IT: &str = "Keep it, with these changes";
/// Its tooltip.
#[must_use]
pub fn keep_tooltip(file: &str, keeper: bool) -> String {
    let side = if keeper { "current" } else { "incoming" };
    format!("Keep {file} as the {side} side has it, and mark it as resolved")
}
/// **Delete it**.
pub const DELETE_IT: &str = "Delete it";
/// Its tooltip.
#[must_use]
pub fn delete_tooltip(file: &str, is_current_deleter: bool) -> String {
    let side = if is_current_deleter { "current" } else { "incoming" };
    format!("Delete {file}, as the {side} side did: the changes above are left out")
}
/// The tooltip of the button that shows the base beside the file.
pub const SHOW_WHAT_IT_WAS: &str = "Show what it was beside what it is now";

/// What the report says first: *Wrote the resolutions of 4 conflicts in 2 files, 1 from the AI.*
#[must_use]
pub fn wrote(conflicts: usize, files: usize, from_ai: usize) -> String {
    let what = if conflicts == 1 {
        String::from("Wrote the resolution of 1 conflict")
    } else {
        format!("Wrote the resolutions of {conflicts} conflicts")
    };
    let ai = match from_ai {
        0 => String::new(),
        count if count == conflicts => String::from(", all from the AI"),
        count => format!(", {count} from the AI"),
    };
    format!("{what} in {}{ai}.", counted(files, "file"))
}
/// *1 file was left with no conflict, and staged.*
#[must_use]
pub fn staged(files: usize) -> String {
    if files == 1 {
        String::from("1 file was left with no conflict, and staged.")
    } else {
        format!("{files} files were left with no conflict, and staged.")
    }
}
/// *No file is left in conflict: the merge can continue.*
#[must_use]
pub fn none_left(operation: Option<Operation>) -> String {
    match operation {
        Some(operation) => format!(
            "No file is left in conflict: the {} can continue.",
            operation_name(operation)
        ),
        None => String::from("No file is left in conflict."),
    }
}
/// *1 conflict is left to resolve.*
#[must_use]
pub fn conflicts_left(count: usize) -> String {
    if count == 1 {
        String::from("1 conflict is left to resolve.")
    } else {
        format!("{count} conflicts are left to resolve.")
    }
}
/// *2 files are still in conflict.*
#[must_use]
pub fn files_left(count: usize) -> String {
    if count == 1 {
        String::from("1 file is still in conflict.")
    } else {
        format!("{count} files are still in conflict.")
    }
}
/// **Undo** in the report.
pub const UNDO: &str = "Undo";
/// Its tooltip.
pub const UNDO_TOOLTIP: &str = "Put back the conflicts of the files written, as git left them";
/// **Continue the merge**.
#[must_use]
pub fn continue_operation(operation: Operation) -> String {
    format!("Continue the {}", operation_name(operation))
}
/// Its tooltip.
#[must_use]
pub fn continue_tooltip(operation: Operation) -> String {
    format!("Commit what is staged and go on with the {}", operation_name(operation))
}
