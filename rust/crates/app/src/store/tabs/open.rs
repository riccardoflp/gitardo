//! Which repositories open in tabs: the ones a workspace had, the ones chosen when it opens for the first time with
//! several (`repo.choose-tabs`), and one more opened from **+** (`repo.open-tab`).

use std::path::{Path, PathBuf};

use model::repo::{Repository, group_tabs};

use super::super::message::{Effect, Found, JobId};
use super::super::state::{Dialog, Opening, State};
use super::super::workspace;
use super::{Tab, forget, remember, show_in_tab, shown_tab, switch_tab, tab_of};
use crate::persisted::{same_folder, same_folders};

/// The repositories of a workspace being opened, while the user chooses which open in tabs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choosing {
    /// The job that found them.
    pub job: JobId,
    /// Every repository found, submodules included.
    pub repositories: Vec<Repository>,
    /// The repositories offered, by their folders, in the order they were found.
    pub offered: Vec<PathBuf>,
    /// The workspace being opened.
    pub opening: Opening,
}

/// The tabs a workspace opens with.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Wanted {
    /// The tabs open whose repositories are still found.
    Kept,
    /// These, in this order.
    Open(Vec<PathBuf>),
    /// The tabs open, and these after them.
    Added(Vec<PathBuf>),
    /// The user chooses among these.
    Asked(Vec<PathBuf>),
}

/// The repositories of a workspace being opened are found: the user chooses which open in tabs when it opens for the
/// first time with several, or when a folder added brings several; otherwise the workspace opens with the tabs it had,
/// all of them for a workspace last opened before there were tabs.
pub fn repositories_found(state: &mut State, job: JobId, found: Found) -> Vec<Effect> {
    match wanted(state, job, &found) {
        Wanted::Asked(offered) => {
            let (Some(opening), Found::Repositories(repositories)) = (state.opening.take(), found) else {
                return Vec::new();
            };
            let _finished = state.jobs.remove(&job);
            let choosing = Choosing {
                job,
                repositories,
                offered,
                opening,
            };
            state.dialog = Some(Dialog::ChooseTabs(Box::new(choosing)));
            Vec::new()
        }
        wanted => open_with(state, (job, found), wanted),
    }
}

/// Opens the repositories checked in the picker, the first one shown; a workspace whose folder was added keeps the
/// repository shown and gets the tabs after its own. Nothing checked, or `Escape`, opens nothing and gives the window
/// back what it showed.
///
/// Feature: repo.choose-tabs
pub fn choose_tabs(state: &mut State, chosen: &[PathBuf]) -> Vec<Effect> {
    let Some(Dialog::ChooseTabs(choosing)) = state.dialog.take() else {
        return Vec::new();
    };
    let Choosing {
        job,
        repositories,
        opening,
        ..
    } = *choosing;
    if chosen.is_empty() {
        workspace::give_back(state, &opening);
        return Vec::new();
    }
    let wanted = if opening.is_added {
        Wanted::Added(chosen.to_vec())
    } else {
        Wanted::Open(chosen.to_vec())
    };
    state.opening = Some(opening);
    open_with(state, (job, Found::Repositories(repositories)), wanted)
}

/// Opens a repository of the workspace in a tab after the tab shown, and shows it; a repository open already has its
/// tab shown, and a worktree or a submodule of a repository open is shown in the tab of that repository
/// (`repo.group-tabs`).
///
/// Feature: repo.open-tab
pub fn open_tab(state: &mut State, folder: &Path) -> Vec<Effect> {
    if state.dialog == Some(Dialog::OpenTab) {
        state.dialog = None;
    }
    if let Some(index) = tab_of(state, folder) {
        return show_in_tab(state, index, folder);
    }
    let found = state
        .repositories()
        .iter()
        .find(|repository| same_folder(&repository.folder, folder))
        .map(|repository| repository.folder.clone());
    let Some(found) = found.filter(|_| !state.is_operation_running) else {
        return Vec::new();
    };
    let at = state.tabs.shown.map_or(state.tabs.open.len(), |shown| shown + 1);
    state.tabs.open.insert(at, Tab::new(found));
    let mut effects = switch_tab(state, at);
    effects.extend(remember(state));
    effects
}

