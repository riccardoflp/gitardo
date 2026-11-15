//! Closing tabs (`repo.close-tab`): a repository taken out of the window, nothing in it changed.

use super::super::feature::Said;
use super::super::message::Effect;
use super::super::state::State;
use super::super::{pages, workspace};
use super::{Tabs, forget, remember, switch, take_view};

/// Closes a tab: the tab to its right is shown, or the one to its left when it was the last; closing the only tab
/// shows the Welcome page. What the tab kept is forgotten and what it waited for cancelled. Refused for the tab where
/// an operation runs.
///
/// Feature: repo.close-tab
pub fn close_tab(state: &mut State, index: usize) -> Vec<Effect> {
    let count = state.tabs.open.len();
    if index >= count || refuses(state, |shown| shown == index) {
        return Vec::new();
    }
    if count == 1 {
        return close_workspace(state);
    }
    let mut effects = Vec::new();
    if state.tabs.shown == Some(index) {
        let next = if index + 1 < count { index + 1 } else { index - 1 };
        effects.extend(switch(state, next, false));
    }
    effects.extend(remove(state, index));
    effects.extend(remember(state));
    effects
}

/// **Close other tabs** in the menu of a tab: that tab is shown, and the others closed.
pub fn close_other_tabs(state: &mut State, index: usize) -> Vec<Effect> {
    let count = state.tabs.open.len();
    if index >= count || refuses(state, |shown| shown != index) {
        return Vec::new();
    }
    let mut effects = Vec::new();
    if state.tabs.shown != Some(index) {
        effects.extend(switch(state, index, false));
    }
    for other in (0..count).rev().filter(|other| *other != index) {
        effects.extend(remove(state, other));
    }
    effects.extend(remember(state));
    effects
}

/// Whether closing is refused because it would close the tab where an operation runs, which is the tab shown: says
/// so.
fn refuses(state: &mut State, closes_shown: impl Fn(usize) -> bool) -> bool {
    if !state.is_operation_running || !state.tabs.shown.is_some_and(closes_shown) {
        return false;
    }
    let title = state.operations.running.as_ref().map_or_else(String::new, |running| {
        running.title.clone().unwrap_or_else(|| running.feature.0.to_owned())
    });
    state.operations.said = Some(Said::error(views::text::repo::wait_before_closing(&title)));
    true
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
