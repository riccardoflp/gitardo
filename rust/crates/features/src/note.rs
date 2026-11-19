//! The notes of commits: written, removed, fetched from a remote and pushed to it (`docs/features/04-details-and-commit-page.md`,
//! D85). Git keeps a reflog for the reference of notes, so that Undo puts it back where it was.

mod edit;
mod fetch;
mod push;
mod remove;
#[cfg(test)]
mod tests;

pub use edit::{NOTE, OLD_NOTE, edit};
pub use fetch::{HAS_NOTES, fetch};
pub use push::push;
pub use remove::remove;

use model::CommitId;
use model::contract::{Context, Danger, FollowUp, JournalEntry, Plan, Refusal, Step, Target, Text, Undo, UndoRecord};

/// What Undo needs to put the reference of notes back where it is now.
fn undo(context: &Context) -> Undo {
    let notes = &context.repository.notes;
    Undo::Journal(UndoRecord::NotesMoved {
        reference: notes.reference.clone(),
        before: notes.tip.clone(),
        after: None,
    })
}

/// What is said when a note is asked of what is not a commit: the draft and a stash have none.
const ONLY_A_COMMIT: &str = "Only a commit has a note.";

/// The commit a note is written on or removed from: a commit of the timeline, not the draft nor a stash.
fn commit_of(context: &Context) -> Option<&CommitId> {
    match &context.target {
        Target::Commit(commit) => Some(commit),
        _ => None, // Only a commit of the timeline has a note.
    }
}

/// The remote the notes are fetched from or pushed to: the one invoked on, or the only one; Quick search on a
/// repository with several remotes is refused with where to find each (Q1458).
fn remote_of(context: &Context) -> Result<&str, Refusal> {
    match (&context.target, context.repository.remotes.as_slice()) {
        (Target::Remote(remote), _) => Ok(remote),
        (_, [only]) => Ok(only),
        (_, []) => Err(Refusal::saying("This repository has no remote.")),
        _ => Err(Refusal::saying(
            "This repository has several remotes: choose one in the branch rail.",
        )),
    }
}

/// The plan that takes back a move of the reference of notes: `git update-ref` back to where it was, checking that it is
/// where the plan left it, or `git update-ref -d` when there was none.
pub(crate) fn restore_plan(
    context: &Context,
    entry: &JournalEntry,
    (reference, before, after): (&str, Option<&CommitId>, Option<&CommitId>),
) -> Plan {
    let mut arguments = vec![String::from("update-ref")];
    match before {
        Some(before) => arguments.extend([reference.to_owned(), before.as_str().to_owned()]),
        None => arguments.extend([String::from("-d"), reference.to_owned()]),
    }
    arguments.extend(after.map(|after| after.as_str().to_owned()));
    let title = Text::from(format!("Undo \"{}\"", entry.title));
    Plan {
        title: title.clone(),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        // The note taken away stays in the reflog of the reference of notes.
        danger: Danger::Confirm(title),
        undo: Undo::TakesBack(entry.number),
        after: vec![FollowUp::Say(Text::from(format!("Undid \"{}\".", entry.title)))],
        on_failure: Vec::new(),
    }
}
