//! The workspace of the window: opening folders (`repo.open-folder`, `repo.open-recent`), the repositories found in
//! them (`repo.read-repositories`), the one shown (`repo.select`) and its reads (`repo.read-state`), and what the
//! window shows when there is none (`repo.no-repository`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use git::read::graph::CommitPage;
use model::filter::checked;
use model::refs::References;
use model::repo::Repository;
use model::state::RepositoryState;
use views::Notice;

use super::message::{Effect, Found, Job, JobId, JobKind};
use super::state::{Opening, RepositoryRead, Shown, State, Workspace};
use crate::persisted::{RecentWorkspace, same_folder, same_folders};
use crate::settings::{SettingsFiles, repository_settings_file};

/// How a workspace is opened.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OpenRequest {
    /// The repository to show when it is found.
    pub remembered: Option<PathBuf>,
    /// Its place among the recent workspaces, when it is one of them.
    pub recent: Option<usize>,
    /// Whether the window shows the workspace at once, saying **Loading repository…** while its repositories are
    /// looked for, or keeps what it shows until they are found.
    pub is_shown_at_once: bool,
    /// Whether it is the workspace shown, looked at again.
    pub is_refresh: bool,
}

/// Starts opening a workspace: its repositories are looked for on a worker. A workspace opened before shows the
/// repository shown in it last, unless the request names one.
pub fn open_workspace(state: &mut State, folders: Vec<PathBuf>, mut request: OpenRequest) -> Vec<Effect> {
    let mut effects = cancel_opening(state);
    if request.remembered.is_none() && !request.is_refresh {
        request.remembered = state
            .persisted
            .recent
            .iter()
            .find(|recent| same_folders(&recent.folders, &folders))
            .and_then(|recent| recent.repository.clone());
    }
    let job = start_job(
        state,
        JobKind::FindRepositories {
            folders: folders.clone(),
        },
    );
    if request.is_shown_at_once {
        effects.extend(cancel_reads(state));
        state.workspace = Some(Workspace {
            folders: folders.clone(),
            repositories: None,
            shown: Shown::Looking,
        });
        state.pages.shown = None;
    }
    state.opening = Some(Opening {
        job: job.id,
        folders,
        remembered: request.remembered,
        recent: request.recent,
        is_refresh: request.is_refresh,
    });
    effects.push(Effect::Start(job));
    effects
}

/// Takes the repositories found for the workspace being opened: refuses a recent one whose folders are all gone,
/// otherwise shows the workspace with the repository remembered for it, or the first one found.
pub fn repositories_found(state: &mut State, job: JobId, found: Found) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(opening) = state.opening.take_if(|opening| opening.job == job) else {
        return Vec::new();
    };
    let repositories = match found {
        Found::AllGone if opening.recent.is_some() => return refuse_recent(state, opening.recent),
        Found::Repositories(repositories) => repositories,
        Found::AllGone => Vec::new(),
        Found::Failed(error) => {
            state.last_error = Some(error);
            Vec::new()
        }
    };
    let chosen = opening
        .remembered
        .as_deref()
        .and_then(|folder| find_repository(&repositories, folder))
        .or_else(|| repositories.first())
        .map(|repository| repository.folder.clone());
    let previous = state.workspace.take();
    let kept = previous
        .map(|workspace| workspace.shown)
        .filter(|shown| match (shown, &chosen) {
            (Shown::Read(read), Some(folder)) => same_folder(&read.folder, folder),
            _ => false,
        });
    let mut effects = Vec::new();
    let shown = match (kept, chosen.clone()) {
        (Some(shown), _) if !opening.is_refresh => shown,
        (kept, Some(folder)) => {
            effects.extend(cancel_reads(state));
            effects.push(start_read(state, folder.clone()));
            kept.unwrap_or(Shown::Reading { folder })
        }
        (_, None) => Shown::NoRepository,
    };
    state.workspace = Some(Workspace {
        folders: opening.folders.clone(),
        repositories: Some(repositories),
        shown,
    });
    if !opening.is_refresh {
        state.pages.shown = None;
        state.persisted.remember_workspace(RecentWorkspace {
            folders: opening.folders,
            repository: chosen,
        });
        effects.push(Effect::WritePersisted(state.persisted.clone()));
    }
    effects.push(watch_settings(state));
    effects
}

/// Takes a read of a repository: the window shows it, unless another repository was chosen meanwhile. A read that
/// failed, as when the folder was deleted, shows `repo.no-repository`.
pub fn repository_read(
    state: &mut State,
    job: JobId,
    folder: &Path,
    (result, read_at): (Result<Box<(RepositoryState, CommitPage)>, String>, i64),
) -> Vec<Effect> {
    let Some(JobKind::ReadRepository { request, .. }) = state.jobs.remove(&job) else {
        return Vec::new();
    };
    let generation = state.next_generation;
    let Some(workspace) = state.workspace.as_mut() else {
        return Vec::new();
    };
    let is_shown = match &workspace.shown {
        Shown::Read(read) => same_folder(&read.folder, folder),
        Shown::Reading { folder: reading } => same_folder(reading, folder),
        Shown::Looking | Shown::NoRepository => false,
    };
    if !is_shown {
        return Vec::new();
    }
    match result {
        Ok(read) => {
            let (repository, commits) = *read;
            state.next_generation += 1;
            let repository = Arc::new(repository);
            workspace.shown = Shown::Read(RepositoryRead {
                folder: folder.to_path_buf(),
                state: Arc::clone(&repository),
                generation,
            });
            let page = (commits.has_more, request.pages, read_at);
            let shown = checked(request.shown.as_ref(), &References::of(&repository));
            let is_changed = super::timeline::take_read(state, (folder, &repository), (commits.rows, shown), page);
            super::rail::arrange(state);
            let mut effects = super::timeline::keep_shown(state, folder);
            effects.extend(super::pages::tick_timeline(state));
            effects.extend(super::search::settle(state));
            if is_changed {
                effects.extend(super::search::repository_read(state));
            }
            effects
        }
        Err(error) => {
            workspace.shown = Shown::NoRepository;
            state.last_error = Some(error);
            Vec::new()
        }
    }
}

