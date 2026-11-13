//! `merge.undo-squash`: takes back a squash merge, which staged its changes and moved no branch, so that the reflog
//! cannot take it back.

use model::contract::{
    Answers, Context, Danger, FollowUp, JournalEntry, Outcome, Plan, Query, Refusal, Step, Text, Undo, UndoRecord,
};

use crate::undo::JOURNAL;

/// Takes back the squash merge the journal kept, as long as nothing was committed or moved since: `git reset --merge`,
/// which takes what is staged out of the index and the working tree, also when it stopped on conflicts.
///
/// Feature: merge.undo-squash
#[must_use]
pub fn undo_squash(context: &Context, answers: &Answers) -> Outcome {
    match answers.journal(JOURNAL) {
        None => Outcome::Read(JOURNAL, Query::Journal),
        Some(Some(entry)) if entry.record == UndoRecord::SquashMerge => Outcome::Run(undo_squash_plan(context, entry)),
        Some(_) => Outcome::Refuse(Refusal::saying("There is no squash merge to undo.")),
    }
}

/// The plan that takes back the squash merge of an entry of the journal, which `undo.last` runs too.
pub(crate) fn undo_squash_plan(context: &Context, entry: &JournalEntry) -> Plan {
    let title = format!("Undo \"{}\"", entry.title);
    Plan {
        title: Text::from(title.as_str()),
        steps: vec![Step::git(context.repository.folder.clone(), ["reset", "--merge"])],
        danger: Danger::Confirm(Text::from(
            "What is staged goes back to HEAD, in the index and in the working tree, and so does what was staged \
             since. Changes that are not staged stay.",
        )),
        undo: Undo::TakesBack(entry.number),
        after: vec![
            FollowUp::SelectHead,
            FollowUp::Say(Text::from(format!("Undid \"{}\".", entry.title))),
        ],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::undo_squash;
    use crate::testing::{context, lines, plan, state};
    use crate::undo::JOURNAL;
    use model::contract::{Answer, Answers, Danger, JournalEntry, Outcome, Target, Text, UndoRecord};

    fn entry(record: UndoRecord) -> Answers {
        Answers::new().with(
            JOURNAL,
            Answer::Journal(Some(JournalEntry {
                number: 3,
                title: Text::from("Merge feature into main (squash)"),
                record,
                head: state().head,
            })),
        )
    }

    #[test]
    fn merge_undo_squash__right_after_a_squash_merge_runs_git_reset_merge() {
        let context = context(state(), Target::Head);
        let plan = plan(undo_squash(&context, &entry(UndoRecord::SquashMerge)));
        assert_eq!(lines(&plan), ["git reset --merge"]);
        assert_eq!(plan.title.as_str(), "Undo \"Merge feature into main (squash)\"");
        assert!(matches!(plan.danger, Danger::Confirm(_)));
    }

    #[test]
    fn merge_undo_squash__with_no_squash_merge_in_the_journal_it_runs_nothing() {
        let context = context(state(), Target::Head);
        assert!(matches!(
            undo_squash(&context, &Answers::new().with(JOURNAL, Answer::Journal(None))),
            Outcome::Refuse(_)
        ));
    }
}
