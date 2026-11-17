//! The parts of the state of the views that are made from the store alone: the branch rail, the branch filter of the
//! timeline, what the inspector shows of the row selected and the bar of views.

use model::graph::Row;
use views::details::SelectedDetails;
use views::filter::{BranchPicker, PickerOpen};
use views::graph::Selected;
use views::page::Page;
use views::refs::{ListFilterOpen, ListFilterState, ReferencesState};
use views::{ViewBarState, ViewShown};

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
        typed: &state.rail.typed,
        shown: state.timeline.shown.as_ref(),
        list_filter: ListFilterState {
            listed: state.rail.listed.as_ref(),
            references: &state.rail.references,
            open: state.rail.list_filter.as_ref().map(|picker| ListFilterOpen {
                typed: &picker.typed,
                window: picker.window,
            }),
        },
        selected,
        sort: sort_order(state),
        open_in: state.settings.features.branch_open_in,
        window: super::worktrees::window_of(state),
        stashes: views::refs::StashesShown {
            unfolded: &state.rail.stashes.unfolded,
            files: &state.rail.stashes.files,
            is_operation_running: state.is_operation_running,
            now,
        },
        github: super::github::shown(state),
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

/// The bar of views: the place shown, how many files are changed once a repository is read, and whether the Activity
/// drawer is open.
pub(super) fn view_bar(state: &State) -> ViewBarState {
    let shown = match (state.workspace.is_some(), state.page_shown()) {
        (true, None) => ViewShown::Timeline,
        (_, Some(Page::Changes)) => ViewShown::Changes,
        (_, Some(Page::Files)) => ViewShown::Files,
        (_, Some(Page::Settings)) => ViewShown::Settings,
        (false, None) | (_, Some(_)) => ViewShown::Other,
    };
    ViewBarState {
        shown,
        changed_files: state.shown_read().map(|read| read.state.working_tree.file_count()),
        is_activity_open: state.is_activity_open,
        is_panel_shown: super::panel::look(state).is_shown,
    }
}
