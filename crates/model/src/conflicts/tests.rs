#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{NamedCommit, Side, SideNamesInput, SideRole, side_names};
use crate::CommitId;
use crate::state::Operation;

/// A commit of a hash made of one digit, with some references at it and a subject.
fn commit(digit: char, references: &[&str]) -> (CommitId, Vec<String>) {
    let id = CommitId::parse(&digit.to_string().repeat(40)).unwrap();
    (id, references.iter().map(|reference| (*reference).to_owned()).collect())
}

/// A side as the derivation gives it.
fn side(role: SideRole, name: Option<&str>, subject: Option<&str>) -> Side {
    Side {
        role,
        name: name.map(str::to_owned),
        subject: subject.map(str::to_owned),
    }
}

#[test]
fn conflicts_side_names__a_merge_names_the_branch_checked_out_and_the_branch_merged() {
    let (head, at_head) = commit('a', &["refs/heads/main", "refs/remotes/origin/main"]);
    let (merged, at_merged) = commit('b', &["refs/heads/feature/map-view"]);
    let names = side_names(&SideNamesInput {
        operation: Some(Operation::Merge),
        checked_out: Some("main"),
        head: Some(NamedCommit {
            id: &head,
            references: &at_head,
            subject: Some("Show the map"),
        }),
        brought_in: Some(NamedCommit {
            id: &merged,
            references: &at_merged,
            subject: Some("Draw the roads"),
        }),
        ..SideNamesInput::default()
    });
    assert_eq!(
        names.current,
        side(SideRole::BranchCheckedOut, Some("main"), Some("Show the map"))
    );
    assert_eq!(
        names.incoming,
        side(
            SideRole::BranchBeingMerged,
            Some("feature/map-view"),
            Some("Draw the roads")
        )
    );
    assert_eq!(names.incoming.role.words(), "The branch being merged");
}

#[test]
fn conflicts_side_names__a_rebase_names_the_branch_onto_and_the_branch_rebased() {
    let (head, none) = commit('c', &[]);
    let (onto, at_onto) = commit('d', &["refs/heads/main", "refs/remotes/origin/HEAD"]);
    let (replayed, _) = commit('e', &[]);
    let names = side_names(&SideNamesInput {
        operation: Some(Operation::Rebase),
        head: Some(NamedCommit {
            id: &head,
            references: &none,
            subject: Some("Add the parser"),
        }),
        brought_in: Some(NamedCommit {
            id: &replayed,
            references: &none,
            subject: Some("Read the numbers"),
        }),
        onto: Some(NamedCommit {
            id: &onto,
            references: &at_onto,
            subject: None,
        }),
        rebased_branch: Some("refs/heads/topic"),
        ..SideNamesInput::default()
    });
    assert_eq!(names.current.name.as_deref(), Some("main"));
    assert_eq!(names.current.role.words(), "What the branch is being rebased onto");
    assert_eq!(
        names.incoming,
        side(SideRole::CommitBeingReplayed, Some("topic"), Some("Read the numbers"))
    );
}

#[test]
fn conflicts_side_names__a_cherry_pick_names_the_commit_by_its_short_hash() {
    let (head, at_head) = commit('a', &["refs/heads/main"]);
    let picked = CommitId::parse("3f2a1c9d0e4b5a6978877665544332211aabbccd").unwrap();
    let at_picked = vec![String::from("refs/heads/fix")];
    let names = side_names(&SideNamesInput {
        operation: Some(Operation::CherryPick),
        checked_out: Some("main"),
        head: Some(NamedCommit {
            id: &head,
            references: &at_head,
            subject: None,
        }),
        brought_in: Some(NamedCommit {
            id: &picked,
            references: &at_picked,
            subject: Some("Fix the totals"),
        }),
        ..SideNamesInput::default()
    });
    assert_eq!(names.current.name.as_deref(), Some("main"));
    assert_eq!(
        names.incoming,
        side(SideRole::CommitBeingPicked, Some("3f2a1c9"), Some("Fix the totals"))
    );
}

#[test]
fn conflicts_side_names__a_detached_head_with_no_branch_is_its_short_hash() {
    let head = CommitId::parse("9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4e3f2a1b0c").unwrap();
    let remote_head = vec![String::from("refs/remotes/origin/HEAD")];
    let (merged, at_merged) = commit('b', &[]);
    let names = side_names(&SideNamesInput {
        operation: Some(Operation::Merge),
        checked_out: None,
        head: Some(NamedCommit {
            id: &head,
            references: &remote_head,
            subject: None,
        }),
        brought_in: Some(NamedCommit {
            id: &merged,
            references: &at_merged,
            subject: None,
        }),
        ..SideNamesInput::default()
    });
    assert_eq!(names.current.name.as_deref(), Some("9b8c7d6"));
    assert_eq!(names.incoming.name.as_deref(), Some("bbbbbbb"));
}

#[test]
fn conflicts_side_names__a_stash_applied_has_words_and_no_names() {
    let (head, at_head) = commit('a', &["refs/heads/main"]);
    let names = side_names(&SideNamesInput {
        operation: None,
        checked_out: Some("main"),
        head: Some(NamedCommit {
            id: &head,
            references: &at_head,
            subject: Some("Show the map"),
        }),
        ..SideNamesInput::default()
    });
    assert_eq!(
        (names.current.role.words(), names.incoming.role.words()),
        ("Your files", "The changes being applied")
    );
    assert_eq!((names.current.name, names.incoming.name), (None, None));
}
