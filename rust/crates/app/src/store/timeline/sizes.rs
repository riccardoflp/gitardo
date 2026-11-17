//! The sizes of the panes of the workspace (`graph.resize-panes`, `graph.resize-graph`, `graph.toggle-references`),
//! kept between runs, and the switches of the timeline in **…**, written in the user's settings file.

use views::graph::{GraphIntent, Pane, TimelineSwitch};

use super::super::message::Effect;
use super::super::state::State;
use super::{paging, relayout, retrace};
use crate::persisted::WorkspacePanes;
use crate::persisted::same_folders;
use crate::persisted::timeline::{panes_of, remember};
use crate::settings::switch_setting;

/// The sizes of the panes of the workspace shown, as kept; where they start when none are.
#[must_use]
pub fn workspace_panes(state: &State) -> WorkspacePanes {
    let Some(workspace) = &state.workspace else {
        return WorkspacePanes::default();
    };
    panes_of(&state.persisted.panes, &workspace.folders)
        .cloned()
        .unwrap_or_else(|| WorkspacePanes {
            folders: workspace.folders.clone(),
            ..WorkspacePanes::default()
        })
}

/// A pane resized, the graph's width dragged, or the branch rail put away or brought back: kept for the workspace.
pub fn resize(state: &mut State, intent: &GraphIntent) -> Vec<Effect> {
    if state.workspace.is_none() {
        return Vec::new();
    }
    let mut panes = workspace_panes(state);
    match intent {
        GraphIntent::SetGraphWidth(width) => panes.graph_width = *width,
        GraphIntent::SetPane(Pane::Rail, size) => panes.sizes.rail = *size,
        GraphIntent::SetPane(Pane::Inspector, size) => panes.sizes.inspector = *size,
        GraphIntent::SetPane(Pane::InspectorOver, size) => panes.sizes.inspector_over = *size,
        GraphIntent::SetPane(Pane::FilesTree, size) => panes.sizes.files_tree = *size,
        GraphIntent::ToggleReferences => panes.is_rail_hidden = !panes.is_rail_hidden,
        _ => return Vec::new(),
    }
    let folders = panes.folders.clone();
    remember(&mut state.persisted.panes, panes, |kept| {
        same_folders(&kept.folders, &folders)
    });
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// A switch of **…** turned: written in the user's settings file, and in force at once. Following only the first
/// parent or showing the stashes reads the timeline again from one page; the labels, the separators and all commits on
/// one line lay it out again; the history of the selected commit is brought out or not.
pub fn switch(state: &mut State, switch: TimelineSwitch, is_on: bool) -> Vec<Effect> {
    let switches = state.settings.switches.with(switch, is_on);
    state.settings.switches = switches;
    let mut effects = Vec::new();
    if let Some(file) = state.user_settings_file.clone() {
        let (key, value) = switch_setting(switches, switch);
        effects.push(Effect::WriteSetting { file, key, value });
    }
    effects.extend(apply_switch(state, switch));
    effects
}

/// What a switch changes in the timeline, once in force.
pub fn apply_switch(state: &mut State, switch: TimelineSwitch) -> Vec<Effect> {
    match switch {
        TimelineSwitch::FirstParent | TimelineSwitch::ShowStashes => paging::read_again(state),
        TimelineSwitch::CombineUpstream | TimelineSwitch::DateSeparators | TimelineSwitch::OneLane => {
            relayout(state);
            Vec::new()
        }
        TimelineSwitch::TraceSelected => {
            retrace(state);
            Vec::new()
        }
        TimelineSwitch::Author | TimelineSwitch::Date | TimelineSwitch::Hash => Vec::new(),
    }
}
