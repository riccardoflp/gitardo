#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::contract::{Answer, Answers, Danger, JournalEntry, Outcome, Query, Target, Text, Undo, UndoRecord};
use model::note::Notes;
use model::state::{Head, RepositoryState};

use super::{NOTE, OLD_NOTE, edit, remove, restore_plan};
use crate::testing::{commit, context, lines, plan, state};

/// A repository whose commit `aaaa…` has a note, the reference of notes at `7777…`.
fn noted() -> RepositoryState {
    RepositoryState {
        notes: Notes {
            tip: Some(commit('7')),
            noted: std::iter::once((commit('a'), commit('8'))).collect(),
            ..Notes::default()
        },
        remotes: vec![String::from("origin")],
        ..state()
    }
}

#[test]
fn note_edit__edit_note_reads_the_note_it_has_and_proposes_it() {
    let given = context(noted(), Target::Commit(commit('a')));
    assert_eq!(
        edit(&given, &Answers::new()),
        Outcome::Read(OLD_NOTE, Query::Note { commit: commit('a') })
    );
    let read = Answers::new().with(OLD_NOTE, Answer::Text(String::from("Reviewed.")));
    let Outcome::Ask(question) = edit(&given, &read) else {
        panic!("the note is asked")
    };
    assert_eq!(question.default, Some(Answer::Text(String::from("Reviewed."))));
    assert_eq!(question.title.as_str(), "Note of aaaaaaa");
    let same = read.clone().with(NOTE, Answer::Text(String::from("Reviewed.\n")));
    assert!(
        matches!(edit(&given, &same), Outcome::Refuse(_)),
        "the same note runs nothing"
    );
}

#[test]
fn note_edit__an_empty_text_is_refused_and_nothing_runs() {
    let given = context(state(), Target::Commit(commit('a')));
    let empty = Answers::new().with(NOTE, Answer::Text(String::from("  \n")));
    let Outcome::Refuse(refusal) = edit(&given, &empty) else {
        panic!("an empty note is refused")
    };
    assert_eq!(refusal.message.as_str(), "A note cannot be empty: remove it instead.");
    let typed = Answers::new().with(NOTE, Answer::Text(String::from("Reviewed.")));
    let plan = plan(edit(&given, &typed));
    assert_eq!(
        lines(&plan),
        ["git notes add -f -F - aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]
    );
    assert_eq!(
        plan.undo,
        Undo::Journal(UndoRecord::NotesMoved {
            reference: String::from("refs/notes/commits"),
            before: None,
            after: None,
        })
    );
    let stash = context(state(), Target::Stash(0));
    assert!(
        matches!(edit(&stash, &typed), Outcome::Refuse(_)),
        "a stash has no note"
    );
}

#[test]
fn note_remove__undo_puts_the_reference_back_where_it_was_or_deletes_it() {
    let given = context(noted(), Target::Commit(commit('a')));
    let plan = plan(remove(&given, &Answers::new()));
    assert_eq!(
        lines(&plan),
        ["git notes remove aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]
    );
    let without = context(state(), Target::Commit(commit('a')));
    assert!(matches!(remove(&without, &Answers::new()), Outcome::Refuse(_)));
    let entry = |before, after| JournalEntry {
        number: 3,
        title: Text::from("Remove the note of aaaaaaa"),
        record: UndoRecord::NotesMoved {
            reference: String::from("refs/notes/commits"),
            before,
            after,
        },
        head: Head::default(),
    };
    let back = entry(Some(commit('7')), Some(commit('9')));
    let (reference, before, after) = ("refs/notes/commits", Some(commit('7')), Some(commit('9')));
    let undone = restore_plan(&given, &back, (reference, before.as_ref(), after.as_ref()));
    assert_eq!(
        lines(&undone),
        [format!(
            "git update-ref refs/notes/commits {} {}",
            commit('7'),
            commit('9')
        )]
    );
    assert_eq!(undone.undo, Undo::TakesBack(3));
    let first = entry(None, Some(commit('9')));
    let undone = restore_plan(&given, &first, (reference, None, after.as_ref()));
    assert_eq!(
        lines(&undone),
        [format!("git update-ref -d refs/notes/commits {}", commit('9'))]
    );
    assert!(matches!(undone.danger, Danger::Confirm(_)));
}
