//! The answers of a read of the repository shown: its state and its first commits at once (`repo.read-state`), or,
//! for a repository that shows nothing yet, the first step of its state with its first commits and then the rest
//! (`repo.read-first`).

use std::path::Path;
use std::sync::Arc;

use super::message::{Effect, JobId, Message};
use super::state::{Shown, State};
use super::{ReadResult, conflicts, feature, github, rail, reads, timeline, workspace};
use crate::persisted::same_folder;

/// Takes what a read of the repository sent.
pub(super) fn reduce(state: &mut State, message: Message) -> Vec<Effect> {
    match message {
        Message::RepositoryRead {
            job,
            folder,
            result,
            read_at,
        } => repository_read(state, job, &folder, (result, read_at)),
        Message::FirstRead {
            job,
            folder,
            result,
            read_at,
        } => first_read(state, job, &folder, (result, read_at)),
        Message::RestRead {
            job,
            folder,
            result,
            read_at,
        } => rest_read(state, job, &folder, (result, read_at)),
        // No other message is of a read of the repository.
        _ => Vec::new(),
    }
}

/// A read of the repository ended: shown, unless the schedule throws it away; then the read that follows it, if one
/// does. At `log.level` `debug`, what git said of a read that failed goes to the Command log.
fn repository_read(state: &mut State, job: JobId, folder: &Path, (result, read_at): (ReadResult, i64)) -> Vec<Effect> {
    let (is_shown, follow) = reads::finished(state, job);
    if let (Err(error), engine::LogLevel::Debug) = (&result, state.log.level()) {
        state.log.record_failure(model::Timestamp(read_at), error);
    }
    let mut effects = if is_shown {
        let mut effects = workspace::repository_read(state, job, folder, (result, read_at));
        effects.extend(feature::after_read(state));
        effects.extend(conflicts::after_read(state));
        effects.extend(github::after_read(state, read_at));
        effects
    } else {
        let _thrown_away = state.jobs.remove(&job);
        Vec::new()
    };
    effects.extend(follow);
    effects
}

/// The first step of a read in two steps came, with the first commits (`repo.read-first`): shown as a read is, but
/// what follows a read waits for the rest of the state, which the window notes it is waiting for.
fn first_read(state: &mut State, job: JobId, folder: &Path, (result, read_at): (ReadResult, i64)) -> Vec<Effect> {
    let (is_shown, follow) = reads::finished(state, job);
    if let (Err(error), engine::LogLevel::Debug) = (&result, state.log.level()) {
        state.log.record_failure(model::Timestamp(read_at), error);
    }
    let mut effects = if is_shown {
        let generation = state.next_generation;
        let effects = workspace::repository_read(state, job, folder, (result, read_at));
        if state.next_generation != generation {
            state.reads.rest = Some(job);
        }
        effects
    } else {
        let _thrown_away = state.jobs.remove(&job);
        Vec::new()
    };
    effects.extend(follow);
    effects
}

/// The rest of the state of a read in two steps came (`repo.read-state`): put in its place when the window still
/// waits for it, no row of the timeline moving, then what follows a read.
fn rest_read(state: &mut State, job: JobId, folder: &Path, (result, read_at): (RestResult, i64)) -> Vec<Effect> {
    if state.reads.rest != Some(job) {
        return Vec::new();
    }
    state.reads.rest = None;
    if let (Err(error), engine::LogLevel::Debug) = (&result, state.log.level()) {
        state.log.record_failure(model::Timestamp(read_at), error);
    }
    let Ok(read) = result else {
        // The rest could not be read: the repository is read again, whole, and says what is wrong if it still is.
        return reads::asked(state, model::repo::ReadCause::RepositoryChosen);
    };
    if !put_rest(state, folder, *read) {
        return Vec::new();
    }
    let mut effects = feature::after_read(state);
    effects.extend(conflicts::after_read(state));
    effects.extend(github::after_read(state, read_at));
    effects
}

/// What the second step of a read in two steps found, or what git said.
type RestResult = Result<Box<(model::state::RepositoryState, git::read::worktree::CountedWorktrees)>, String>;

/// Puts the rest of the state of the repository shown in its place, when it is still the one of that folder: the
/// timeline keeps its rows, and what waited for the rest shows it. Says whether it was taken.
fn put_rest(
    state: &mut State,
    folder: &Path,
    (repository, counted): (model::state::RepositoryState, git::read::worktree::CountedWorktrees),
) -> bool {
    let generation = state.next_generation;
    let Some(workspace) = state.workspace.as_mut() else {
        return false;
    };
    let Shown::Read(read) = &mut workspace.shown else {
        return false;
    };
    if !same_folder(&read.folder, folder) {
        return false;
    }
    let repository = Arc::new(repository);
    read.state = Arc::clone(&repository);
    read.generation = generation;
    state.next_generation += 1;
    state.reads.counted = counted;
    let _rows_kept = timeline::keep_rows(state, &repository);
    rail::arrange(state);
    true
}
