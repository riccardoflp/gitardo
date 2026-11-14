//! The Stashes list of the branch rail as the store keeps it (`stash.pane`): the stashes unfolded, the files read of
//! each, kept while it is listed since a stash does not change, and **Show Stashes**, which shows the list.

use std::collections::{BTreeMap, BTreeSet};

use git::read::details::CommitRequest;
use model::CommitId;
use model::details::CommitDetail;
use views::RailList;
use views::graph::GraphIntent;
use views::refs::StashFilesRead;

use super::message::{Effect, JobId, JobKind};
use super::state::State;

/// The stashes unfolded in the Stashes list, and what was read of their files.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StashesList {
    /// The stashes unfolded, by their commits.
    pub unfolded: BTreeSet<CommitId>,
    /// What was read of the files of each stash unfolded once, by its commit.
    pub files: BTreeMap<CommitId, StashFilesRead>,
    /// The reads of the files running, with the stash each is of.
    pub jobs: BTreeMap<JobId, CommitId>,
}

/// The arrow of a stash: unfolded, its files are read unless they were read already; folded, they are kept.
pub fn toggle(state: &mut State, commit: CommitId) -> Vec<Effect> {
    let list = &mut state.rail.stashes;
    if list.unfolded.remove(&commit) {
        return Vec::new();
    }
    let _new = list.unfolded.insert(commit.clone());
    if matches!(
        list.files.get(&commit),
        Some(StashFilesRead::Loading | StashFilesRead::Read(_))
    ) {
        return Vec::new();
    }
    let Some(folder) = state.shown_folder().cloned() else {
        return Vec::new();
    };
    let kind = JobKind::ReadDetails {
        folder,
        request: CommitRequest::Commit(commit.clone()),
    };
    let job = super::workspace::start_job(state, kind);
    let list = &mut state.rail.stashes;
    let _loading = list.files.insert(commit.clone(), StashFilesRead::Loading);
    let _new = list.jobs.insert(job.id, commit);
    vec![Effect::Start(job)]
}

/// Whether a read of details is the read of the files of a stash of the list.
#[must_use]
pub fn is_files_read(state: &State, job: JobId) -> bool {
    state.rail.stashes.jobs.contains_key(&job)
}

/// The files of a stash read, kept unless the stash is gone since; a stash that is not in the repository any more
/// counts as one git could not read.
pub fn files_read(state: &mut State, job: JobId, result: Result<Option<Box<CommitDetail>>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(commit) = state.rail.stashes.jobs.remove(&job) else {
        return Vec::new();
    };
    let read = match result {
        Ok(Some(detail)) => StashFilesRead::Read(detail),
        Ok(None) => StashFilesRead::Failed,
        Err(error) => {
            state.last_error = Some(error);
            StashFilesRead::Failed
        }
    };
    if let Some(files) = state.rail.stashes.files.get_mut(&commit) {
        *files = read;
    }
    Vec::new()
}

/// Forgets what was read and unfolded of the stashes that are no longer listed, once the repository is read again.
pub fn forget_gone(state: &mut State) {
    let Some(read) = state.shown_read() else {
        return;
    };
    let listed: BTreeSet<CommitId> = read.state.stashes.iter().map(|stash| stash.commit.clone()).collect();
    let list = &mut state.rail.stashes;
    list.unfolded.retain(|commit| listed.contains(commit));
    list.files.retain(|commit, _| listed.contains(commit));
}

/// **Show Stashes**: the timeline shown with the branch rail, the filter box of the rail emptied, and the Stashes list
/// open, newest stash first.
pub fn show(state: &mut State) -> Vec<Effect> {
    let lists = &mut state.rail.lists;
    if !lists.is_open(&RailList::Stashes) {
        let _turned = lists.turned.insert(RailList::Stashes);
    }
    state.rail.typed.clear();
    let mut effects = super::pages::show_timeline(state);
    if super::timeline::workspace_panes(state).is_rail_hidden {
        effects.extend(super::timeline::reduce_graph(state, GraphIntent::ToggleReferences));
    }
    effects
}
