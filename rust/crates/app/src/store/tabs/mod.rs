//! The tabs of the repositories open in the window (`repo.tabs`, `repo.switch-tab`, `repo.keep-open`): one for each
//! repository of the workspace, the one shown, and what each of the others keeps while it is not.
//!
//! The store holds what the repository shown shows in the fields of [`State`]; a tab that is left takes those fields
//! with it, as a [`RepositoryView`], and gives them back when it is shown again. What its reads answer meanwhile is
//! kept with it, in the order it came, and told to the store once it is shown.

mod close;
mod data;
mod open;
mod view;

use std::path::{Path, PathBuf};

use views::Intent;

pub use close::{close_other_tabs, close_tab};
pub use data::TabsData;
pub use open::{Choosing, choose_tabs, open_tab, repositories_found};
pub use view::RepositoryView;
use view::{Answered, put_view, take_view};

use super::message::{Effect, JobId, Message};
use super::state::State;
use crate::persisted::{RecentWorkspace, same_folder};

/// The tabs of the window.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Tabs {
    /// The tabs, in their order.
    pub open: Vec<Tab>,
    /// The tab shown; none while the window shows no repository.
    pub shown: Option<usize>,
}

/// A repository open in a tab.
#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    /// The folder of the repository.
    pub folder: PathBuf,
    /// What it showed when it was left, with `repositories.keepOpen` on; none for the tab shown, for one never shown,
    /// and with the setting off.
    pub kept: Option<Box<RepositoryView>>,
    /// What its reads answered while it was not shown, in the order it came.
    pub waiting: Vec<Message>,
}

impl Tab {
    /// A tab for a repository, which keeps nothing yet.
    #[must_use]
    pub fn new(folder: PathBuf) -> Tab {
        Tab {
            folder,
            kept: None,
            waiting: Vec::new(),
        }
    }
}

/// What the island of the tabs and the pickers of the tabs asked.
pub fn reduce(state: &mut State, intent: Intent) -> Vec<Effect> {
    match intent {
        Intent::SwitchTab(index) => switch_tab(state, index),
        Intent::OpenTab(folder) => open_tab(state, &folder),
        Intent::PickTabToOpen => {
            open::pick_tab(state);
            Vec::new()
        }
        Intent::TabsChosen(chosen) => {
            let chosen: Vec<PathBuf> = match (&state.dialog, chosen) {
                (Some(super::state::Dialog::ChooseTabs(choosing)), Some(places)) => places
                    .iter()
                    .filter_map(|place| choosing.offered.get(*place).cloned())
                    .collect(),
                _ => Vec::new(),
            };
            choose_tabs(state, &chosen)
        }
        Intent::CloseTab(index) => close_tab(state, index),
        Intent::CloseOtherTabs(index) => close_other_tabs(state, index),
        Intent::CloseTabShown => state.tabs.shown.map_or_default(|shown| close_tab(state, shown)),
        _ => Vec::new(),
    }
}

/// Marks the tab of the repository shown as the one shown, after a repository was chosen without the tabs.
pub(super) fn follow_shown(state: &mut State) {
    if let Some(index) = shown_tab(state) {
        state.tabs.shown = Some(index);
    }
}

/// The tab of the repository shown, or of the repository whose worktree is shown in its place.
pub(super) fn shown_tab(state: &State) -> Option<usize> {
    let folder = state.worktrees.window_folder.as_ref().or(state.shown_folder())?;
    state.tabs.open.iter().position(|tab| same_folder(&tab.folder, folder))
}

/// Shows the repository of another tab (`repo.switch-tab`): nothing while an operation runs, or when it is the tab
/// shown. With `repositories.keepOpen` on, the tab left keeps what it shows, and a tab that kept something shows it at
/// once, read again behind it; otherwise the repository is read as when it is chosen (`repo.select`).
///
/// Feature: repo.switch-tab
pub fn switch_tab(state: &mut State, index: usize) -> Vec<Effect> {
    if state.is_operation_running || state.tabs.shown == Some(index) {
        return Vec::new();
    }
    let keep_open = state.settings.keep_open;
    switch(state, index, keep_open)
}

