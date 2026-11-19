//! What the island of the tabs and the pickers of the tabs are drawn from (`repo.tabs`, `repo.open-tab`,
//! `repo.choose-tabs`).

use model::repo::{WorktreeSummary, group_tabs, name_repositories};
use views::graph::RepositoryEntry;
use views::repo::{TabEntry, TabSummary, TabsPickerKind, TabsPickerState, TabsState};

use super::super::state::{Dialog, Shown, State};
use super::menu::{TabShows, tab_name};
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

/// The repositories of the workspace with no tab, which **+** offers to open (`repo.open-tab`): one for each group of
/// `repo.group-tabs` none of whose repositories has a tab.
fn not_open(state: &State, repositories: &[RepositoryEntry]) -> Vec<RepositoryEntry> {
    let groups = group_tabs(state.repositories());
    repositories
        .iter()
        .filter(|entry| {
            groups
                .iter()
                .find(|group| group.holds(&entry.folder))
                .is_none_or(|group| {
                    same_folder(&group.repository.folder, &entry.folder)
                        && !state.tabs.open.iter().any(|tab| group.holds(&tab.folder))
                })
        })
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

/// What a row, or a row of a worktree or a submodule under it, says a folder is on (`repo.worktree-summary`): the folder shown from its own
/// read; any other from its read in the background, or before that from what its tab read last; none before.
fn summary_of(state: &State, (folder, tab): (&std::path::Path, &super::Tab)) -> Option<TabSummary> {
    let is_shown = state.shown_folder().is_some_and(|shown| same_folder(shown, folder));
    let read = if is_shown {
        state.workspace.as_ref().map(|workspace| &workspace.shown)
    } else if let Some(summary) = state.tabs.summaries.of(folder) {
        return Some(summary.clone());
    } else {
        tab.kept
            .as_ref()
            .filter(|kept| super::kept_folder(kept).is_some_and(|kept| same_folder(kept, folder)))
            .and_then(|kept| kept.shown.as_ref())
    };
    match read? {
        Shown::Read(read) => Some(TabSummary {
            summary: WorktreeSummary::of_state(&read.state),
            is_stale: false,
        }),
        Shown::Looking | Shown::Reading { .. } | Shown::NoRepository => None,
    }
}

/// The tabs as the island draws them: each by the name of its repository among those of the workspace, with the
/// worktree or the submodule it shows after ` › ` (`repo.group-tabs`), the folder of what it shows for its tooltip,
/// and the repositories of its group for the rows under it, unless it was folded.
fn entries(state: &State, repositories: &[RepositoryEntry]) -> Vec<TabEntry> {
    let groups = group_tabs(state.repositories());
    let folded = super::super::timeline::workspace_panes(state).folded_tabs;
    state
        .tabs
        .open
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let own = repositories
                .iter()
                .find(|entry| same_folder(&entry.folder, &tab.folder))
                .map_or_else(|| tab.folder.display().to_string(), |entry| entry.name.clone());
            let mut shows = TabShows::new(state, (index, tab), &groups, &own);
            for member in &mut shows.members {
                member.summary = summary_of(state, (&member.folder, tab));
            }
            let is_shown = state.tabs.shown == Some(index);
            TabEntry {
                name: tab_name(&own, shows.shown_label.as_deref()),
                summary: summary_of(state, (&shows.folder, tab)),
                repository: own,
                place: shows.place,
                folder: shows.folder.display().to_string(),
                is_shown,
                is_running: super::operation_tab(state) == Some(index),
                members: shows.members,
                is_unfolded: !folded.iter().any(|folder| same_folder(folder, &tab.folder)),
                looked_after: state
                    .shown_read()
                    .filter(|_| is_shown)
                    .map(|read| read.state.is_looked_after),
            }
        })
        .collect()
}
