//! The entries of the operations in the menus of references and commits, and what a double click on a branch does.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::CommitId;
use model::Timestamp;
use model::branch::OpenedReference;
use model::contract::{FeatureId, OpenIn, Target};
use model::state::{Branch, Head, RemoteBranch, RepositoryState, Tag, Upstream, Worktree};

use super::{
    OperationEntry, ReferenceMenus, branch_entries, chosen, commit_entries, open_intent, remote_entries, tag_entries,
};
use crate::feature::FeatureIntent;
use crate::intent::Intent;

/// The hash of a commit named by a letter.
fn id(letter: char) -> CommitId {
    CommitId::parse(&letter.to_string().repeat(40)).unwrap_or_else(|| panic!("{letter} makes no hash"))
}

/// A local branch at a commit, following `origin/<name>` with some commits of its own and some to take.
fn branch(name: &str, at: char, (ahead, behind): (u32, u32)) -> Branch {
    Branch {
        name: name.to_owned(),
        commit: id(at),
        upstream: Some(Upstream {
            reference: format!("refs/remotes/origin/{name}"),
            remote: String::from("origin"),
            remote_branch: name.to_owned(),
            ahead,
            behind,
            is_gone: false,
        }),
        is_checked_out: name == "main",
        date: Timestamp(0),
    }
}

/// `main` checked out at `a`; `feature` at `b`, two commits behind `origin/feature`; `topic` at `c`, one ahead of
/// `origin/topic` and two behind; `linked` checked out in another worktree with nothing to commit; the tags `v1` at
/// `a` and `v0` at `b`; the remote `origin`.
fn repository() -> RepositoryState {
    let remote = |name: &str, at| RemoteBranch {
        name: format!("origin/{name}"),
        remote: String::from("origin"),
        commit: id(at),
        date: Timestamp(0),
    };
    let tag = |name: &str, at| Tag {
        name: name.to_owned(),
        commit: id(at),
        date: Timestamp(0),
    };
    RepositoryState {
        head: Head {
            commit: Some(id('a')),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![
            branch("main", 'a', (0, 0)),
            branch("feature", 'b', (0, 2)),
            branch("topic", 'c', (1, 2)),
            branch("linked", 'd', (0, 1)),
        ],
        remote_branches: vec![remote("feature", 'e'), remote("topic", 'f'), remote("new", 'a')],
        tags: vec![tag("v1", 'a'), tag("v0", 'b')],
        worktrees: vec![Worktree {
            folder: PathBuf::from("app-linked"),
            commit: Some(id('d')),
            branch: Some(String::from("refs/heads/linked")),
            is_main: false,
            is_current: false,
            is_missing: false,
            uncommitted: Some(0),
        }],
        remotes: vec![String::from("origin")],
        ..RepositoryState::default()
    }
}

fn menus(repository: &RepositoryState, open_in: OpenIn) -> ReferenceMenus<'_> {
    ReferenceMenus { repository, open_in }
}

/// The words of some entries, with `(off)` after those greyed out.
fn labels(entries: &[OperationEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| match entry.run {
            Some(_) => entry.label.clone(),
            None => format!("{} (off)", entry.label),
        })
        .collect()
}

/// The entries of a local branch of the repository, by its name.
fn of_branch(repository: &RepositoryState, open_in: OpenIn, name: &str) -> Vec<String> {
    let branch = repository.branches.iter().find(|branch| branch.name == name);
    labels(
        &branch
            .map(|branch| branch_entries(menus(repository, open_in), branch))
            .unwrap_or_default(),
    )
}

#[test]
fn branch_checkout__the_menu_of_the_branch_checked_out_has_no_check_out_entry() {
    let repository = repository();
    assert_eq!(
        of_branch(&repository, OpenIn::Checkout, "main"),
        ["Create branch here\u{2026}", "Rename\u{2026}"]
    );
}

#[test]
fn branch_checkout__the_menu_of_a_branch_another_worktree_has_has_its_entries_and_no_check_out() {
    let repository = repository();
    assert_eq!(
        of_branch(&repository, OpenIn::Checkout, "linked"),
        [
            "Open worktree in new window (off)",
            "Remove worktree\u{2026} (off)",
            "Remove worktree and delete branch\u{2026} (off)",
            "Create branch here\u{2026}",
            "Fast-forward to origin/linked",
            "Rename branch\u{2026}",
        ]
    );
}

#[test]
fn branch_checkout__with_open_in_set_to_worktree_the_double_click_opens_a_worktree_and_checks_out_nothing() {
    let repository = repository();
    let opened = |open_in| open_intent(menus(&repository, open_in), OpenedReference::Branch("feature"));
    let run = |feature| {
        Some(Intent::Feature(FeatureIntent::Run(
            FeatureId(feature),
            Target::Branch(String::from("feature")),
        )))
    };
    assert_eq!(opened(OpenIn::Checkout), run("branch.checkout"));
    assert_eq!(opened(OpenIn::Worktree), run("worktree.open-in-worktree"));
}

