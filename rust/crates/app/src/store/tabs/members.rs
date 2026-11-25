//! The worktrees and the submodules of a repository, shown in its tab (`repo.group-tabs`): each one keeps what it
//! showed apart from the others, as a tab does (`repo.keep-open`), and its reads answer it while another is shown.

use std::path::{Path, PathBuf};

use model::repo::{TabGroup, group_tabs};

use super::super::message::{Effect, Message};
use super::super::state::{Shown, State};
use super::super::timeline::workspace_panes;
use super::super::workspace;
use super::view::{Answered, put_view, take_view};
use super::{RepositoryView, kept_on_leaving, switch_tab};
use crate::persisted::timeline::remember;
use crate::persisted::{same_folder, same_folders};

/// A repository of the group of a tab that the tab does not show, with what it kept.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    /// Its folder.
    pub folder: PathBuf,
    /// What it showed when another of the group was shown in its place.
    pub kept: Box<RepositoryView>,
    /// What its reads answered since, in the order it came.
    pub waiting: Vec<Message>,
}

/// The group of the repositories of the workspace that holds a folder (`repo.group-tabs`).
#[must_use]
pub fn group_of(state: &State, folder: &Path) -> Option<TabGroup> {
    group_tabs(state.repositories())
        .into_iter()
        .find(|group| group.holds(folder))
}

/// The tab of a folder: the one of that folder, or the one whose group holds it.
#[must_use]
pub fn tab_of(state: &State, folder: &Path) -> Option<usize> {
    let tabs = &state.tabs.open;
    tabs.iter()
        .position(|tab| same_folder(&tab.folder, folder))
        .or_else(|| {
            let group = group_of(state, folder)?;
            tabs.iter().position(|tab| group.holds(&tab.folder))
        })
}

/// Folds or unfolds the rows of the worktrees and the submodules under the row of a tab, kept for the workspace as
/// the folds of the sections of the side column are.
pub fn fold_tab(state: &mut State, index: usize) -> Vec<Effect> {
    let Some(folder) = state.tabs.open.get(index).map(|tab| tab.folder.clone()) else {
        return Vec::new();
    };
    if state.workspace.is_none() {
        return Vec::new();
    }
    let mut panes = workspace_panes(state);
    let was_folded = panes.folded_tabs.iter().any(|kept| same_folder(kept, &folder));
    panes.folded_tabs.retain(|kept| !same_folder(kept, &folder));
    if !was_folded {
        panes.folded_tabs.push(folder);
    }
    let folders = panes.folders.clone();
    remember(&mut state.persisted.panes, panes, |kept| {
        same_folders(&kept.folders, &folders)
    });
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// Shows a repository of a group in a tab: the tab is shown first, then the repository in it.
pub fn show_in_tab(state: &mut State, index: usize, folder: &Path) -> Vec<Effect> {
    if state.is_operation_running || index >= state.tabs.open.len() {
        return Vec::new();
    }
    let mut effects = switch_tab(state, index);
    if state.tabs.shown == Some(index) {
        effects.extend(show_member(state, folder));
    }
    effects
}

/// Shows in the tab shown another repository of its group, a worktree or a submodule, or its own again: what the one
/// left showed is kept apart, as `repositories.keepOpen` says, and the one shown finds what it kept, read again behind
/// it; with nothing kept, a worktree of the repository left takes the rows that one has read (`repo.share-history`),
/// and anything else is read for the first time.
pub fn show_member(state: &mut State, folder: &Path) -> Vec<Effect> {
    if state.is_operation_running {
        return Vec::new();
    }
    let (Some(index), Some(current)) = (state.tabs.shown, state.shown_folder().cloned()) else {
        return Vec::new();
    };
    let Some(tab_folder) = state.tabs.open.get(index).map(|tab| tab.folder.clone()) else {
        return Vec::new();
    };
    if same_folder(&current, folder) {
        return Vec::new();
    }
    // What the worktree chosen may take of the view left is asked while that view is whole, its pages still coming.
    let offer = super::share::offered(state, folder);
    let mut effects = workspace::cancel_reads(state);
    let keep_open = state.settings.keep_open;
    let (left, not_wanted) = kept_on_leaving(take_view(state), keep_open);
    effects.extend(not_wanted.into_iter().map(Effect::Cancel));
    state.dialog = None;
    let Some(tab) = state.tabs.open.get_mut(index) else {
        return effects;
    };
    tab.members.retain(|member| !same_folder(&member.folder, &current));
    if let Some(left) = left {
        tab.members.push(Member {
            folder: current,
            kept: Box::new(left),
            waiting: Vec::new(),
        });
    }
    let kept = tab
        .members
        .iter()
        .position(|member| same_folder(&member.folder, folder))
        .map(|at| tab.members.remove(at));
    let Some(member) = kept else {
        if !same_folder(folder, &tab_folder) {
            state.worktrees.window_folder = Some(tab_folder);
        }
        effects.extend(match offer {
            Some(offer) => super::share::show_shared(state, folder, offer),
            None => workspace::show_folder(state, folder),
        });
        return effects;
    };
    put_view(state, *member.kept);
    effects.push(workspace::watch_settings(state));
    effects.push(workspace::start_read(state, folder.to_path_buf()));
    for message in member.waiting {
        effects.extend(super::super::reduce_message(state, message));
    }
    effects
}

/// The repository of the group of a tab, among those it does not show, that waits for what a message answers.
pub(super) fn member_waiting(members: &[Member], message: &Message) -> Option<usize> {
    let answered = Answered::of(message)?;
    members.iter().position(|member| answered.is_for(&member.kept))
}

/// The folder a view kept shows.
#[must_use]
pub fn kept_folder(view: &RepositoryView) -> Option<&Path> {
    match view.shown.as_ref()? {
        Shown::Read(read) => Some(&read.folder),
        Shown::Reading { folder } => Some(folder),
        Shown::Looking | Shown::NoRepository => None,
    }
}
