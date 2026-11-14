//! `stash.undo-drop`: brings back a stash dropped in Gitardo, while its commit is still in the repository.

use model::CommitId;
use model::contract::{
    Answers, Context, Danger, FollowUp, JournalEntry, Outcome, Plan, Query, Refusal, Step, Text, Undo,
};

/// Stores again, with its message, the stash that the newest entry of the journal says was dropped, while nothing else
/// happened since; refuses with *Nothing to undo* otherwise.
///
/// Feature: stash.undo-drop
#[must_use]
pub fn undo_drop(context: &Context, answers: &Answers) -> Outcome {
    match answers.journal(crate::undo::JOURNAL) {
        None => Outcome::Read(crate::undo::JOURNAL, Query::Journal),
        Some(Some(
            entry @ JournalEntry {
                record: model::contract::UndoRecord::StashDropped { message, commit },
                ..
            },
        )) => Outcome::Run(restore_plan(context, entry, (message, commit))),
        Some(_) => Outcome::Refuse(Refusal::saying("Nothing to undo")),
    }
}

/// `git stash store -m <message> <hash>`, which takes back the entry of the journal of a stash dropped.
pub(crate) fn restore_plan(context: &Context, entry: &JournalEntry, (message, commit): (&str, &CommitId)) -> Plan {
    Plan {
        title: Text::from("Undo: restore dropped stash"),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["stash", "store", "-m", message, commit.as_str()],
        )],
        danger: Danger::Confirm(Text::from(format!(
            "The stash \"{message}\" comes back as stash@{{0}}."
        ))),
        undo: Undo::TakesBack(entry.number),
        after: vec![FollowUp::Say(Text::from(format!("Restored the stash \"{message}\".")))],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::undo_drop;
    use crate::testing::{commit, context, lines, plan, state};
    use crate::undo::JOURNAL;
    use model::contract::{Answer, Answers, Danger, JournalEntry, Outcome, Target, Text, Undo, UndoRecord};

    fn dropped(message: &str) -> Answers {
        let entry = JournalEntry {
            number: 3,
            title: Text::from("Drop stash@{2}"),
            record: UndoRecord::StashDropped {
                message: message.to_owned(),
                commit: commit('c'),
            },
            head: state().head,
        };
        Answers::new().with(JOURNAL, Answer::Journal(Some(entry)))
    }

    #[test]
    fn stash_undo_drop__stores_the_stash_again_with_its_message() {
        let plan = plan(undo_drop(
            &context(state(), Target::Head),
            &dropped("On main: try this"),
        ));
        assert_eq!(plan.title.as_str(), "Undo: restore dropped stash");
        assert_eq!(
            lines(&plan),
            [format!("git stash store -m \"On main: try this\" {}", commit('c'))]
        );
        assert!(matches!(plan.danger, Danger::Confirm(_)));
        assert_eq!(plan.undo, Undo::TakesBack(3));
    }

    #[test]
    fn stash_undo_drop__nothing_in_the_journal_is_nothing_to_undo() {
        let answers = Answers::new().with(JOURNAL, Answer::Journal(None));
        assert!(matches!(
            undo_drop(&context(state(), Target::Head), &answers),
            Outcome::Refuse(_)
        ));
    }

    #[test]
    fn stash_undo_drop__a_stash_dropped_from_further_down_comes_back_first() {
        let plan = plan(undo_drop(&context(state(), Target::Head), &dropped("On main: two")));
        let Danger::Confirm(note) = plan.danger else {
            panic!("Undo shows what it does first")
        };
        assert!(note.as_str().contains("stash@{0}"), "{note}");
    }
}