/// Chooses the repository shown (`repo.select`): nothing while an operation runs or when it is the one shown; otherwise
/// it is read, and remembered for the workspace.
pub fn select(state: &mut State, folder: &Path) -> Vec<Effect> {
    if state.is_operation_running {
        return Vec::new();
    }
    state.dialog = None;
    if state.shown_folder().is_some_and(|shown| same_folder(shown, folder)) {
        return Vec::new();
    }
    let Some(chosen) = find_repository(state.repositories(), folder).map(|found| found.folder.clone()) else {
        return Vec::new();
    };
    // Choosing a repository leaves a worktree shown in place of the window's folder.
    state.worktrees.window_folder = None;
    let mut effects = cancel_reads(state);
    effects.push(start_read(state, chosen.clone()));
    if let Some(workspace) = state.workspace.as_mut() {
        workspace.shown = Shown::Reading { folder: chosen.clone() };
        let folders = workspace.folders.clone();
        state.persisted.remember_workspace(RecentWorkspace {
            folders,
            repository: Some(chosen),
        });
        effects.push(Effect::WritePersisted(state.persisted.clone()));
    }
    state.pages.shown = None;
    effects.push(watch_settings(state));
    effects
}

/// Points the views at a folder that is not among the repositories of the workspace, another worktree of the one
/// shown (`worktree.show-in-gitardo`): it is read, and not remembered for the workspace.
pub(super) fn show_folder(state: &mut State, folder: &Path) -> Vec<Effect> {
    state.dialog = None;
    let mut effects = cancel_reads(state);
    effects.push(start_read(state, folder.to_path_buf()));
    if let Some(workspace) = state.workspace.as_mut() {
        workspace.shown = Shown::Reading {
            folder: folder.to_path_buf(),
        };
    }
    state.pages.shown = None;
    effects.push(watch_settings(state));
    effects
}

/// What the **Select repository…** command does: the picker with several repositories, a notice otherwise.
pub fn pick_repository(state: &mut State) {
    match state.repositories().len() {
        0 => state.notice = Some(Notice::NoRepositoryToSelect),
        1 => state.notice = Some(Notice::SingleRepository),
        _ => state.dialog = Some(super::state::Dialog::Picker),
    }
}

/// The settings files of the window: the user's, and the one of the repository shown.
#[must_use]
pub fn watch_settings(state: &State) -> Effect {
    Effect::WatchSettings(SettingsFiles {
        user: state.user_settings_file.clone(),
        repository: state.shown_folder().map(|folder| repository_settings_file(folder)),
    })
}

/// Takes a recent workspace whose folders are all gone out of the list, and says so.
fn refuse_recent(state: &mut State, recent: Option<usize>) -> Vec<Effect> {
    if let Some(index) = recent.filter(|index| *index < state.persisted.recent.len()) {
        let _gone = state.persisted.recent.remove(index);
    }
    state.notice = Some(Notice::RecentGone);
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// The repository of a folder among those found.
fn find_repository<'a>(repositories: &'a [Repository], folder: &Path) -> Option<&'a Repository> {
    repositories
        .iter()
        .find(|repository| same_folder(&repository.folder, folder))
}

/// Starts a read of a repository with the settings in force: as many pages as its timeline has loaded when it is the
/// one shown, one otherwise.
fn start_read(state: &mut State, folder: PathBuf) -> Effect {
    let is_shown = state
        .timeline
        .folder
        .as_deref()
        .is_some_and(|shown| same_folder(shown, &folder));
    let pages = if is_shown { state.timeline.pages } else { 1 };
    let mut request = super::timeline::read_request(state, pages);
    request.shown = super::timeline::kept_shown(state, &folder);
    let job = start_job(state, JobKind::ReadRepository { folder, request });
    super::reads::adopt(state, job.id, model::repo::ReadCause::RepositoryChosen);
    Effect::Start(job)
}

/// Notes a job as started and gives it its number.
pub(super) fn start_job(state: &mut State, kind: JobKind) -> Job {
    let id = JobId(state.next_job);
    state.next_job += 1;
    let _new = state.jobs.insert(id, kind.clone());
    Job { id, kind }
}

/// Cancels the reads of repositories running, and of the next pages of the timeline: their results are not wanted any
/// more.
fn cancel_reads(state: &mut State) -> Vec<Effect> {
    super::reads::stopped(state);
    let mut effects = super::timeline::cancel_more(state);
    let reads: Vec<JobId> = state
        .jobs
        .iter()
        .filter(|(_, kind)| matches!(kind, JobKind::ReadRepository { .. }))
        .map(|(id, _)| *id)
        .collect();
    effects.extend(reads.into_iter().map(|id| {
        let _cancelled = state.jobs.remove(&id);
        Effect::Cancel(id)
    }));
    effects
}

/// Cancels the workspace being opened, which another one replaces.
fn cancel_opening(state: &mut State) -> Vec<Effect> {
    state
        .opening
        .take()
        .map(|opening| {
            let _cancelled = state.jobs.remove(&opening.job);
            Effect::Cancel(opening.job)
        })
        .into_iter()
        .collect()
}

/// Cancels every job, when the window closes.
pub fn cancel_all(state: &mut State) -> Vec<Effect> {
    state.opening = None;
    std::mem::take(&mut state.jobs)
        .into_keys()
        .map(Effect::Cancel)
        .collect()
}
