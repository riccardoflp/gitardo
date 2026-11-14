#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{Checkout, Distance, Submodule, SubmoduleStatus, status};
use crate::CommitId;

fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap()
}

/// A submodule `lib` recorded at `recorded`, with what its folder has checked out.
fn submodule(recorded: Option<CommitId>, checkout: Option<Checkout>) -> Submodule {
    Submodule {
        name: String::from("lib"),
        folder: String::from("lib"),
        parent: None,
        url: Some(String::from("https://example.com/lib.git")),
        branch: None,
        recorded,
        staged: None,
        checkout,
    }
}

/// A checkout of a commit, at a distance from the recorded one.
fn checkout(commit: CommitId, distance: Option<Distance>) -> Checkout {
    Checkout {
        commit: Some(commit),
        branch: None,
        uncommitted: 0,
        distance,
    }
}

fn distance(ahead: u32, behind: u32) -> Distance {
    Distance { ahead, behind }
}

#[test]
fn submodule_status__each_row_of_the_table_is_given_in_its_case() {
    let recorded = commit('a');
    let other = commit('b');
    let cases = [
        (submodule(Some(recorded.clone()), None), SubmoduleStatus::NotInitialized),
        (
            submodule(None, Some(checkout(other.clone(), None))),
            SubmoduleStatus::Added,
        ),
        (
            submodule(Some(recorded.clone()), Some(checkout(other.clone(), None))),
            SubmoduleStatus::Unknown {
                recorded: recorded.clone(),
            },
        ),
        (
            submodule(
                Some(recorded.clone()),
                Some(checkout(other.clone(), Some(distance(2, 1)))),
            ),
            SubmoduleStatus::Both {
                distance: Distance { ahead: 2, behind: 1 },
                recorded: recorded.clone(),
            },
        ),
        (
            submodule(
                Some(recorded.clone()),
                Some(checkout(other.clone(), Some(distance(2, 0)))),
            ),
            SubmoduleStatus::Ahead {
                commits: 2,
                recorded: recorded.clone(),
            },
        ),
        (
            submodule(
                Some(recorded.clone()),
                Some(checkout(other.clone(), Some(distance(0, 1)))),
            ),
            SubmoduleStatus::Behind {
                commits: 1,
                recorded: recorded.clone(),
            },
        ),
        (
            submodule(Some(recorded.clone()), Some(checkout(recorded.clone(), None))),
            SubmoduleStatus::OnIt {
                recorded: recorded.clone(),
            },
        ),
    ];
    for (submodule, expected) in cases {
        assert_eq!(status(&submodule), expected, "{submodule:?}");
    }
}

#[test]
fn submodule_status__one_commit_is_said_1_commit_more_n_commits() {
    let recorded = commit('a');
    let other = commit('b');
    let said = |distance| {
        status(&submodule(
            Some(recorded.clone()),
            Some(checkout(other.clone(), distance)),
        ))
        .sentence()
    };
    assert_eq!(
        said(Some(distance(2, 1))),
        "2 commits ahead of and 1 commit behind the recorded commit aaaaaaa"
    );
    assert_eq!(
        said(Some(distance(1, 0))),
        "1 commit ahead of the recorded commit aaaaaaa"
    );
    assert_eq!(
        said(Some(distance(0, 3))),
        "3 commits behind the recorded commit aaaaaaa"
    );
    assert_eq!(
        status(&submodule(
            Some(recorded.clone()),
            Some(checkout(recorded.clone(), None))
        ))
        .sentence(),
        "On the recorded commit aaaaaaa"
    );
    assert_eq!(
        said(None),
        "The recorded commit aaaaaaa is not in it: fetch in it, or update it"
    );
    assert_eq!(
        status(&submodule(None, None)).sentence(),
        "Not initialized: its folder has no checkout"
    );
    assert_eq!(
        status(&submodule(None, Some(checkout(other, None)))).sentence(),
        "Added, not committed yet"
    );
}
