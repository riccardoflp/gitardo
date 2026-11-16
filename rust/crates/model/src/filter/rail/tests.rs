//! What the branch rail lists from the choice of its own filter, from references made by hand.

use super::super::testing::references;
use super::super::{BranchesShown, Kinds, ReferenceKind, preset};
use super::{Choice, Listed, listed};
use crate::filter::Preset;

fn ticked(names: &[&str]) -> BranchesShown {
    BranchesShown::References(names.iter().map(|name| (*name).to_owned()).collect())
}

/// The full names a choice lists, in a stable order; none for everything.
fn names(listed: &Listed) -> Option<Vec<String>> {
    match listed {
        Listed::Everything => None,
        Listed::Only(names) => {
            let mut names: Vec<String> = names.iter().cloned().collect();
            names.sort();
            Some(names)
        }
    }
}

#[test]
fn refs_listed__all_lists_every_branch_remote_branch_and_tag() {
    let repository = references(&["main", "feature"], &["origin/main"], &["v1.0"]);
    let all = listed(&Choice { listed: None }, &repository);
    assert_eq!(all, Listed::Everything);
    for name in [
        "refs/heads/main",
        "refs/heads/feature",
        "refs/remotes/origin/main",
        "refs/tags/v1.0",
    ] {
        assert!(all.has(name), "{name}");
    }
}

#[test]
fn refs_listed__current_lists_the_branch_checked_out_and_its_upstream() {
    let repository = references(&["main", "feature"], &["origin/main", "origin/feature"], &["v1.0"]);
    let current = preset(Preset::Current);
    let chosen = listed(
        &Choice {
            listed: current.as_ref(),
        },
        &repository,
    );
    assert_eq!(
        names(&chosen),
        Some(vec![
            String::from("refs/heads/main"),
            String::from("refs/remotes/origin/main")
        ])
    );
    let upstream = preset(Preset::Upstream);
    assert_eq!(
        listed(
            &Choice {
                listed: upstream.as_ref()
            },
            &repository
        ),
        chosen
    );
}

#[test]
fn refs_listed__with_main_and_old_ticked_and_old_deleted_the_rail_lists_main() {
    let repository = references(&["main", "feature"], &[], &[]);
    let choice = ticked(&["refs/heads/main", "refs/heads/old"]);
    let chosen = listed(&Choice { listed: Some(&choice) }, &repository);
    assert_eq!(names(&chosen), Some(vec![String::from("refs/heads/main")]));
    let all_gone = ticked(&["refs/heads/old"]);
    assert_eq!(
        listed(
            &Choice {
                listed: Some(&all_gone)
            },
            &repository
        ),
        Listed::Everything,
        "with none left, everything is listed"
    );
}

#[test]
fn refs_listed__local_lists_the_local_branches_alone() {
    let repository = references(&["main", "feature"], &["origin/main"], &["v1.0"]);
    let local = BranchesShown::Kinds(Kinds::only(ReferenceKind::Local));
    let chosen = listed(&Choice { listed: Some(&local) }, &repository);
    assert_eq!(
        names(&chosen),
        Some(vec![
            String::from("refs/heads/feature"),
            String::from("refs/heads/main")
        ])
    );
}
