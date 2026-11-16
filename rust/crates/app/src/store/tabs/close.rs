//! Closing tabs (`repo.close-tab`): a repository taken out of the window, nothing in it changed. A File page of a tab
//! closed with edits not saved asks first, as closing that page asks.

use std::path::{Path, PathBuf};

use views::file_view::{CloseAnswer, FilePageId};
use views::page::Page;

use super::super::feature::Said;
use super::super::message::Effect;
use super::super::state::State;
use super::super::{pages, workspace};
use super::{Tabs, forget, remember, switch, switch_tab, take_view};
use crate::persisted::same_folder;

/// Tabs being closed, one after the other, while the pages in them with edits not saved ask what to do with them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Closing {
    /// The folders of the tabs still to close, in order.
    folders: Vec<PathBuf>,
    /// The folder of the tab to show once they are closed, when it is not one of them.
    back_to: Option<PathBuf>,
    /// The page asked about last.
    asked: Option<FilePageId>,
    /// The pages whose edits the user chose not to save.
    not_saved: Vec<FilePageId>,
}

/// Closes a tab: the tab to its right is shown, or the one to its left when it was the last; closing the only tab
/// shows the Welcome page. What the tab kept is forgotten and what it waited for cancelled. A File page of the tab with
/// edits not saved asks **Save changes to app.rs?** first, the tab shown while it asks: **Save** saves it, then the
/// tab closes; **Don't save** closes it with the edits lost; **Cancel** keeps the tab. Refused for the tab where an
/// operation runs.
///
/// Feature: repo.close-tab
pub fn close_tab(state: &mut State, index: usize) -> Vec<Effect> {
    let count = state.tabs.open.len();
    if index >= count || state.tabs.closing.is_some() || refuses(state, |shown| shown == index) {
        return Vec::new();
    }
    let back_to = state
        .tabs
        .shown
        .filter(|shown| *shown != index)
        .and_then(|shown| folder_at(state, shown));
    let folders = folder_at(state, index).into_iter().collect();
    start(state, folders, back_to)
}

/// **Close other tabs** in the menu of a tab: that tab is shown, and the others closed, each asking as closing it asks.
pub fn close_other_tabs(state: &mut State, index: usize) -> Vec<Effect> {
    let count = state.tabs.open.len();
    if index >= count || state.tabs.closing.is_some() || refuses(state, |shown| shown != index) {
        return Vec::new();
    }
    let folders = (0..count)
        .filter(|other| *other != index)
        .filter_map(|other| folder_at(state, other))
        .collect();
    let back_to = folder_at(state, index);
    start(state, folders, back_to)
}

/// The answer to **Save changes to app.rs?** asked while tabs close: **Cancel** keeps the tabs not closed yet, **Don't
/// save** lets the tab close with the edits lost.
pub fn close_answered(state: &mut State, page: FilePageId, answer: CloseAnswer) {
    let Some(closing) = state
        .tabs
        .closing
        .as_mut()
        .filter(|closing| closing.asked == Some(page))
    else {
        return;
    };
    match answer {
        CloseAnswer::Cancel => state.tabs.closing = None,
        CloseAnswer::DontSave => closing.not_saved.push(page),
        CloseAnswer::Save => {}
    }
}

/// Goes on closing tabs once the page asked about is answered and saved; a save that failed or was refused keeps the
/// tab, with its edits.
pub fn follow_closing(state: &mut State) -> Vec<Effect> {
    let Some(closing) = state.tabs.closing.as_ref() else {
        return Vec::new();
    };
    if state.file_pages.closing.is_some() || state.file_pages.saving.is_some() || state.is_operation_running {
        return Vec::new();
    }
    if let Some(asked) = closing.asked {
        let is_unsaved = state
            .file_pages
            .pages
            .iter()
            .any(|page| page.id == asked && page.is_unsaved());
        if is_unsaved && !closing.not_saved.contains(&asked) {
            state.tabs.closing = None;
            return Vec::new();
        }
    }
    go_on(state)
}

/// Starts closing tabs.
fn start(state: &mut State, folders: Vec<PathBuf>, back_to: Option<PathBuf>) -> Vec<Effect> {
    state.tabs.closing = Some(Closing {
        folders,
        back_to,
        ..Closing::default()
    });
    go_on(state)
}

