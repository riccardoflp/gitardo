//! The **…** of the Worktrees list of the branch rail (`branch.pane`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::CommitId;
use model::contract::OpenIn;
use model::state::{RepositoryState, Worktree};

use super::ReferenceMenus;
use crate::worktree::{WorktreeWindow, list_entries};

/// The labels of the **…** of the Worktrees list of a repository with a worktree whose folder is missing or not.
fn labels_with(is_missing: bool) -> Vec<String> {
    let repository = RepositoryState {
        worktrees: vec![Worktree {
            folder: PathBuf::from("/code/app.worktrees/fix"),
            commit: CommitId::parse(&"a".repeat(40)),
            branch: Some(String::from("refs/heads/fix")),
            is_main: false,
            is_current: false,
            is_missing,
            uncommitted: None,
        }],
        ..RepositoryState::default()
    };
    let menus = ReferenceMenus {
        repository: &repository,
        open_in: OpenIn::Checkout,
        window: WorktreeWindow::default(),
        github: crate::github::GitHubShown::default(),
    };
    list_entries(menus).into_iter().map(|entry| entry.label).collect()
}

#[test]
fn branch_pane__the_more_of_the_worktrees_list_has_prune_missing_worktrees_only_while_a_folder_is_missing() {
    let prune = String::from("Prune missing worktrees");
    assert!(labels_with(true).contains(&prune));
    assert!(!labels_with(false).contains(&prune));
}
