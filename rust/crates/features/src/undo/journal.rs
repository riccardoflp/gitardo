//! The Undo of an operation of Gitardo that the reflog does not see, from what the engine's journal kept of it.

use std::ffi::OsString;

use model::changes::pathspec;
use model::contract::{Context, Danger, FollowUp, JournalEntry, Plan, Step, Text, Undo, UndoRecord};

/// The plan that takes back the operation of an entry of the journal.
pub(super) fn plan(context: &Context, entry: &JournalEntry) -> Plan {
    if let UndoRecord::StashDropped { message, commit } = &entry.record {
        return crate::stash::restore_plan(context, entry, (message, commit));
    }
    if entry.record == UndoRecord::SquashMerge {
        return crate::merge::undo_squash_plan(context, entry);
    }
    if matches!(entry.record, UndoRecord::ConflictsResolved { .. }) {
        return crate::conflicts::restore_plan(context, entry);
    }
    let (what, arguments) = what_and_arguments(entry);
    let title = Text::from(format!("Undo {what}"));
    let warning = match entry.record {
        UndoRecord::BranchDeleted { .. } => Text::from(format!("{title}. The branch does not get its upstream back.")),
        _ => title.clone(), // Nothing more to say than what it takes back.
    };
    Plan {
        title,
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::Destructive(warning),
        undo: Undo::TakesBack(entry.number),
        after: vec![
            FollowUp::SelectHead,
            FollowUp::Say(Text::from(format!("Undid {what}."))),
        ],
        on_failure: Vec::new(),
    }
}

/// What the entry did, in words, and the arguments of the git command that takes it back.
fn what_and_arguments(entry: &JournalEntry) -> (String, Vec<OsString>) {
    let words = |list: &[&str]| list.iter().map(OsString::from).collect::<Vec<_>>();
    match &entry.record {
        UndoRecord::BranchDeleted { name, commit } => (
            format!("the deletion of branch {name}"),
            words(&["branch", name, commit.as_str()]),
        ),
        UndoRecord::BranchCreated { name } => {
            (format!("the creation of branch {name}"), words(&["branch", "-D", name]))
        }
        UndoRecord::StashDropped { message, commit } => (
            format!("the drop of the stash \"{message}\""),
            words(&["stash", "store", "-m", message, commit.as_str()]),
        ),
        UndoRecord::ConflictsResolved { files } => {
            let mut arguments = words(&["checkout", "--merge", "--"]);
            arguments.extend(files.iter().map(|file| pathspec(file)));
            let count = match files.len() {
                1 => String::from("1 file"),
                count => format!("{count} files"),
            };
            (format!("the resolution of the conflicts in {count}"), arguments)
        }
        UndoRecord::SquashMerge => (String::from("the squash merge"), words(&["reset", "--merge"])),
        UndoRecord::Series { head_before } => (
            format!("\"{}\"", entry.title),
            words(&["reset", "--keep", head_before.as_str()]),
        ),
        UndoRecord::SplitLast { commit } => (
            format!("the split of {}", commit.short()),
            words(&["reset", "--mixed", commit.as_str()]),
        ), // `undo.last` takes these back by the reflog, or refuses: never here.
        UndoRecord::Reflog | UndoRecord::NotUndoable { .. } => (format!("\"{}\"", entry.title), Vec::new()),
    }
}
