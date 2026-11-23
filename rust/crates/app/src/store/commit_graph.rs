//! The notice of git's commit-graph file (`upkeep.offer-commit-graph`): once the history of the repository shown is
//! read whole and counts 10,000 commits or more, while git keeps no commit-graph file of it and would read one, the
//! status bar says that git can read it about twice as fast with one. It is said once each time the window opens the
//! repository; **Not for this repository** is kept in `state.json` under `commitGraphDeclined`, and it is said no more.

use std::path::{Path, PathBuf};

use model::state::RepositoryState;
use model::upkeep::offers_commit_graph;

use super::message::Effect;
use super::state::State;
use crate::persisted::same_folder;

/// What the window knows of the notice of the commit-graph file.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CommitGraphOffer {
    /// The repositories it was said for since the window opened, by the folder of their main worktree: once each.
    pub said: Vec<PathBuf>,
    /// The repository it is said for, until it is put away.
    pub notice: Option<PathBuf>,
}

/// The repository a state is of, by the folder of its main worktree, which its worktrees share: what the notice is said
/// once for, and what **Not for this repository** remembers.
#[must_use]
pub fn repository_of(repository: &RepositoryState) -> &Path {
    repository
        .worktrees
        .iter()
        .find(|worktree| worktree.is_main)
        .map_or(repository.folder.as_path(), |worktree| worktree.folder.as_path())
}

/// Says the notice for the repository shown once its history is read whole, when it counts 10,000 commits or more and
/// git keeps no commit-graph file of it and would read one; once each time the window opens it, and never for a
/// repository declined.
pub fn follow(state: &mut State) {
    let timeline = &state.timeline;
    let is_read_whole = timeline.stream.is_none()
        && timeline.more_job.is_none()
        && timeline.narrow_job.is_none()
        && !timeline.is_reading_again;
    let Some(read) = state.shown_read().filter(|read| {
        is_read_whole
            && timeline.folder.as_deref() == Some(read.folder.as_path())
            && offers_commit_graph(timeline.read_rows().len(), read.state.commit_graph)
    }) else {
        return;
    };
    let repository = repository_of(&read.state).to_path_buf();
    let is_known = |folders: &[PathBuf]| folders.iter().any(|folder| same_folder(folder, &repository));
    if is_known(&state.commit_graph.said) || is_known(&state.persisted.commit_graph_declined) {
        return;
    }
    state.commit_graph.said.push(repository.clone());
    state.commit_graph.notice = Some(repository);
}

/// Whether the notice is said now: for the repository shown, while git still keeps no commit-graph file of it.
#[must_use]
pub fn is_said(state: &State) -> bool {
    let Some(notice) = &state.commit_graph.notice else {
        return false;
    };
    state
        .shown_read()
        .is_some_and(|read| read.state.commit_graph.is_missing() && same_folder(repository_of(&read.state), notice))
}

/// Puts the notice shown away: a notice of the window, or that of git's commit-graph file, which **Not for this
/// repository** says no more for its repository.
pub fn notice(state: &mut State, intent: &views::Intent) -> Vec<Effect> {
    match intent {
        views::Intent::DismissCommitGraph => dismiss(state),
        views::Intent::DeclineCommitGraph => decline(state),
        _ => {
            state.notice = None;
            Vec::new()
        }
    }
}

/// **×** and **Write it**: the notice is put away, and said again the next time the window opens the repository.
fn dismiss(state: &mut State) -> Vec<Effect> {
    state.commit_graph.notice = None;
    Vec::new()
}

/// **Not for this repository**: the notice is put away and said no more for its repository, which `state.json` keeps.
fn decline(state: &mut State) -> Vec<Effect> {
    let Some(repository) = state.commit_graph.notice.take() else {
        return Vec::new();
    };
    let declined = &mut state.persisted.commit_graph_declined;
    if !declined.iter().any(|folder| same_folder(folder, &repository)) {
        declined.push(repository);
    }
    vec![Effect::WritePersisted(state.persisted.clone())]
}
