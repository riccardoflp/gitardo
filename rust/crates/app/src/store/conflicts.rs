//! An operation stopped on conflicts, in the store: the conflicts the rules resolve, counted again after each read of a
//! repository with files in conflict when `conflicts.autoResolve` or `conflicts.ai` is on (`conflicts.banner`,
//! `conflicts.stopped`), the banner as the views are given it, and a file in conflict opened where it can be resolved
//! (`conflicts.open-file`).

use std::path::PathBuf;

use model::conflicts::RuleCounts;
use views::conflicts::{BannerState, ConflictsIntent, OpenAt, StoppedState, open_target};
use views::diff::DiffVersion;

use super::feature::Said;
use super::message::{Effect, JobId, JobKind};
use super::state::{RepositoryRead, State};
use crate::persisted::same_folder;

/// The conflicts counted in the repository shown.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Conflicts {
    /// The job counting them, while it runs.
    pub job: Option<JobId>,
    /// The last count, with the folder of the repository it is of: it stays until the next one replaces it.
    pub counts: Option<(PathBuf, RuleCounts)>,
}

/// After a read of the repository shown: its conflicts are counted again when it has files in conflict and a setting
/// wants the count; otherwise the count is forgotten. The message of an operation that failed becomes that of
/// `conflicts.stopped` when it left files in conflict.
pub fn after_read(state: &mut State) -> Vec<Effect> {
    check_stopped(state);
    let mut effects: Vec<Effect> = state
        .conflicts
        .job
        .take()
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .into_iter()
        .collect();
    let features = &state.settings.features;
    let is_wanted = features.conflicts_auto_resolve || features.conflicts_ai;
    let folder = state
        .shown_read()
        .filter(|read| is_wanted && !read.state.working_tree.conflicted.is_empty())
        .map(|read| (read.folder.clone(), read.state.folder.clone()));
    let Some((folder, top)) = folder else {
        state.conflicts.counts = None;
        return effects;
    };
    if state
        .conflicts
        .counts
        .as_ref()
        .is_some_and(|(counted, _)| !same_folder(counted, &folder))
    {
        state.conflicts.counts = None;
    }
    let job = super::workspace::start_job(state, JobKind::CountConflicts { folder: top });
    state.conflicts.job = Some(job.id);
    effects.push(Effect::Start(job));
    effects
}

/// Takes the count of the conflicts; a count no longer wanted, or one that failed, is dropped.
pub fn counted(state: &mut State, job: JobId, result: Result<RuleCounts, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    if state.conflicts.job != Some(job) {
        return Vec::new();
    }
    state.conflicts.job = None;
    let folder = state.shown_read().map(|read| read.folder.clone());
    state.conflicts.counts = match (folder, result) {
        (Some(folder), Ok(counts)) => Some((folder, counts)),
        (None, _) | (_, Err(_)) => None,
    };
    Vec::new()
}

/// The count of the conflicts of a repository read, when there is one.
#[must_use]
pub fn counts_of(state: &State, read: &RepositoryRead) -> Option<RuleCounts> {
    state
        .conflicts
        .counts
        .as_ref()
        .filter(|(folder, _)| same_folder(folder, &read.folder))
        .map(|(_, counts)| *counts)
}

/// The banner of the repository read, while an operation is in progress or files are in conflict.
#[must_use]
pub fn banner_state<'a>(state: &'a State, read: &'a RepositoryRead) -> Option<BannerState<'a>> {
    let repository = &read.state;
    let conflicted = repository.working_tree.conflicted.len();
    let stop = super::rebase::stop_state(repository);
    if repository.operation.is_none() && conflicted == 0 && stop.is_none() {
        return None;
    }
    let features = &state.settings.features;
    Some(BannerState {
        operation: repository.operation,
        conflicted,
        stop,
        counts: counts_of(state, read),
        auto_resolve: features.conflicts_auto_resolve,
        ai: features.conflicts_ai,
        is_running: state.is_operation_running,
        resolved_as_before: repository.resolved_as_before.len(),
    })
}

/// An intent of a file in conflict: a click on it opens it where it can be resolved, unless it is resolved meanwhile;
/// from the line number of a card of the review, at that line: the File page at it, the Merge page at the region of the
/// result it is in.
pub fn reduce(state: &mut State, intent: ConflictsIntent) -> Vec<Effect> {
    let (path, line) = match intent {
        ConflictsIntent::Open(path) => (path, None),
        ConflictsIntent::OpenAtLine(path, line) => (path, Some(line)),
        ConflictsIntent::ShowWhatGitWrote { path, is_staged } => return show_what_git_wrote(state, &path, is_staged),
    };
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let repository = &read.state;
    let target = repository
        .working_tree
        .conflicted
        .iter()
        .find(|file| file.path == path)
        .and_then(|file| open_target(file, repository.operation));
    match target {
        Some(OpenAt::Merge(path)) => super::merge_page::open_at(state, &path, line),
        Some(OpenAt::Diff(target)) => super::diff_page::open(state, target),
        Some(OpenAt::File(mut target)) => {
            target.line = line;
            super::file_page::open(state, target)
        }
        None => Vec::new(),
    }
}

/// Once the repository is read again after an operation failed, its message becomes that of `conflicts.stopped` when
/// files are in conflict.
fn check_stopped(state: &mut State) {
    let Some(title) = state.operations.check_stopped.take() else {
        return;
    };
    let conflicted = state
        .shown_read()
        .map_or(0, |read| read.state.working_tree.conflicted.len());
    if let (1.., Some(said)) = (conflicted, state.operations.said.as_mut()) {
        said.stopped = Some((title, conflicted));
    }
}

/// The message of an operation that stopped on conflicts, when what was said is one.
#[must_use]
pub fn stopped_state<'a>(state: &'a State, said: &'a Said) -> Option<StoppedState<'a>> {
    let (title, conflicted) = said.stopped.as_ref()?;
    let read = state.shown_read()?;
    let features = &state.settings.features;
    Some(StoppedState {
        title,
        conflicted: *conflicted,
        operation: read.state.operation,
        counts: counts_of(state, read),
        auto_resolve: features.conflicts_auto_resolve,
        ai: features.conflicts_ai,
        resolved_as_before: read.state.resolved_as_before.len(),
    })
}

/// **Show what git wrote** (`rerere.resolved-as-before`): the Diff page of a file as git marked it, titled **As git
/// marked it**, against the file on disk; of a file git staged, of the last commit and the index.
fn show_what_git_wrote(state: &mut State, path: &str, is_staged: bool) -> Vec<Effect> {
    let (left, right) = if is_staged {
        (DiffVersion::Head, DiffVersion::Index)
    } else {
        (DiffVersion::Marked, DiffVersion::WorkingTree)
    };
    let target = views::diff::DiffTarget::file(left, right, views::diff::DiffFile::same(path));
    super::diff_page::open(state, target)
}
