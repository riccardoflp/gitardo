//! A branch another worktree has, in the branch rail and on its label in the timeline
//! (`worktree.branch-in-another-worktree`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::collections::HashMap;
use std::path::PathBuf;

use model::contract::OpenIn;
use model::graph::{GraphSettings, LabelKind, labels};
use model::refs::{References, SortOrder};
use model::state::{Branch, Head, RepositoryState, Worktree};
use model::{CommitId, Timestamp};

use super::ReferencesState;
use super::entry::{entry_tooltip, other_worktree};
use super::rows::{Entry, Marks};
use crate::intent::RailLists;

const LOGIN: &str = "/code/app.worktrees/feature/login";

fn id(letter: char) -> CommitId {
    CommitId::parse(&letter.to_string().repeat(40)).unwrap_or_else(|| panic!("{letter} makes no hash"))
}

fn branch(name: &str, commit: char, is_checked_out: bool) -> Branch {
    Branch {
        name: name.to_owned(),
        commit: id(commit),
        upstream: None,
        is_checked_out,
        date: Timestamp(0),
    }
}

fn worktree(folder: &str, branch: &str, commit: char, is_current: bool) -> Worktree {
    Worktree {
        folder: PathBuf::from(folder),
        commit: Some(id(commit)),
        branch: Some(format!("refs/heads/{branch}")),
        is_main: is_current,
        is_current,
        is_missing: false,
        uncommitted: Some(2),
    }
}

/// `/code/app` on `main`, and the worktree of `feature/login` with two files not committed.
fn repository() -> RepositoryState {
    RepositoryState {
        folder: PathBuf::from("/code/app"),
        head: Head {
            commit: Some(id('a')),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![branch("main", 'a', true), branch("feature/login", 'b', false)],
        worktrees: vec![
            worktree("/code/app", "main", 'a', true),
            worktree(LOGIN, "feature/login", 'b', false),
        ],
        ..RepositoryState::default()
    }
}

#[test]
fn worktree_branch_in_another_worktree__the_rail_and_the_timeline_mark_it_and_its_tooltip_names_the_worktree() {
    let repository = repository();
    let references = References {
        branches: repository.branches.clone(),
        ..References::default()
    };
    let (lists, colors) = (RailLists::default(), HashMap::new());
    let state = ReferencesState {
        repository: &repository,
        references: &references,
        lists: &lists,
        branch_colors: &colors,
        remote_colors: &colors,
        favorites: &[],
        recent: &[],
        typed: "",
        shown: None,
        follows_timeline: true,
        selected: None,
        sort: SortOrder::Name,
        open_in: OpenIn::Checkout,
        window: crate::worktree::WorktreeWindow::default(),
        stashes: super::stashes_tests::nothing_unfolded(),
    };
    let (main, login) = (branch("main", 'a', true), branch("feature/login", 'b', false));
    assert!(
        other_worktree(&state, &main).is_none(),
        "the branch checked out here has no worktree icon"
    );
    let found = other_worktree(&state, &login).map(|worktree| worktree.folder.clone());
    assert_eq!(found, Some(PathBuf::from(LOGIN)), "it is drawn with the worktree icon");
    let said = entry_tooltip(&state, &Entry::Branch(&login), Marks::default());
    assert!(
        said.contains("In the worktree") && said.contains("2 files not committed"),
        "{said}"
    );
    let found = labels(&repository, &GraphSettings::default());
    let on_label = found
        .get(&id('b'))
        .and_then(|labels| labels.first())
        .map(|label| &label.kind);
    let Some(LabelKind::Branch { worktree, .. }) = on_label else {
        panic!("{on_label:?}")
    };
    assert_eq!(
        worktree.as_deref(),
        Some(PathBuf::from(LOGIN).as_path()),
        "its label has the worktree icon"
    );
}
