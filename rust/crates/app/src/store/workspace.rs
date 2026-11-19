//! The workspace of the window: opening folders (`repo.open-folder`, `repo.open-recent`), the repositories found in
//! them (`repo.read-repositories`), the one shown (`repo.select`) and its reads (`repo.read-state`), and what the
//! window shows when there is none (`repo.no-repository`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use model::filter::checked;
use model::refs::References;
use model::repo::Repository;
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
    /// Whether it is the workspace shown with a folder added.
    pub is_added: bool,
    /// The workspace file of VS Code its folders come from.
    pub file: Option<PathBuf>,
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
    let page_before = (request.is_shown_at_once && state.workspace.is_none()).then_some(state.pages.shown);
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
        is_added: request.is_added,
        file: request.file,
        page_before,
    });
    effects.push(Effect::Start(job));
    effects
}

/// Opens a recent workspace, with the repository shown in it last; the window keeps what it shows until its
/// repositories are found, since one whose folders are all gone is refused.
///
/// Feature: repo.open-recent
pub(super) fn open_recent(state: &mut State, index: usize) -> Vec<Effect> {
    let Some(recent) = state.persisted.recent.get(index).cloned() else {
        return Vec::new();
    };
    let request = OpenRequest {
        remembered: recent.repository,
        recent: Some(index),
        file: recent.file,
        ..OpenRequest::default()
    };
    open_workspace(state, recent.folders, request)
}

/// Takes the repositories found for the workspace being opened: refuses a recent one whose folders are all gone,
/// otherwise shows the workspace with the repository remembered for it, or the first one found.
pub fn repositories_found(state: &mut State, job: JobId, found: Found) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(opening) = state.opening.take_if(|opening| opening.job == job) else {
        return Vec::new();
    };
    let repositories = match found {
        Found::AllGone if opening.recent.is_some() || opening.file.is_some() => {
            give_back(state, &opening);
            return refuse_recent(state, opening.recent);
        }
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
            file: opening.file,
            ..RecentWorkspace::default()
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
    (result, read_at): (super::ReadResult, i64),
) -> Vec<Effect> {
    let Some(JobKind::ReadRepository { request, .. }) = state.jobs.remove(&job) else {
        return Vec::new();
    };
    let generation = state.next_generation;
    let Some(workspace) = state.workspace.as_mut().filter(|workspace| match &workspace.shown {
        Shown::Read(read) => same_folder(&read.folder, folder),
        Shown::Reading { folder: reading } => same_folder(reading, folder),
        Shown::Looking | Shown::NoRepository => false,
    }) else {
        // Another repository was chosen meanwhile: the pages that follow are not read.
        return result
            .map(|read| {
                read.1
                    .as_ref()
                    .map(|page| super::timeline::cancel_unwanted(job, page))
                    .unwrap_or_default()
            })
            .unwrap_or_default();
    };
    match result {
        Ok(read) => {
            let (repository, commits, counted) = *read;
            state.next_generation += 1;
            let repository = Arc::new(repository);
            workspace.shown = Shown::Read(RepositoryRead {
                folder: folder.to_path_buf(),
                state: Arc::clone(&repository),
                generation,
            });
            state.reads.counted = counted;
            let mut effects = Vec::new();
            let is_changed = match commits {
                Some(commits) => {
                    let shown = checked(request.shown.as_ref(), &References::of(&repository));
                    effects.extend(super::timeline::stop_stream(state));
                    super::timeline::take_read(state, (folder, &repository), (commits, shown), (job, read_at))
                }
                None => super::timeline::keep_rows(state, &repository),
            };
            super::rail::arrange(state);
            effects.extend(super::timeline::keep_shown(state, folder));
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
///
/// Feature: repo.select
pub fn select(state: &mut State, folder: &Path) -> Vec<Effect> {
    if state.is_operation_running {
        return Vec::new();
    }
    choose(state, folder)
}

/// Shows a repository of the workspace as `repo.select` does, the repository shown being read for it; a tab shown
/// while an operation runs is chosen this way too.
pub(super) fn choose(state: &mut State, folder: &Path) -> Vec<Effect> {
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
            ..RecentWorkspace::default()
        });
        effects.push(Effect::WritePersisted(state.persisted.clone()));
    }
    state.pages.shown = None;
    super::tabs::follow_shown(state);
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

/// Gives the window back what it showed before a workspace it had none before was shown at once, while its
/// repositories were looked for: nothing opens.
pub(super) fn give_back(state: &mut State, opening: &Opening) {
    if let Some(page) = opening.page_before {
        state.workspace = None;
        state.pages.shown = page;
    }
}

/// Says that the folders of a workspace are all gone, and takes it out of the recent ones when it is one of them.
fn refuse_recent(state: &mut State, recent: Option<usize>) -> Vec<Effect> {
    if let Some(index) = recent.filter(|index| *index < state.persisted.recent.len()) {
        let _gone = state.persisted.recent.remove(index);
    }
    state.notice = Some(Notice::RecentGone);
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// The repository of a folder among those found.
/// The folder of the repository of the workspace that holds a file or a folder, the innermost when they are inside one
/// another.
pub fn repository_holding(state: &State, path: &Path) -> Option<PathBuf> {
    state
        .repositories()
        .iter()
        .filter_map(|repository| {
            model::repo::path_in_folder(path, &repository.folder).map(|inside| (inside.len(), &repository.folder))
        })
        .min_by_key(|(inside, _)| *inside)
        .map(|(_, folder)| folder.clone())
}

fn find_repository<'a>(repositories: &'a [Repository], folder: &Path) -> Option<&'a Repository> {
    repositories
        .iter()
        .find(|repository| same_folder(&repository.folder, folder))
}