/// **Open Repository in a Tab** of Quick search: the picker of the repositories with no tab.
pub fn pick_tab(state: &mut State) {
    state.dialog = Some(Dialog::OpenTab);
}

/// Shows the workspace found with the tabs wanted, the repository to show among them.
fn open_with(state: &mut State, (job, found): (JobId, Found), wanted: Wanted) -> Vec<Effect> {
    if let (Wanted::Open(tabs), Some(opening)) = (&wanted, state.opening.as_mut().filter(|opening| opening.job == job))
    {
        let remembered = opening
            .remembered
            .take()
            .filter(|remembered| tabs.iter().any(|tab| same_folder(tab, remembered)));
        opening.remembered = remembered.or_else(|| tabs.first().cloned());
    }
    let mut effects = workspace::repositories_found(state, job, found);
    effects.extend(follow(state, wanted));
    effects
}

/// The tabs a workspace being opened wants, from what was found in its folders.
fn wanted(state: &State, job: JobId, found: &Found) -> Wanted {
    let Some(opening) = state.opening.as_ref().filter(|opening| opening.job == job) else {
        return Wanted::Kept;
    };
    let repositories = match found {
        Found::Repositories(repositories) => repositories,
        // A recent workspace or a workspace file whose folders are all gone is refused, and the window stays as it was.
        Found::AllGone if opening.recent.is_some() || opening.file.is_some() => return Wanted::Kept,
        Found::AllGone | Found::Failed(_) => return Wanted::Open(Vec::new()),
    };
    if opening.is_refresh {
        return Wanted::Kept;
    }
    let own = own_tabs(repositories);
    if opening.is_added {
        let added: Vec<PathBuf> = own
            .into_iter()
            .filter(|folder| !state.tabs.open.iter().any(|tab| same_folder(&tab.folder, folder)))
            .collect();
        return if added.len() > 1 {
            Wanted::Asked(added)
        } else {
            Wanted::Added(added)
        };
    }
    let recent = state
        .persisted
        .recent
        .iter()
        .find(|recent| same_folders(&recent.folders, &opening.folders));
    match recent.map(|recent| recent.tabs.as_ref()) {
        Some(Some(tabs)) => {
            let open: Vec<PathBuf> = tabs
                .iter()
                .filter(|tab| repositories.iter().any(|found| same_folder(&found.folder, tab)))
                .cloned()
                .collect();
            if open.is_empty() {
                Wanted::Open(own.into_iter().take(1).collect())
            } else {
                Wanted::Open(open)
            }
        }
        None if own.len() > 1 => Wanted::Asked(own),
        Some(None) | None => Wanted::Open(own),
    }
}

/// The repositories that have a tab of their own, in the order they were found: one for each group of
/// `repo.group-tabs`, its worktrees and its submodules being shown in its tab.
fn own_tabs(repositories: &[Repository]) -> Vec<PathBuf> {
    group_tabs(repositories)
        .into_iter()
        .map(|group| group.repository.folder)
        .collect()
}

/// The tabs of the window once a workspace is found: a tab still wanted keeps what it kept, the one shown is the one
/// of the repository shown; what the tabs gone waited for is cancelled. The tabs are remembered with the workspace.
fn follow(state: &mut State, wanted: Wanted) -> Vec<Effect> {
    let found: Vec<PathBuf> = state.repositories().iter().map(|found| found.folder.clone()).collect();
    let mut before = std::mem::take(&mut state.tabs.open);
    let still_found = || {
        before
            .iter()
            .map(|tab| tab.folder.clone())
            .filter(|folder| found.iter().any(|found| same_folder(found, folder)))
            .collect::<Vec<PathBuf>>()
    };
    let folders = match wanted {
        Wanted::Kept | Wanted::Asked(_) => still_found(),
        Wanted::Open(folders) => folders,
        Wanted::Added(added) => {
            let mut folders = still_found();
            folders.extend(added);
            folders
        }
    };
    state.tabs.open = folders
        .into_iter()
        .map(|folder| {
            before
                .iter()
                .position(|tab| same_folder(&tab.folder, &folder))
                .map_or_else(|| Tab::new(folder), |index| before.swap_remove(index))
        })
        .collect();
    state.tabs.shown = shown_tab(state);
    let mut effects: Vec<Effect> = before.into_iter().flat_map(forget).collect();
    effects.extend(remember(state));
    effects
}
