//! The branches the timeline shows (`filter.picker`, `filter.show-only`, `filter.show-all`, `filter.remember`):
//! chosen, checked against the references the repository has, kept for it between runs, and read. Narrowing reads only
//! the first page of the new history: the state of the repository, as last read, stays.

use std::path::{Path, PathBuf};

use git::read::graph::CommitPage;
use model::filter::{BranchesShown, StoredValue, checked, remember};
use model::refs::References;

use super::super::message::{Effect, JobId, JobKind};
use super::super::state::State;
use super::{commits_request, paging, take_read};
use crate::persisted::rail::shown_of;
use crate::persisted::timeline::remember as remember_last;
use crate::persisted::{RepositoryShown, same_folder};

/// The branches a read of a repository asks for, before the worker checks them against its references: the ones the
/// timeline shows for the repository shown, and those kept for another.
#[must_use]
pub fn kept_shown(state: &State, folder: &Path) -> Option<BranchesShown> {
    if state
        .timeline
        .folder
        .as_deref()
        .is_some_and(|shown| same_folder(shown, folder))
    {
        return state.timeline.shown.clone();
    }
    match shown_of(&state.persisted.branches_shown, folder)? {
        StoredValue::References(names) => (!names.is_empty()).then(|| BranchesShown::References(names.clone())),
        stored => remember(stored, &References::default()),
    }
}

/// Shows some branches, none being everything: checked against the references of the repository as last read, kept
/// for it, and the first page of their history read. The same choice again changes nothing.
pub fn show(state: &mut State, shown: Option<&BranchesShown>) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let folder = read.folder.clone();
    let shown = checked(shown, &References::of(&read.state));
    if state.timeline.shown == shown {
        return Vec::new();
    }
    state.timeline.shown = shown;
    let mut effects = keep_shown(state, &folder);
    effects.extend(narrow(state, folder));
    effects
}

/// Keeps the branches the timeline shows for a repository, when they changed; everything shown keeps nothing.
pub fn keep_shown(state: &mut State, folder: &Path) -> Vec<Effect> {
    let stored = state.timeline.shown.as_ref().map(StoredValue::of);
    if shown_of(&state.persisted.branches_shown, folder) == stored.as_ref() {
        return Vec::new();
    }
    match stored {
        Some(shown) => {
            let entry = RepositoryShown {
                folder: folder.to_path_buf(),
                shown,
            };
            remember_last(&mut state.persisted.branches_shown, entry, |kept| {
                same_folder(&kept.folder, folder)
            });
        }
        None => state
            .persisted
            .branches_shown
            .retain(|kept| !same_folder(&kept.folder, folder)),
    }
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// Reads the first page of the history of the branches shown, the rows listed dimmed meanwhile; while the repository
/// itself is being read, that read starts again with them instead.
fn narrow(state: &mut State, folder: PathBuf) -> Vec<Effect> {
    let is_read_running = state
        .jobs
        .values()
        .any(|kind| matches!(kind, JobKind::ReadRepository { .. }));
    if is_read_running {
        return paging::read_again(state);
    }
    let mut effects = paging::cancel_more(state);
    let Some(read) = state.shown_read() else {
        return effects;
    };
    let request = commits_request(&read.state, &paging::read_request(state, 1));
    let job = paging::start(state, JobKind::ReadTimeline { folder, request });
    state.timeline.narrow_job = Some(job.id);
    state.timeline.is_reading_again = true;
    effects.push(Effect::Start(job));
    effects
}

/// Takes the first page of the history of the branches shown, with the draft when they have what is checked out; the
/// commit selected is brought into sight again.
pub fn timeline_read(state: &mut State, job: JobId, result: Result<CommitPage, String>) -> Vec<Effect> {
    let Some(JobKind::ReadTimeline { .. }) = state.jobs.remove(&job) else {
        return Vec::new();
    };
    if state.timeline.narrow_job != Some(job) {
        return Vec::new();
    }
    state.timeline.narrow_job = None;
    let page = match result {
        Ok(page) => page,
        Err(error) => {
            state.timeline.is_reading_again = false;
            state.last_error = Some(error);
            return Vec::new();
        }
    };
    let Some(read) = state.shown_read().cloned() else {
        return Vec::new();
    };
    let shown = state.timeline.shown.clone();
    let read_at = state.timeline.read_at;
    take_read(
        state,
        (&read.folder, &read.state),
        (page.rows, shown),
        (page.has_more, 1, read_at),
    );
    if state.timeline.selected.is_some() {
        let serial = state.timeline.reveal.map_or(1, |(serial, _)| serial + 1);
        state.timeline.reveal = Some((serial, ui::RevealPlace::Center));
    }
    Vec::new()
}
