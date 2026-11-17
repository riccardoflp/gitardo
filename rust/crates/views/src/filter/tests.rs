//! The words of the bar above the commits, the rows of the list of the branch filter, the entries of the menus of a
//! reference and the kinds of the menu of the view.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::filter::{BranchesShown, Kinds, ReferenceKind};
use model::refs::References;
use model::state::{Branch, RemoteBranch, Tag};
use model::{CommitId, Timestamp};
use ui::{Availability, MenuEntry};

use super::list::{PickRow, pick_rows};
use super::{FilterEntry, filter_entries, filter_entry_intent};
use crate::graph::kind_entries;
use crate::intent::{FilterIntent, Intent};
use crate::text::filter::{CHOSEN_IN_FILTER, bar_words};

fn commit() -> CommitId {
    CommitId::parse(&"a".repeat(40)).unwrap_or_else(|| panic!("no hash"))
}

fn references(branches: &[&str], remotes: &[&str], tags: &[&str]) -> References {
    References {
        branches: branches
            .iter()
            .map(|name| Branch {
                name: (*name).to_owned(),
                commit: commit(),
                upstream: None,
                is_checked_out: *name == "main",
                date: Timestamp(0),
            })
            .collect(),
        remote_branches: remotes
            .iter()
            .map(|name| RemoteBranch {
                name: (*name).to_owned(),
                remote: String::from("origin"),
                commit: commit(),
                date: Timestamp(0),
            })
            .collect(),
        tags: tags
            .iter()
            .map(|name| Tag {
                name: (*name).to_owned(),
                commit: commit(),
                date: Timestamp(0),
            })
            .collect(),
    }
}

fn ticked(names: &[&str]) -> BranchesShown {
    BranchesShown::References(names.iter().map(|name| (*name).to_owned()).collect())
}

#[test]
fn filter_bar__narrowed_to_five_references_it_names_three_and_2_more() {
    let shown = ticked(&[
        "refs/heads/main",
        "refs/heads/feature",
        "refs/heads/fix",
        "refs/tags/v1.0",
        "refs/tags/v2.0",
    ]);
    assert_eq!(
        bar_words(&shown, (40, false)),
        "Only the history of main, feature, fix and 2 more \u{b7} 40 commits"
    );
}

#[test]
fn filter_bar__narrowed_to_upstream_it_says_the_current_branch_and_its_upstream() {
    assert_eq!(
        bar_words(&BranchesShown::Upstream, (1, false)),
        "Only the history of the current branch and its upstream \u{b7} 1 commit"
    );
    let local_and_tags = BranchesShown::Kinds(Kinds::only(ReferenceKind::Local).with(ReferenceKind::Tag, true));
    assert_eq!(
        bar_words(&local_and_tags, (2, false)),
        "Only the history of the local branches and the tags \u{b7} 2 commits"
    );
}

#[test]
fn filter_bar__with_1000_commits_loaded_and_more_it_says_1000_plus() {
    let shown = ticked(&["refs/heads/main"]);
    assert_eq!(
        bar_words(&shown, (1000, true)),
        "Only the history of main \u{b7} 1000+ commits"
    );
}

#[test]
fn filter_picker__typing_main_lists_main_before_origin_main_before_maintenance() {
    let references = references(&["domain", "main", "maintenance", "fix"], &["origin/main"], &["v1.0"]);
    let rows: Vec<String> = pick_rows(&references, "main")
        .into_iter()
        .map(|row| match row {
            PickRow::Heading(text) | PickRow::Empty(text) => text.to_owned(),
            PickRow::Reference { name, .. } => format!("  {name}"),
        })
        .collect();
    assert_eq!(
        rows,
        ["  main", "  origin/main", "  maintenance", "  domain"],
        "one list of the matches, the best first, while something is typed"
    );
    let all: Vec<String> = pick_rows(&references, "")
        .into_iter()
        .filter_map(|row| match row {
            PickRow::Heading(text) => Some(text.to_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(all, ["Local", "Remotes", "Tags"], "the groups while nothing is typed");
}

#[test]
fn filter_picker__with_no_name_left_or_no_reference_it_says_so() {
    let some = references(&["main"], &[], &[]);
    assert_eq!(
        pick_rows(&some, "zzz"),
        [PickRow::Empty("No branch or tag has that in its name")]
    );
    let none = references(&[], &[], &[]);
    assert_eq!(pick_rows(&none, ""), [PickRow::Empty("No branches or tags yet")]);
}

#[test]
fn filter_show_too__narrowed_to_main_the_entry_on_feature_shows_both() {
    let references = references(&["main", "feature"], &[], &[]);
    let main = ticked(&["refs/heads/main"]);
    let entries = filter_entries(Some(&main), "refs/heads/feature", &references);
    assert_eq!(
        entries,
        [FilterEntry::ShowOnly, FilterEntry::ShowToo, FilterEntry::ShowAll]
    );
    assert_eq!(
        filter_entry_intent(FilterEntry::ShowToo, Some(&main), "refs/heads/feature", &references),
        Intent::Filter(FilterIntent::Show(Some(ticked(&[
            "refs/heads/main",
            "refs/heads/feature"
        ]))))
    );
    assert_eq!(
        filter_entries(None, "refs/heads/feature", &references),
        [FilterEntry::ShowOnly]
    );
}

#[test]
fn filter_hide__narrowed_to_main_and_feature_the_entry_on_feature_shows_main_alone() {
    let references = references(&["main", "feature"], &[], &[]);
    let both = ticked(&["refs/heads/main", "refs/heads/feature"]);
    assert_eq!(
        filter_entries(Some(&both), "refs/heads/feature", &references),
        [FilterEntry::ShowOnly, FilterEntry::Hide, FilterEntry::ShowAll]
    );
    assert_eq!(
        filter_entry_intent(FilterEntry::Hide, Some(&both), "refs/heads/feature", &references),
        Intent::Filter(FilterIntent::Show(Some(ticked(&["refs/heads/main"]))))
    );
}

#[test]
fn filter_show_all__the_entry_shows_everything() {
    let references = references(&["main"], &[], &[]);
    let main = ticked(&["refs/heads/main"]);
    assert_eq!(
        filter_entries(Some(&main), "refs/heads/main", &references),
        [FilterEntry::ShowAll]
    );
    assert_eq!(
        filter_entry_intent(FilterEntry::ShowAll, Some(&main), "refs/heads/main", &references),
        Intent::Filter(FilterIntent::Show(None))
    );
}

#[test]
fn filter_kinds__with_main_ticked_alone_the_three_entries_are_greyed_out_with_chosen_in_the_filter() {
    let entries = kind_entries(None);
    assert_eq!(entries.len(), 3);
    for entry in &entries {
        let MenuEntry::Item(item) = entry else {
            panic!("a kind is an item");
        };
        assert_eq!(item.availability, Availability::Disabled);
        assert_eq!(item.shortcut, Some(CHOSEN_IN_FILTER));
    }
}

#[test]
fn filter_kinds__the_one_kind_left_on_is_greyed_out_and_the_others_can_be_chosen() {
    let tags = kind_entries(Some(Kinds::only(ReferenceKind::Tag)));
    let availability: Vec<Availability> = tags
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some(item.availability),
            _ => None,
        })
        .collect();
    assert_eq!(
        availability,
        [Availability::Enabled, Availability::Enabled, Availability::Disabled]
    );
}
