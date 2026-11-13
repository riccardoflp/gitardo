//! The menus of the worktrees, how one stands in the Worktrees list, and the menus of the branches other worktrees have.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::{Path, PathBuf};

use model::CommitId;
use model::Timestamp;
use model::branch::OpenedReference;
use model::contract::{FeatureId, OpenIn, Target};
use model::state::{Branch, Head, RepositoryState, Upstream, Worktree};

use super::{WorktreeWindow, label, list_entries, open_intent, standing, tooltip, worktree_entries};
use crate::branch::{OperationEntry, ReferenceMenus, branch_entries, open_intent as branch_open_intent};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::worktree as words;

const MAIN: &str = "/code/app";
const LOGIN: &str = "/code/app.worktrees/feature/login";
const GONE: &str = "/code/app.worktrees/gone";

fn id(letter: char) -> CommitId {
    CommitId::parse(&letter.to_string().repeat(40)).unwrap_or_else(|| panic!("{letter} makes no hash"))
}

fn branch(name: &str, (ahead, behind): (u32, u32), is_checked_out: bool) -> Branch {
    Branch {
        name: name.to_owned(),
        commit: id('a'),
        upstream: Some(Upstream {
            reference: format!("refs/remotes/origin/{name}"),
            remote: String::from("origin"),
            remote_branch: format!("refs/heads/{name}"),
            ahead,
            behind,
            is_gone: false,
        }),
        is_checked_out,
        date: Timestamp(0),
    }
}

fn worktree(folder: &str, branch: &str, (is_main, is_current): (bool, bool), uncommitted: usize) -> Worktree {
    Worktree {
        folder: PathBuf::from(folder),
        commit: Some(id('a')),
        branch: Some(format!("refs/heads/{branch}")),
        is_main,
        is_current,
        is_missing: false,
        uncommitted: Some(uncommitted),
    }
}