/// Starts a read of a repository with the settings in force.
pub(super) fn start_read(state: &mut State, folder: PathBuf) -> Effect {
    let mut request = super::timeline::read_request(state);
    request.shown = super::timeline::kept_shown(state, &folder);
    // A repository chosen is read whole: every other worktree is counted again.
    let counted = git::read::worktree::CountedWorktrees::default();
    let job = start_job(
        state,
        JobKind::ReadRepository {
            folder,
            request,
            kept: None,
            counted,
        },
    );
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
pub(super) fn cancel_reads(state: &mut State) -> Vec<Effect> {
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

/// Who draws the title bar, and whether the window is maximized, as it changed (`window.title-bar`).
pub(super) fn caption_changed(state: &mut State, caption: views::window::CaptionState) -> Vec<Effect> {
    state.caption = caption;
    Vec::new()
}

/// The window is closed: every read and operation stopped, its size, its place and whether it is maximized kept with
/// what is persisted, then it quits.
pub(super) fn close_requested(state: &mut State, window: crate::persisted::WindowPlace) -> Vec<Effect> {
    let mut effects = cancel_all(state);
    effects.extend(super::tabs::cancel_all(state));
    let kept = state.persisted.window;
    state.persisted.window = crate::persisted::WindowPlace {
        size: window.size.or(kept.size),
        position: window.position.or(kept.position),
        is_maximized: window.is_maximized,
    };
    effects.push(Effect::WritePersisted(state.persisted.clone()));
    effects.push(Effect::Quit);
    effects
}

/// Opens a repository the window made, a clone or a new one, in a tab and shows it: with no workspace open, its folder
/// is the workspace; a folder of the workspace is looked at again, its repository shown; any other folder is added to
/// the workspace, as **Add to the workspace** adds one, and shown (`repo.clone`, `repo.init`).
pub fn open_made(state: &mut State, folder: PathBuf) -> Vec<Effect> {
    let Some(folders) = state.workspace.as_ref().map(|workspace| workspace.folders.clone()) else {
        let request = OpenRequest {
            is_shown_at_once: true,
            ..OpenRequest::default()
        };
        return open_workspace(state, vec![folder], request);
    };
    state.pages.shown = None;
    let is_in_workspace = folders.iter().any(|known| same_folder(known, &folder));
    let request = OpenRequest {
        remembered: Some(folder.clone()),
        is_added: !is_in_workspace,
        ..OpenRequest::default()
    };
    let mut folders = folders;
    if !is_in_workspace {
        folders.push(folder);
    }
    open_workspace(state, folders, request)
}
