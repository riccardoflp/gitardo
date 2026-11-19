//! What the tests of the operations of worktrees share: a repository with a main worktree, a worktree of
//! `feature/login` and one whose folder is gone.

use std::path::PathBuf;

use model::contract::{Context, Target};
use model::state::{RepositoryState, Worktree};

use crate::testing::{branch, commit, context, state};

/// The main worktree, the folder the views show.
pub const MAIN: &str = "/code/app";
/// The worktree of `feature/login`.
pub const LOGIN: &str = "/code/app.worktrees/feature/login";
/// The worktree of `gone`, whose folder is missing.
pub const GONE: &str = "/code/app.worktrees/gone";

/// A worktree.
pub fn worktree(folder: &str, branch: Option<&str>, (is_main, is_current, is_missing): (bool, bool, bool)) -> Worktree {
    Worktree {
        folder: PathBuf::from(folder),
        commit: Some(commit('b')),
        branch: branch.map(|name| format!("refs/heads/{name}")),
        is_main,
        is_current,
        is_missing,
        locked: None,
        uncommitted: (!is_missing).then_some(0),
    }
}

/// The repository in `/code/app` on `main`, with the worktrees of `feature/login` and of `gone`.
pub fn repository() -> RepositoryState {
    RepositoryState {
        folder: PathBuf::from(MAIN),
        branches: vec![
            branch("main", 'a', true),
            branch("feature/login", 'b', false),
            branch("gone", 'b', false),
            branch("release", 'c', false),
        ],
        worktrees: vec![
            worktree(MAIN, Some("main"), (true, true, false)),
            worktree(LOGIN, Some("feature/login"), (false, false, false)),
            worktree(GONE, Some("gone"), (false, false, true)),
        ],
        ..state()
    }
}

/// The context of a feature invoked on `target` in [`repository`], with `/home/someone` as the home folder.
pub fn with_worktrees(target: Target) -> Context {
    let mut context = context(repository(), target);
    context.settings.worktrees.home = PathBuf::from("/home/someone");
    context
}
