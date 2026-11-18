//! Closing with tasks running (`tasks.stop`): a repository tab or the window closed while tasks of theirs run asks
//! **`<n>` tasks are running.** first; **Stop them and close** stops them as **Stop** does and closes once they ended,
//! **Cancel** leaves them running and closes nothing.

use std::path::PathBuf;

use views::Intent;
use views::panel::{PanelIntent, PanelTab};

use super::super::message::Effect;
use super::super::state::State;
use super::super::{pages, panel, tabs, workspace};
use super::runs::stop;
use crate::persisted::WindowPlace;

/// What waits for the answer to **`<n>` tasks are running.**
#[derive(Clone, Debug, PartialEq)]
pub enum Closing {
    /// The window, with where it was when it was closed.
    Window(WindowPlace),
    /// Repository tabs, with what asked to close them.
    Tabs(Intent),
}

/// The folders of the repository tabs an intent closes; none for an intent that closes no tab.
fn tabs_closed(state: &State, intent: &Intent) -> Option<Vec<PathBuf>> {
    let folder_at = |index: usize| state.tabs.open.get(index).map(|tab| tab.folder.clone());
    match intent {
        Intent::CloseTab(index) => Some(folder_at(*index).into_iter().collect()),
        Intent::CloseTabShown => Some(state.tabs.shown.and_then(folder_at).into_iter().collect()),
        Intent::CloseOtherTabs(kept) => {
            let others = (0..state.tabs.open.len()).filter(|index| index != kept);
            Some(others.filter_map(folder_at).collect())
        }
        // The other intents of the tabs close none.
        _ => None,
    }
}

/// The terminals of the tasks that run in some repository tabs, or in all of them.
fn running_in(state: &State, tabs: Option<&[PathBuf]>) -> Vec<u64> {
    let all = state.tasks_tab.tabs.iter();
    let chosen = all.filter(|(key, _)| tabs.is_none_or(|tabs| tabs.contains(key)));
    let entries = chosen.flat_map(|(_, tab)| tab.entries.iter());
    entries
        .filter(|entry| entry.is_running())
        .map(|entry| entry.id)
        .collect()
}

/// How many tasks the question asked now is about.
pub(super) fn asked_about(state: &State) -> Option<usize> {
    let count = match state.tasks_tab.closing.as_ref()? {
        Closing::Window(_) => running_in(state, None).len(),
        Closing::Tabs(intent) => running_in(state, tabs_closed(state, intent).as_deref()).len(),
    };
    Some(count)
}

/// Asks the question: it is drawn by the Tasks tab, which the bottom panel is shown at.
fn ask(state: &mut State, closing: Closing) -> Vec<Effect> {
    state.tasks_tab.closing = Some(closing);
    panel::reduce(state, &PanelIntent::Show(PanelTab::Tasks))
}

/// The window is closed: asked about first while tasks run in any of its repository tabs.
pub fn close_requested(state: &mut State, window: WindowPlace) -> Vec<Effect> {
    if running_in(state, None).is_empty() {
        return workspace::close_requested(state, window);
    }
    ask(state, Closing::Window(window))
}

/// What the island of the tabs asked: closing tabs is asked about first while tasks run in them.
pub fn close_tabs(state: &mut State, intent: Intent) -> Vec<Effect> {
    let closed = tabs_closed(state, &intent);
    if closed.is_some_and(|closed| !running_in(state, Some(&closed)).is_empty()) {
        return ask(state, Closing::Tabs(intent));
    }
    pages::starting_again(state, |state| tabs::reduce(state, intent))
}

/// The answer to **`<n>` tasks are running.**: **Stop them and close** stops the tasks as **Stop** does, then closes
/// what was asked once they ended; **Cancel** leaves them running.
pub(super) fn answered(state: &mut State, is_closing: bool) -> Vec<Effect> {
    let Some(closing) = state.tasks_tab.closing.take() else {
        return Vec::new();
    };
    if !is_closing {
        return Vec::new();
    }
    let closed = match &closing {
        Closing::Window(_) => None,
        Closing::Tabs(intent) => tabs_closed(state, intent),
    };
    let stopped = running_in(state, closed.as_deref());
    let mut effects: Vec<Effect> = Vec::new();
    for tab in state.tasks_tab.tabs.values_mut() {
        effects.extend(stop(tab, |entry| stopped.contains(&entry.id)));
    }
    state.tasks_tab.closing_once_ended = Some((closing, stopped));
    effects.extend(go_on(state));
    effects
}

/// Closes what waited for its tasks to end, once none of them runs.
pub(super) fn go_on(state: &mut State) -> Vec<Effect> {
    let Some((_, stopped)) = &state.tasks_tab.closing_once_ended else {
        return Vec::new();
    };
    let mut entries = state.tasks_tab.tabs.values().flat_map(|tab| tab.entries.iter());
    if entries.any(|entry| entry.is_running() && stopped.contains(&entry.id)) {
        return Vec::new();
    }
    match state.tasks_tab.closing_once_ended.take() {
        Some((Closing::Window(window), _)) => workspace::close_requested(state, window),
        Some((Closing::Tabs(intent), _)) => pages::starting_again(state, |state| tabs::reduce(state, intent)),
        None => Vec::new(),
    }
}
