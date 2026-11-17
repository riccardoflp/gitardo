//! The Submodules list of the branch rail made from submodules made by hand (`submodule.section`): its rows, the names
//! and the indents of submodules inside others, the tooltip, the marks of where each stands and its menu.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::CommitId;
use model::contract::OpenIn;
use model::refs::{References, SortOrder};
use model::state::RepositoryState;
use model::submodule::{Checkout, Distance, Submodule, status};

use super::ReferencesState;
use super::rows::{Entry, RailRow, rail_rows};
use super::submodules::{menu_groups, shown_name, status_marks, tooltip};
use crate::icons;
use crate::intent::{RailList, RailLists};

/// A commit of a test, by a digit.
fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("no hash"))
}

/// `lib`, two commits ahead of its recorded commit on `main`, with one file changed.
fn lib() -> Submodule {
    Submodule {
        name: String::from("lib"),
        folder: String::from("lib"),
        parent: None,
        url: Some(String::from("https://example.com/acme/lib.git")),
        branch: None,
        recorded: Some(commit('1')),
        staged: None,
        checkout: Some(Checkout {
            commit: Some(commit('2')),
            branch: Some(String::from("main")),
            uncommitted: 1,
            distance: Some(Distance { ahead: 2, behind: 0 }),
        }),
    }
}

/// A submodule on its recorded commit, inside another when it has a parent.
fn on_it(folder: &str, parent: Option<&str>) -> Submodule {
    Submodule {
        name: folder.rsplit('/').next().unwrap_or(folder).to_owned(),
        folder: folder.to_owned(),
        parent: parent.map(str::to_owned),
        url: None,
        branch: None,
        recorded: Some(commit('3')),
        staged: None,
        checkout: Some(Checkout {
            commit: Some(commit('3')),
            branch: None,
            uncommitted: 0,
            distance: Some(Distance::default()),
        }),
    }
}

/// The names of the Submodules list's entries, open, of a repository with those submodules.
fn listed(submodules: Vec<Submodule>) -> Option<Vec<String>> {
    let repository = RepositoryState {
        submodules,
        ..RepositoryState::default()
    };
    let references = References::default();
    let mut lists = RailLists::default();
    let _ = lists.turned.insert(RailList::Submodules);
    let state = ReferencesState {
        repository: &repository,
        references: &references,
        lists: &lists,
        branch_colors: &std::collections::HashMap::new(),
        remote_colors: &std::collections::HashMap::new(),
        favorites: &[],
        recent: &[],
        typed: "",
        shown: None,
        list_filter: super::stashes_tests::everything_listed(&references),
        selected: None,
        sort: SortOrder::Name,
        open_in: OpenIn::Checkout,
        window: crate::worktree::WorktreeWindow::default(),
        stashes: super::stashes_tests::nothing_unfolded(),
        github: crate::github::GitHubShown::default(),
    };
    let rows = rail_rows(&state, false);
    let has_list = rows
        .iter()
        .any(|row| matches!(row, RailRow::Kept(head) if head.list == RailList::Submodules));
    has_list.then(|| {
        rows.iter()
            .filter_map(|row| match row {
                RailRow::Entry {
                    entry: Entry::Submodule(submodule),
                    ..
                } => Some(shown_name(submodule).to_owned()),
                _ => None,
            })
            .collect()
    })
}

#[test]
fn submodule_section__lib_two_commits_ahead_on_main_with_a_file_changed_has_an_arrow_up_with_2_main_and_a_pencil_with_1()
 {
    let lib = lib();
    assert_eq!(listed(vec![lib.clone()]), Some(vec![String::from("lib")]));
    assert_eq!(
        status_marks(&status(&lib)),
        [(Some(2), icons::TO_PUSH, ui::Tone::Warning)]
    );
    let checkout = lib.checkout.as_ref();
    assert_eq!(checkout.and_then(|checkout| checkout.branch.as_deref()), Some("main"));
    assert_eq!(checkout.map(|checkout| checkout.uncommitted), Some(1));
}

#[test]
fn submodule_section__vendor_a_b_shows_as_b_under_vendor_a() {
    let submodules = vec![on_it("vendor/a", None), on_it("vendor/a/b", Some("vendor/a"))];
    assert_eq!(
        listed(submodules),
        Some(vec![String::from("vendor/a"), String::from("b")])
    );
}

#[test]
fn submodule_section__the_tooltip_of_lib_has_its_url_and_both_commits() {
    let mut lib = lib();
    lib.staged = Some(commit('4'));
    assert_eq!(
        tooltip(&lib),
        "lib\nhttps://example.com/acme/lib.git\n2 commits ahead of the recorded commit 1111111\nChecked out: main at \
         2222222\nStaged as the recorded commit: 4444444\n1 file not committed"
    );
}

#[test]
fn submodule_section__a_repository_without_submodules_has_no_submodules_list() {
    assert_eq!(listed(Vec::new()), None);
}

#[test]
fn submodule_section__the_menu_has_the_operations_offered_in_the_order_of_the_card() {
    let labels = |submodule: &Submodule| -> Vec<Vec<String>> {
        menu_groups(submodule, "windows")
            .iter()
            .map(|group| group.iter().map(|entry| entry.label.clone()).collect())
            .collect()
    };
    assert_eq!(
        labels(&lib()),
        [
            vec![
                "Open in new window",
                "Open in terminal",
                "Reveal in File Explorer",
                "Show in Gitardo"
            ],
            vec![
                "Update to the recorded commit 1111111",
                "Stage 2222222 as the recorded commit",
                "Update to the latest of its remote",
                "Synchronize URLs from .gitmodules",
            ],
            vec!["Copy path", "Copy URL"],
            vec!["Deinitialize\u{2026}", "Remove\u{2026}"],
        ]
    );
    let mut empty = on_it("lib", None);
    empty.checkout = None;
    assert_eq!(
        labels(&empty),
        [
            vec!["Initialize", "Synchronize URLs from .gitmodules"],
            vec!["Copy path"],
            vec!["Remove\u{2026}"],
        ]
    );
}

#[test]
fn submodule_reveal__the_entry_is_named_after_the_file_manager_of_each_system() {
    let words = crate::text::submodule::reveal;
    assert_eq!(words("windows"), "Reveal in File Explorer");
    assert_eq!(words("macos"), "Reveal in Finder");
    assert_eq!(words("linux"), "Open containing folder");
}