/// Shows the tab at a place, the tab left keeping what it shows when `keeps_left` says so; what it waited for is
/// cancelled otherwise.
fn switch(state: &mut State, index: usize, keeps_left: bool) -> Vec<Effect> {
    let Some(folder) = state.tabs.open.get(index).map(|tab| tab.folder.clone()) else {
        return Vec::new();
    };
    let mut effects = super::workspace::cancel_reads(state);
    let (left, not_wanted) = kept_on_leaving(take_view(state), keeps_left);
    effects.extend(not_wanted.into_iter().map(Effect::Cancel));
    if let Some(tab) = state.tabs.shown.and_then(|shown| state.tabs.open.get_mut(shown)) {
        tab.kept = left.map(Box::new);
    }
    state.dialog = None;
    let Some(tab) = state.tabs.open.get_mut(index) else {
        return effects;
    };
    let kept = tab.kept.take();
    let waiting = std::mem::take(&mut tab.waiting);
    state.tabs.shown = Some(index);
    let Some(kept) = kept else {
        effects.extend(super::workspace::select(state, &folder));
        return effects;
    };
    put_view(state, *kept);
    effects.extend(remember(state));
    effects.push(super::workspace::watch_settings(state));
    let read_folder = state.shown_folder().cloned().unwrap_or(folder);
    effects.push(super::workspace::start_read(state, read_folder));
    for message in waiting {
        effects.extend(super::reduce_message(state, message));
    }
    effects
}

/// Remembers with the workspace its tabs open and the one shown, for the next time it opens.
pub(super) fn remember(state: &mut State) -> Vec<Effect> {
    let Some(workspace) = state.workspace.as_ref() else {
        return Vec::new();
    };
    let shown = state.tabs.shown.and_then(|shown| state.tabs.open.get(shown));
    let recent = RecentWorkspace {
        folders: workspace.folders.clone(),
        repository: shown.map(|tab| tab.folder.clone()),
        tabs: Some(state.tabs.open.iter().map(|tab| tab.folder.clone()).collect()),
        file: None,
    };
    state.persisted.remember_workspace(recent);
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// What a tab keeps when it is left (`repo.keep-open`), and the jobs it started that nobody waits for any more: with
/// `repositories.keepOpen` on, all it shows, still waiting for its jobs; with it off, nothing but what is kept between
/// runs anyway, which the store keeps by repository itself, and none of its jobs.
///
/// Feature: repo.keep-open
#[must_use]
pub fn kept_on_leaving(view: RepositoryView, keep_open: bool) -> (Option<RepositoryView>, Vec<JobId>) {
    if keep_open {
        (Some(view), Vec::new())
    } else {
        (None, view.jobs.into_keys().collect())
    }
}

/// Shows the tab of a repository of the workspace: chosen in the picker of `repo.select`, or the one a file is in,
/// shown before the file's history or blame (`repo.owner-of-file`).
pub fn show_repository(state: &mut State, folder: &Path) -> Vec<Effect> {
    let Some(index) = state.tabs.open.iter().position(|tab| same_folder(&tab.folder, folder)) else {
        return super::workspace::select(state, folder);
    };
    // The tab shown, showing a worktree or a submodule in its place, comes back to its own repository.
    if state.tabs.shown == Some(index) {
        return super::workspace::select(state, folder);
    }
    switch_tab(state, index)
}

/// The tab that waits for what a message answers, when it is not the tab shown.
#[must_use]
pub fn waiting_for(state: &State, message: &Message) -> Option<usize> {
    let answered = Answered::of(message)?;
    state
        .tabs
        .open
        .iter()
        .position(|tab| tab.kept.as_ref().is_some_and(|view| answered.is_for(view)))
}

/// Keeps a message for the tab that waits for it, until it is shown.
pub fn keep_for(state: &mut State, index: usize, message: Message) {
    if let Some(tab) = state.tabs.open.get_mut(index) {
        tab.waiting.push(message);
    }
}

/// With `repositories.keepOpen` turned off, the tabs not shown forget what they kept, and what they waited for is
/// cancelled.
pub fn settings_changed(state: &mut State) -> Vec<Effect> {
    if state.settings.keep_open {
        return Vec::new();
    }
    let mut effects = Vec::new();
    for tab in &mut state.tabs.open {
        tab.waiting.clear();
        if let Some(kept) = tab.kept.take() {
            effects.extend(kept.jobs.into_keys().map(Effect::Cancel));
        }
    }
    effects
}

/// Every job the tabs not shown wait for, which closing the window cancels.
pub fn cancel_all(state: &mut State) -> Vec<Effect> {
    std::mem::take(&mut state.tabs.open)
        .into_iter()
        .flat_map(forget)
        .collect()
}

/// What a tab closed waited for, cancelled.
pub(super) fn forget(tab: Tab) -> Vec<Effect> {
    tab.kept
        .map(|kept| kept.jobs.into_keys().map(Effect::Cancel).collect())
        .unwrap_or_default()
}
