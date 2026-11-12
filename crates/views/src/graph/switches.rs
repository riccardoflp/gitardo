//! The switches of the timeline in **…** that change what is read or brought out: only the first parent of merges
//! (`graph.first-parent`) and the history of the selected commit (`graph.trace-selected`). Each writes its setting in
//! the user's settings file through the store.

use ui::Ui;

use super::intent::{GraphIntent, TimelineSwitch};
use crate::intent::Intent;

/// **Show only the first parent of merges**, as it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FirstParentState {
    /// Whether it is on.
    pub is_on: bool,
}

/// What choosing **Show only the first parent of merges** in **…** asks: the setting turned the other way, which
/// reads the timeline again from one page.
///
/// Feature: graph.first-parent
pub fn first_parent_view(state: &FirstParentState, _ui: &mut Ui<'_>) -> Vec<Intent> {
    vec![Intent::Graph(GraphIntent::Switch(
        TimelineSwitch::FirstParent,
        !state.is_on,
    ))]
}

/// **Bring out the history of the selected commit**, as it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedTrace {
    /// Whether it is on.
    pub is_on: bool,
}

/// What choosing **Bring out the history of the selected commit** in **…** asks: the setting turned the other way.
/// The history itself is drawn by the cells of the graph, less strongly than under a label.
///
/// Feature: graph.trace-selected
pub fn trace_selected_view(state: &SelectedTrace, _ui: &mut Ui<'_>) -> Vec<Intent> {
    vec![Intent::Graph(GraphIntent::Switch(
        TimelineSwitch::TraceSelected,
        !state.is_on,
    ))]
}
