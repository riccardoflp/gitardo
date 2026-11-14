use super::{DragItem, Dropped, Onto, dragged_item, drop_entries, entry_intent};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use model::CommitId;
use model::contract::{Answer, AnswerKey, FeatureId, Target};
use model::state::{Branch, Head, RepositoryState};

/// A commit of the tests.
fn commit(letter: char) -> CommitId {
    CommitId::parse(&letter.to_string().repeat(40)).unwrap()
}

/// A local branch at a commit.
fn branch(name: &str, at: char, is_checked_out: bool) -> Branch {
    Branch {
        name: name.to_owned(),
        commit: commit(at),
        upstream: None,
        is_checked_out,
        date: model::Timestamp(0),
    }
}

/// `main` checked out at `a`, `feature` at `b`.
fn repository() -> RepositoryState {
    RepositoryState {
        head: Head {
            commit: Some(commit('a')),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![branch("main", 'a', true), branch("feature", 'b', false)],
        ..RepositoryState::default()
    }
}

#[test]
fn what_a_drag_carries_is_read_back_and_a_drag_of_something_else_is_not() {
    for item in [
        DragItem::Commit(commit('c')),
        DragItem::Branch(String::from("feature/login")),
        DragItem::RemoteBranch(String::from("origin/feature")),
    ] {
        assert_eq!(dragged_item(&item.payload()), Some(item));
    }
    assert_eq!(dragged_item(""), None, "a line of the interactive rebase");
    assert_eq!(dragged_item("commit not-a-hash"), None);
}

#[test]
fn a_branch_dropped_on_another_commit_is_rebased_there_with_its_name_answered() {
    let repository = repository();
    let dropped = Dropped {
        item: DragItem::Branch(String::from("feature")),
        onto: Onto::Commit(commit('c')),
    };
    let entries = drop_entries(&repository, &dropped);
    let [entry] = entries.as_slice() else {
        panic!("one entry: {entries:?}");
    };
    let Intent::Feature(FeatureIntent::RunWith(feature, target, answers)) = entry_intent(&repository, &dropped, entry)
    else {
        panic!("the rebase runs with the branch answered");
    };
    assert_eq!(feature, FeatureId("rebase.branch-onto"));
    assert_eq!(target, Target::Commit(commit('c')));
    assert_eq!(
        answers.get(AnswerKey("branch")),
        Some(&Answer::Choice(String::from("feature")))
    );
}

#[test]
fn the_branch_checked_out_dropped_on_another_commit_is_rebased_or_reset_on_that_commit() {
    let repository = repository();
    let dropped = Dropped {
        item: DragItem::Branch(String::from("main")),
        onto: Onto::Commit(commit('c')),
    };
    let intents: Vec<Intent> = drop_entries(&repository, &dropped)
        .iter()
        .map(|entry| entry_intent(&repository, &dropped, entry))
        .collect();
    let run = |feature| Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Commit(commit('c'))));
    assert_eq!(intents, [run("rebase.onto"), run("reset.to-commit")]);
}

#[test]
fn a_branch_dropped_on_the_draft_is_merged_or_rebased_onto_and_one_gone_offers_nothing() {
    let repository = repository();
    let dropped = Dropped {
        item: DragItem::Branch(String::from("feature")),
        onto: Onto::Draft,
    };
    let intents: Vec<Intent> = drop_entries(&repository, &dropped)
        .iter()
        .map(|entry| entry_intent(&repository, &dropped, entry))
        .collect();
    let run = |feature| {
        Intent::Feature(FeatureIntent::Run(
            FeatureId(feature),
            Target::Branch(String::from("feature")),
        ))
    };
    assert_eq!(
        intents,
        [run("merge.into-current"), run("merge.choose-way"), run("rebase.onto")]
    );
    let gone = Dropped {
        item: DragItem::Branch(String::from("deleted")),
        onto: Onto::Draft,
    };
    assert_eq!(drop_entries(&repository, &gone), []);
}
