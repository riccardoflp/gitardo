//! The parts of the state of the views that are made from the store alone: the branch rail, the branch filter of the
//! header and what the inspector shows of the row selected.

use model::graph::Row;
use views::details::SelectedDetails;
use views::filter::{BranchPicker, PickerOpen};
use views::graph::Selected;
use views::refs::ReferencesState;

use super::rail::{favorites, sort_order};
use super::state::{RepositoryRead, State};
use super::timeline::Timeline;

/// The branch rail of the repository shown, the ages of its stashes said from a moment in seconds since 1970.
pub(super) fn rail<'a>(state: &'a State, read: &'a RepositoryRead, now: i64) -> ReferencesState<'a> {
    let selected = match &state.timeline.selected {
        Some(Selected::Commit(commit)) => Some(commit),
        _ => None,
    };
    ReferencesState {
        repository: &read.state,
        references: &state.rail.references,
        lists: &state.rail.lists,
        branch_colors: &state.timeline.layout.branch_colors,
        remote_colors: &state.timeline.layout.remote_colors,
        favorites: favorites(state),
        recent: &state.rail.recent,
        submodules: &[],
        typed: &state.rail.typed,
        shown: state.timeline.shown.as_ref(),
        follows_timeline: state.settings.follows_graph_filter,
        selected,
        sort: sort_order(state),
        open_in: state.settings.features.branch_open_in,
        stashes: views::refs::StashesShown {
            unfolded: &state.rail.stashes.unfolded,
            files: &state.rail.stashes.files,
            is_operation_running: state.is_operation_running,
            now,
        },
    }
}

/// The branch filter of the header.
pub(super) fn branch_picker(state: &State) -> BranchPicker<'_> {
    BranchPicker {
        shown: state.timeline.shown.as_ref(),
        references: &state.rail.references,
        open: state.rail.picker.as_ref().map(|picker| PickerOpen {
            typed: &picker.typed,
            window: picker.window,
        }),
        follows_timeline: state.settings.follows_graph_filter,
    }
}

/// What the inspector shows of the row selected, as the timeline read it.
pub(super) fn selected_details(timeline: &Timeline) -> Option<SelectedDetails<'_>> {
    let commit = match timeline.selected.as_ref()? {
        Selected::Draft => return timeline.draft.map(SelectedDetails::Draft),
        Selected::Commit(commit) => commit,
    };
    let row = timeline
        .layout
        .shown_of
        .get(commit)
        .and_then(|shown| timeline.layout.read_of.get(*shown))
        .and_then(|read| timeline.rows.get(*read))
        .filter(|row| row.id() == Some(commit))
        .or_else(|| timeline.rows.iter().find(|row| row.id() == Some(commit)))?;
    match row {
        Row::Commit(commit) => Some(SelectedDetails::Commit(commit)),
        Row::Stash { commit, index } => Some(SelectedDetails::Stash { commit, index: *index }),
        Row::Uncommitted { .. } => None,
    }
}