#[test]
fn branch_open__with_open_in_set_to_worktree_the_menu_of_feature_starts_with_open_in_worktree() {
    let repository = repository();
    let checkout = of_branch(&repository, OpenIn::Checkout, "feature");
    assert_eq!(
        checkout.get(..2),
        Some(&["Check out feature".to_owned(), "Open in worktree (off)".to_owned()][..])
    );
    let worktree = of_branch(&repository, OpenIn::Worktree, "feature");
    assert_eq!(
        worktree.get(..2),
        Some(&["Open in worktree (off)".to_owned(), "Check out feature".to_owned()][..])
    );
}

#[test]
fn branch_fast_forward__offered_to_a_branch_only_behind_and_not_to_one_with_a_commit_of_its_own() {
    let repository = repository();
    assert!(
        of_branch(&repository, OpenIn::Checkout, "feature").contains(&String::from("Fast-forward to origin/feature"))
    );
    assert!(
        !of_branch(&repository, OpenIn::Checkout, "topic")
            .iter()
            .any(|label| label.starts_with("Fast-forward"))
    );
}

#[test]
fn branch_fast_forward__a_branch_whose_worktree_has_a_file_not_committed_has_no_such_entry() {
    let mut repository = repository();
    for worktree in &mut repository.worktrees {
        worktree.uncommitted = Some(1);
    }
    assert!(
        !of_branch(&repository, OpenIn::Checkout, "linked")
            .iter()
            .any(|label| label.starts_with("Fast-forward"))
    );
}

#[test]
fn branch_checkout_remote_and_rebase__the_entry_is_there_only_when_the_branch_has_commits_of_its_own_and_is_not_checked_out()
 {
    let mut repository = repository();
    let of_remote = |repository: &RepositoryState, name: &str| {
        let remote = repository.remote_branches.iter().find(|remote| remote.name == name);
        labels(
            &remote
                .map(|remote| remote_entries(menus(repository, OpenIn::Checkout), remote))
                .unwrap_or_default(),
        )
    };
    let rebase = String::from("Check out topic and rebase it onto this");
    assert!(of_remote(&repository, "origin/topic").contains(&rebase));
    assert!(
        !of_remote(&repository, "origin/feature")
            .iter()
            .any(|label| label.contains("rebase")),
        "only behind"
    );
    for branch in &mut repository.branches {
        branch.is_checked_out = branch.name == "topic";
    }
    assert!(!of_remote(&repository, "origin/topic").contains(&rebase), "checked out");
    assert_eq!(
        of_remote(&repository, "origin/new"),
        [
            "Check out as local branch",
            "Open in worktree (off)",
            "Create branch here\u{2026}",
            "Delete from remote",
        ]
    );
}

#[test]
fn branch_checkout_detached__the_menu_of_the_commit_checked_out_has_no_such_entry() {
    let repository = repository();
    let of_commit = |letter| labels(&commit_entries(menus(&repository, OpenIn::Checkout), &id(letter)));
    assert_eq!(
        of_commit('a'),
        ["Create branch here\u{2026}", "Create tag here\u{2026}"]
    );
    let entries = commit_entries(menus(&repository, OpenIn::Checkout), &id('b'));
    assert_eq!(
        labels(&entries).last().map(String::as_str),
        Some("Check out commit (detached)")
    );
    assert_eq!(
        chosen(&entries, 2),
        Some(Intent::Feature(FeatureIntent::Run(
            FeatureId("branch.checkout-detached"),
            Target::Commit(id('b'))
        )))
    );
}

#[test]
fn branch_delete__the_branch_checked_out_has_no_delete_entry() {
    let repository = repository();
    assert!(!of_branch(&repository, OpenIn::Checkout, "main").contains(&String::from("Delete")));
    let feature = of_branch(&repository, OpenIn::Checkout, "feature");
    assert_eq!(feature.last().map(String::as_str), Some("Delete"));
}

/// The entries of a tag of the repository, by its name.
fn of_tag(repository: &RepositoryState, name: &str) -> Vec<String> {
    let tag = repository.tags.iter().find(|tag| tag.name == name);
    labels(
        &tag.map(|tag| tag_entries(menus(repository, OpenIn::Checkout), tag))
            .unwrap_or_default(),
    )
}

#[test]
fn tag_push__a_repository_with_no_remote_has_no_push_tag_entry() {
    let mut repository = repository();
    assert!(of_tag(&repository, "v0").contains(&String::from("Push tag")));
    repository.remotes.clear();
    assert_eq!(
        of_tag(&repository, "v0"),
        ["Check out tag (detached)", "Create branch here\u{2026}", "Delete"]
    );
}

#[test]
fn tag_checkout__a_tag_on_the_commit_checked_out_has_no_such_entry() {
    let repository = repository();
    assert_eq!(
        of_tag(&repository, "v1"),
        ["Create branch here\u{2026}", "Push tag", "Delete"]
    );
}