/// `/code/app` on `main`, the worktree of `feature/login` two ahead of its upstream and one behind with three files
/// not committed, and the worktree of `gone`, whose folder is missing.
fn repository() -> RepositoryState {
    RepositoryState {
        folder: PathBuf::from(MAIN),
        head: Head {
            commit: Some(id('a')),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![
            branch("main", (0, 0), true),
            branch("feature/login", (2, 1), false),
            branch("gone", (0, 0), false),
            branch("release", (0, 0), false),
        ],
        worktrees: vec![
            worktree(MAIN, "main", (true, true), 0),
            worktree(LOGIN, "feature/login", (false, false), 3),
            Worktree {
                is_missing: true,
                uncommitted: None,
                ..worktree(GONE, "gone", (false, false), 0)
            },
        ],
        remotes: vec![String::from("origin")],
        ..RepositoryState::default()
    }
}

/// The same repository, seen from the worktree of `feature/login` shown in the window of `/code/app`.
fn from_login() -> RepositoryState {
    let mut repository = repository();
    for worktree in &mut repository.worktrees {
        worktree.is_current = worktree.folder == Path::new(LOGIN);
    }
    for branch in &mut repository.branches {
        branch.is_checked_out = branch.name == "feature/login";
    }
    repository
}

fn menus<'a>(repository: &'a RepositoryState, window: WorktreeWindow<'static>) -> ReferenceMenus<'a> {
    ReferenceMenus {
        repository,
        open_in: OpenIn::Checkout,
        window,
        github: crate::github::GitHubShown::default(),
    }
}

fn labels(entries: &[OperationEntry]) -> Vec<String> {
    entries.iter().map(|entry| entry.label.clone()).collect()
}

fn of_worktree(repository: &RepositoryState, window: WorktreeWindow<'static>, folder: &str) -> Vec<String> {
    let worktree = repository
        .worktrees
        .iter()
        .find(|worktree| worktree.folder == Path::new(folder));
    labels(
        &worktree
            .map(|worktree| worktree_entries(menus(repository, window), worktree))
            .unwrap_or_default(),
    )
}

fn of_branch(repository: &RepositoryState, window: WorktreeWindow<'static>, name: &str) -> Vec<String> {
    let branch = repository.branches.iter().find(|branch| branch.name == name);
    labels(
        &branch
            .map(|branch| branch_entries(menus(repository, window), branch))
            .unwrap_or_default(),
    )
}

#[test]
fn worktree_section__a_worktree_ahead_and_behind_with_files_shows_how_it_stands_and_its_count() {
    let repository = repository();
    let login = repository.worktrees.get(1).unwrap();
    assert_eq!(label(&repository, WorktreeWindow::default(), login), "feature/login");
    assert_eq!(standing(&repository, login), "feature/login \u{2191}2 \u{2193}1");
    let said = tooltip(&repository, login);
    assert!(said.contains(LOGIN) && said.contains("3 files not committed"), "{said}");
}

#[test]
fn worktree_section__a_missing_worktree_shows_missing_and_a_double_click_does_nothing() {
    let repository = repository();
    let gone = repository.worktrees.get(2).unwrap();
    assert_eq!(standing(&repository, gone), "missing");
    assert_eq!(open_intent(gone), None);
    let login = repository.worktrees.get(1).unwrap();
    assert_eq!(
        open_intent(login),
        Some(Intent::Feature(FeatureIntent::Run(
            FeatureId("worktree.open"),
            Target::Worktree(PathBuf::from(LOGIN))
        )))
    );
}

#[test]
fn worktree_section__the_count_of_files_comes_from_each_read_of_the_list() {
    let mut repository = repository();
    if let Some(login) = repository.worktrees.get_mut(1) {
        login.uncommitted = Some(4);
    }
    let said = tooltip(&repository, repository.worktrees.get(1).unwrap());
    assert!(said.contains("4 files not committed"), "{said}");
}

#[test]
fn worktree_section__the_main_and_the_current_worktree_have_no_rename_nor_remove() {
    let repository = repository();
    let main = of_worktree(&repository, WorktreeWindow::default(), MAIN);
    for absent in [
        words::RENAME,
        words::REMOVE,
        words::OPEN_IN_NEW_WINDOW,
        words::SHOW_IN_GITARDO,
    ] {
        assert!(!main.contains(&absent.to_owned()), "{absent} in {main:?}");
    }
    let login = of_worktree(&repository, WorktreeWindow::default(), LOGIN);
    for present in [
        words::RENAME,
        words::REMOVE,
        words::OPEN_IN_NEW_WINDOW,
        words::SHOW_IN_GITARDO,
    ] {
        assert!(login.contains(&present.to_owned()), "{present} not in {login:?}");
    }
}

#[test]
fn worktree_section__prune_is_in_the_menus_only_while_a_worktree_is_missing() {
    let mut repository = repository();
    let list = labels(&list_entries(menus(&repository, WorktreeWindow::default())));
    assert!(list.contains(&words::PRUNE.to_owned()));
    assert!(of_worktree(&repository, WorktreeWindow::default(), LOGIN).contains(&words::PRUNE.to_owned()));
    repository.worktrees.truncate(2);
    let list = labels(&list_entries(menus(&repository, WorktreeWindow::default())));
    assert!(!list.contains(&words::PRUNE.to_owned()));
    assert!(!of_worktree(&repository, WorktreeWindow::default(), LOGIN).contains(&words::PRUNE.to_owned()));
}

#[test]
fn worktree_open_in_editor__with_no_external_editor_the_entry_is_not_in_the_menus() {
    let repository = repository();
    let without = of_worktree(&repository, WorktreeWindow::default(), LOGIN);
    assert!(!without.contains(&words::OPEN_IN_EDITOR.to_owned()));
    let with = WorktreeWindow {
        has_editor: true,
        ..WorktreeWindow::default()
    };
    assert!(of_worktree(&repository, with, LOGIN).contains(&words::OPEN_IN_EDITOR.to_owned()));
    let branch = of_branch(&repository, WorktreeWindow::default(), "feature/login");
    assert!(!branch.contains(&words::OPEN_WORKTREE_IN_EDITOR.to_owned()));
}

#[test]
fn worktree_create_from_branch__the_entry_is_not_on_the_branch_checked_out_nor_on_one_another_worktree_has() {
    let repository = repository();
    let create = words::CREATE_WORKTREE.to_owned();
    assert!(of_branch(&repository, WorktreeWindow::default(), "release").contains(&create));
    assert!(!of_branch(&repository, WorktreeWindow::default(), "main").contains(&create));
    assert!(!of_branch(&repository, WorktreeWindow::default(), "feature/login").contains(&create));
}

#[test]
fn worktree_rename__the_entry_is_not_offered_on_the_main_worktree_nor_on_the_folder_of_the_window() {
    let repository = from_login();
    let window = WorktreeWindow {
        folder: Some(Path::new(MAIN)),
        ..WorktreeWindow::default()
    };
    assert!(!of_worktree(&repository, window, MAIN).contains(&words::RENAME.to_owned()));
    let mut other = repository.clone();
    other
        .worktrees
        .push(worktree("/code/app.worktrees/fix", "release", (false, false), 0));
    let window = WorktreeWindow {
        folder: Some(Path::new("/code/app.worktrees/fix")),
        ..WorktreeWindow::default()
    };
    assert!(!of_worktree(&other, window, "/code/app.worktrees/fix").contains(&words::RENAME.to_owned()));
}

#[test]
fn worktree_show_in_gitardo__while_shown_the_folder_of_the_window_has_no_rename_nor_remove() {
    let repository = from_login();
    let window = WorktreeWindow {
        folder: Some(Path::new(MAIN)),
        ..WorktreeWindow::default()
    };
    let main = of_worktree(&repository, window, MAIN);
    assert!(
        main.contains(&words::SHOW_IN_GITARDO.to_owned()),
        "it goes back: {main:?}"
    );
    assert!(!main.contains(&words::RENAME.to_owned()) && !main.contains(&words::REMOVE.to_owned()));
}

#[test]
fn worktree_branch_in_another_worktree__its_menu_starts_with_opening_it_and_has_no_check_out_nor_delete() {
    let repository = repository();
    let entries = of_branch(&repository, WorktreeWindow::default(), "feature/login");
    assert_eq!(entries.first().map(String::as_str), Some("Open worktree in new window"));
    assert!(
        !entries
            .iter()
            .any(|entry| entry.starts_with("Check out") || entry == "Delete")
    );
}

#[test]
fn worktree_branch_in_another_worktree__a_double_click_opens_the_worktree_and_checks_nothing_out() {
    let repository = repository();
    let opened = branch_open_intent(
        menus(&repository, WorktreeWindow::default()),
        OpenedReference::Branch("feature/login"),
    );
    let Some(Intent::Feature(FeatureIntent::Run(feature, _))) = opened else {
        panic!("{opened:?}")
    };
    assert_eq!(feature, FeatureId("worktree.open"));
}

#[test]
fn worktree_branch_in_another_worktree__from_another_worktree_the_menu_of_main_has_no_remove_worktree() {
    let repository = from_login();
    let window = WorktreeWindow {
        folder: Some(Path::new(MAIN)),
        ..WorktreeWindow::default()
    };
    let main = of_branch(&repository, window, "main");
    assert_eq!(main.first().map(String::as_str), Some("Open worktree in new window"));
    assert!(
        !main
            .iter()
            .any(|entry| entry.starts_with("Remove worktree") || entry.starts_with("Rename worktree"))
    );
}
