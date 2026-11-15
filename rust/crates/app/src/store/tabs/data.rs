//! What the island of the tabs and the pickers of the tabs are drawn from (`repo.tabs`, `repo.open-tab`,
//! `repo.choose-tabs`).

use model::repo::name_repositories;
use views::graph::RepositoryEntry;
use views::repo::{TabEntry, TabsPickerKind, TabsPickerState, TabsState};

use super::super::state::{Dialog, State};
use crate::persisted::same_folder;

/// The words of the tabs, made once for a frame.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TabsData {
    /// The tabs, by the names of their repositories, the one shown marked.
    tabs: Vec<TabEntry>,
    /// The repositories of the workspace with no tab.
    not_open: Vec<RepositoryEntry>,
    /// The repositories the picker of the tabs open lists, while it is open.
    picked: Vec<RepositoryEntry>,
}

impl TabsData {
    /// The words of the tabs of a state, with the repositories of the workspace by their names.
    #[must_use]
    pub fn new(state: &State, repositories: &[RepositoryEntry]) -> TabsData {
        let not_open = not_open(state, repositories);
        let picked = match &state.dialog {
            Some(Dialog::OpenTab) => not_open.clone(),
            Some(Dialog::ChooseTabs(choosing)) => name_repositories(&choosing.opening.folders, &choosing.offered)
                .into_iter()
                .zip(&choosing.offered)
                .map(|(name, folder)| RepositoryEntry {
                    name,
                    folder: folder.clone(),
                    is_current: false,
                })
                .collect(),
            _ => Vec::new(),
        };
        TabsData {
            tabs: entries(state, repositories),
            not_open,
            picked,
        }
    }

    /// What the island of the tabs shows; none while the window shows no workspace.
    #[must_use]
    pub fn island<'a>(&'a self, state: &State) -> Option<TabsState<'a>> {
        state.workspace.is_some().then_some(TabsState {
            tabs: &self.tabs,
            not_open: &self.not_open,
        })
    }

    /// The picker of the tabs open, if one is.
    #[must_use]
    pub fn picker<'a>(&'a self, state: &State) -> Option<TabsPickerState<'a>> {
        let kind = match &state.dialog {
            Some(Dialog::OpenTab) => TabsPickerKind::Open,
            Some(Dialog::ChooseTabs(_)) => TabsPickerKind::Choose,
            _ => return None,
        };
        Some(TabsPickerState {
            kind,
            repositories: &self.picked,
        })
    }
}

/// The repositories of the workspace with no tab, which **+** offers to open (`repo.open-tab`).
fn not_open(state: &State, repositories: &[RepositoryEntry]) -> Vec<RepositoryEntry> {
    repositories
        .iter()
        .filter(|entry| {
            !state
                .tabs
                .open
                .iter()
                .any(|tab| same_folder(&tab.folder, &entry.folder))
        })
        .cloned()
        .collect()
}

/// The tabs as the island draws them: each by the name of its repository among those of the workspace, and with the
/// folder of what it shows for its tooltip, a worktree shown in place of its repository included.
fn entries(state: &State, repositories: &[RepositoryEntry]) -> Vec<TabEntry> {
    let shown = state.shown_folder();
    state
        .tabs
        .open
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let is_shown = state.tabs.shown == Some(index);
            let name = repositories
                .iter()
                .find(|entry| same_folder(&entry.folder, &tab.folder))
                .map_or_else(|| tab.folder.display().to_string(), |entry| entry.name.clone());
            let folder = match shown {
                Some(shown) if is_shown => shown.display().to_string(),
                _ => tab.folder.display().to_string(),
            };
            TabEntry { name, folder, is_shown }
        })
        .collect()
}
