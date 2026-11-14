//! `conflicts.restore`: marks the conflicts of files again as git had marked them when it stopped, after Gitardo wrote
//! resolutions in them or took one side of them.

use std::ffi::OsString;

use model::changes::pathspec;
use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, JournalEntry, Outcome, Plan, Query, Step, Text, Undo, UndoRecord,
};

use super::refuse;

/// The newest entry of the engine's journal that still holds.
pub const JOURNAL: AnswerKey = AnswerKey("journal");

/// Marks again the conflicts of the files the last resolution of Gitardo wrote, staged ones included:
/// `git checkout --merge -- <file>…`. It is offered only while that resolution is the last thing Gitardo did and
/// `HEAD` has not moved since, as the journal of Undo keeps it.
///
/// Feature: conflicts.restore
#[must_use]
pub fn restore(context: &Context, answers: &Answers) -> Outcome {
    match answers.journal(JOURNAL) {
        None => Outcome::Read(JOURNAL, Query::Journal),
        Some(Some(entry)) if matches!(entry.record, UndoRecord::ConflictsResolved { .. }) => {
            Outcome::Run(restore_plan(context, entry))
        }
        Some(_) => refuse("There is no resolution to undo."),
    }
}

/// The plan that marks again the conflicts of the files of an entry of the journal; it takes the entry back.
pub(crate) fn restore_plan(context: &Context, entry: &JournalEntry) -> Plan {
    let files = match &entry.record {
        UndoRecord::ConflictsResolved { files } => files.as_slice(),
        // Only a resolution of conflicts has files to mark again.
        _ => &[],
    };
    let what = match files {
        [file] => file.clone(),
        files => format!("{} files", files.len()),
    };
    let mut arguments = vec![OsString::from("checkout"), "--merge".into(), "--".into()];
    arguments.extend(files.iter().map(|file| pathspec(file)));
    let title = Text::from(format!("Undo: restore the conflicts of {what}"));
    Plan {
        title: title.clone(),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::Destructive(Text::from(
            "The files go back to how git left them when it stopped, with every conflict marked. What was changed in \
             them since is lost.",
        )),
        undo: Undo::TakesBack(entry.number),
        after: vec![FollowUp::Say(Text::from(format!("Restored the conflicts of {what}.")))],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{JOURNAL, restore};
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answer, Answers, Danger, JournalEntry, Outcome, Query, Target, Text, UndoRecord};

    fn resolved(files: &[&str]) -> Answers {
        let entry = JournalEntry {
            number: 4,
            title: Text::from("Resolve all 3 conflicts automatically"),
            record: UndoRecord::ConflictsResolved {
                files: files.iter().map(|file| (*file).to_owned()).collect(),
            },
            head: model::state::Head::default(),
        };
        Answers::new().with(JOURNAL, Answer::Journal(Some(entry)))
    }

    #[test]
    fn conflicts_restore__marks_every_file_written_again_staged_ones_included() {
        let context = context(state(), Target::Head);
        assert!(matches!(
            restore(&context, &Answers::new()),
            Outcome::Read(JOURNAL, Query::Journal)
        ));
        let plan = plan(restore(&context, &resolved(&["first.txt", "second.txt"])));
        assert_eq!(lines(&plan), ["git checkout --merge -- first.txt second.txt"]);
        assert_eq!(plan.title.as_str(), "Undo: restore the conflicts of 2 files");
    }

    #[test]
    fn conflicts_restore__asks_first_with_the_warning() {
        let plan = plan(restore(&context(state(), Target::Head), &resolved(&["a.txt"])));
        assert_eq!(plan.title.as_str(), "Undo: restore the conflicts of a.txt");
        assert!(matches!(plan.danger, Danger::Destructive(_)));
    }

    #[test]
    fn with_no_resolution_in_the_journal_it_refuses() {
        let answers = Answers::new().with(JOURNAL, Answer::Journal(None));
        assert!(matches!(
            restore(&context(state(), Target::Head), &answers),
            Outcome::Refuse(_)
        ));
    }
}