/// Closes the tabs still to close, until one has a page to ask about; then shows the tab to show, and remembers the
/// tabs open.
fn go_on(state: &mut State) -> Vec<Effect> {
    let mut effects = Vec::new();
    while let Some(next) = state
        .tabs
        .closing
        .as_ref()
        .map(|closing| closing.folders.first().cloned())
    {
        let Some(folder) = next else {
            let back_to = state.tabs.closing.take().and_then(|closing| closing.back_to);
            if let Some(index) = back_to.and_then(|folder| position(state, &folder)) {
                effects.extend(switch_tab(state, index));
            }
            effects.extend(remember(state));
            return effects;
        };
        if let Some(index) = position(state, &folder) {
            if let Some(page) = unsaved_page(state, index) {
                effects.extend(ask(state, index, page));
                return effects;
            }
            effects.extend(remove_tab(state, index));
        }
        if let Some(closing) = state.tabs.closing.as_mut()
            && !closing.folders.is_empty()
        {
            let _closed = closing.folders.remove(0);
        }
    }
    effects
}

/// A File page of a tab with edits not saved, which the user has not chosen to lose.
fn unsaved_page(state: &State, index: usize) -> Option<FilePageId> {
    let not_saved = state
        .tabs
        .closing
        .as_ref()
        .map_or(&[][..], |closing| closing.not_saved.as_slice());
    let pages = if state.tabs.shown == Some(index) {
        &state.file_pages.pages
    } else {
        &state.tabs.open.get(index)?.kept.as_ref()?.file_pages.pages
    };
    pages
        .iter()
        .find(|page| page.is_unsaved() && !not_saved.contains(&page.id))
        .map(|page| page.id)
}

/// Shows the tab and its page with edits not saved, and asks **Save changes to app.rs?**
fn ask(state: &mut State, index: usize, page: FilePageId) -> Vec<Effect> {
    let effects = switch_tab(state, index);
    if let Some(at) = state.pages.open.iter().position(|open| *open == Page::File(page)) {
        state.pages.shown = Some(at);
    }
    state.file_pages.closing = Some(page);
    if let Some(closing) = state.tabs.closing.as_mut() {
        closing.asked = Some(page);
    }
    effects
}

/// Takes a tab out of the window: the tab to its right is shown in its place, or the one to its left; the only tab
/// closes the workspace.
fn remove_tab(state: &mut State, index: usize) -> Vec<Effect> {
    let count = state.tabs.open.len();
    if count == 1 {
        return close_workspace(state);
    }
    let mut effects = Vec::new();
    if state.tabs.shown == Some(index) {
        let next = if index + 1 < count { index + 1 } else { index - 1 };
        effects.extend(switch(state, next, false));
    }
    effects.extend(remove(state, index));
    effects
}

/// Whether closing is refused because it would close the tab where an operation runs, shown or not: says so.
fn refuses(state: &mut State, closes: impl Fn(usize) -> bool) -> bool {
    if !super::operation_tab(state).is_some_and(closes) {
        return false;
    }
    let title = super::operation_title(state);
    state.operations.said = Some(Said::error(views::text::repo::wait_before_closing(&title)));
    true
}

/// The folder of the tab at a place.
fn folder_at(state: &State, index: usize) -> Option<PathBuf> {
    state.tabs.open.get(index).map(|tab| tab.folder.clone())
}

/// The place of the tab of a folder.
fn position(state: &State, folder: &Path) -> Option<usize> {
    state.tabs.open.iter().position(|tab| same_folder(&tab.folder, folder))
}

/// Takes a tab that is not shown out of the window.
fn remove(state: &mut State, index: usize) -> Vec<Effect> {
    let tab = state.tabs.open.remove(index);
    if let Some(shown) = state.tabs.shown.filter(|shown| *shown > index) {
        state.tabs.shown = Some(shown - 1);
    }
    forget(tab)
}

/// Closing the only tab closes the workspace: the window shows the Welcome page, and the workspace is remembered with
/// no tab open.
fn close_workspace(state: &mut State) -> Vec<Effect> {
    let mut effects = workspace::cancel_reads(state);
    effects.extend(take_view(state).jobs.into_keys().map(Effect::Cancel));
    state.tabs.open.clear();
    state.tabs.shown = None;
    effects.extend(remember(state));
    state.tabs = Tabs::default();
    state.workspace = None;
    state.dialog = None;
    pages::show_page(state, views::page::Page::Welcome);
    effects.push(workspace::watch_settings(state));
    effects
}
