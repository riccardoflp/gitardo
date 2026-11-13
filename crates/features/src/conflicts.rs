//! While a merge, a rebase, a cherry-pick, a revert or a patch has stopped, or files are in conflict without one: going
//! on, skipping a step, going back, staging a file resolved, taking a whole file from one side, resolving the
//! conflicts that need no choice, and marking them again (`docs/features/09-conflicts.md`).

mod abort_operation;
mod continue_operation;
mod mark_resolved;
mod resolve_automatically;
mod restore;
mod rules;
mod skip_step;
mod take_side;

pub use abort_operation::abort_operation;
pub use continue_operation::continue_operation;
pub use mark_resolved::mark_resolved;
pub(crate) use mark_resolved::staging;
pub use resolve_automatically::{AGAIN, RESOLVE, TEXTS, resolve_automatically};
pub use restore::{JOURNAL, restore};
pub use rules::{FileResolution, count_by_rules, resolve_files};
pub use skip_step::skip_step;
pub use take_side::{SIDE, STAGES, take_side};

pub(crate) use restore::restore_plan;

use model::contract::{Context, Outcome, Refusal};
use model::state::Operation;

/// The operation in progress, as the commands of git name it: `merge`, `rebase`, `cherry-pick`, `revert`, `am`.
pub(crate) fn command(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Patch => "am",
    }
}

/// The operation in progress, as the titles and the messages name it: `merge`, `rebase`, `cherry-pick`, `revert`,
/// `patch`.
#[must_use]
pub fn noun(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Patch => "patch",
    }
}

/// A refusal that offers nothing else.
pub(crate) fn refuse(message: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(message))
}

/// The operation in progress, or the refusal when there is none.
pub(crate) fn in_progress(context: &Context) -> Result<Operation, Box<Outcome>> {
    context
        .repository
        .operation
        .ok_or_else(|| Box::new(refuse("No merge, rebase, cherry-pick, revert or patch is in progress.")))
}

/// Whether a file is in conflict, as the repository was last read.
pub(crate) fn is_in_conflict(context: &Context, path: &str) -> bool {
    context
        .repository
        .working_tree
        .conflicted
        .iter()
        .any(|file| file.path == path)
}

/// The file an operation of a row of the Changes page is invoked on.
pub(crate) fn target_file(context: &Context) -> Option<&str> {
    match &context.target {
        model::contract::Target::File(path) => Some(path),
        model::contract::Target::Files(paths) if paths.len() == 1 => paths.first().map(String::as_str),
        _ => None, // The other targets name no file.
    }
}
