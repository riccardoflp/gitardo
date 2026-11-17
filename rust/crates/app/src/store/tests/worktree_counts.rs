//! What the reads of the window counted in the other worktrees (`repo.read-state`, M15): kept by a read after a change,
//! counted again on **Refresh**.

use std::path::PathBuf;
use std::time::Duration;

use git::read::repo::{StateRequest, read_state_counting};
use git::read::worktree::CountedWorktrees;
use model::repo::Change;
use views::Intent;

use super::{act, folder, open, started, window};
use crate::store::{JobKind, Message, State, reduce};

/// What a read of a throwaway repository with another worktree counted there.
fn counted_in_a_repository() -> CountedWorktrees {
    let mut repository = testkit::Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let _added = repository.git(&["worktree", "add", "-b", "feature", "feature"]);
    let git = git::Git::new(repository.path());
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    let (_, counted) = read_state_counting(&git, &request, &CountedWorktrees::default()).unwrap();
    assert_ne!(counted, CountedWorktrees::default());
    counted
}

/// What the read of the repository an answer started was given of the read before.
fn counted_given(effects: &[crate::store::Effect]) -> Option<CountedWorktrees> {
    started(effects).into_iter().find_map(|(_, kind)| match kind {
        JobKind::ReadRepository { counted, .. } => Some(counted),
        _ => None,
    })
}

/// A window showing `/work/app`, with what a read before counted in its other worktrees.
fn showing(counted: &CountedWorktrees) -> State {
    let mut state = window();
    let app = folder("app");
    let _opened = open(&mut state, &app, &[(&app, std::slice::from_ref(&app))]);
    state.reads.counted = counted.clone();
    state
}

#[test]
fn repo_read_state__a_read_after_a_change_keeps_the_counts_of_the_other_worktrees_and_refresh_counts_them_again() {
    let counted = counted_in_a_repository();
    let mut state = showing(&counted);
    let _changed = reduce(
        &mut state,
        Message::Changed {
            changes: vec![Change::GitFolder(PathBuf::from("HEAD"))],
            at: Duration::from_secs(1),
        },
    );
    let woke = reduce(
        &mut state,
        Message::Wake {
            at: Duration::from_millis(1_400),
        },
    );
    assert_eq!(
        counted_given(&woke),
        Some(counted.clone()),
        "a read after a change keeps the counts"
    );

    let mut state = showing(&counted);
    let app = folder("app");
    let refreshed = act(&mut state, Intent::Refresh, &[(&app, std::slice::from_ref(&app))]);
    assert_eq!(
        counted_given(&refreshed),
        Some(CountedWorktrees::default()),
        "Refresh counts every worktree again"
    );
}
