//! The parts of the state of the views that are made from the store alone: the branch rail, the branch filter of the
//! timeline, what the inspector shows of the row selected, the bar of views and the way **Pull** takes.

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
        tag_marks: state.rail.tag_marks.get(&read.folder),
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

/// The bar of views: the place shown, how many files are changed once a repository is read, and whether the bottom
/// panel shows its Activity tab or another.
pub(super) fn view_bar(state: &State) -> ViewBarState {
    let shown = match (state.workspace.is_some(), state.page_shown()) {
        (true, None) => ViewShown::Timeline,
        (_, Some(Page::Changes)) => ViewShown::Changes,
        (_, Some(Page::Files)) => ViewShown::Files,
        (_, Some(Page::Search)) => ViewShown::Search,
        (_, Some(Page::Settings)) => ViewShown::Settings,
        (false, None) | (_, Some(_)) => ViewShown::Other,
    };
    ViewBarState {
        shown,
        changed_files: state.shown_read().map(|read| read.state.working_tree.file_count()),
        is_activity_shown: super::panel::is_activity_shown(state),
        is_panel_shown: super::panel::look(state).is_shown && !super::panel::is_activity_shown(state),
    }
}

/// The place among the ways to pull of the menu of **Sync** of the way **Pull** takes, as the settings say.
pub(super) fn pull_way(settings: &model::contract::FeatureSettings) -> usize {
    let way = features::sync::default_way(settings);
    views::graph::PULL_WAYS
        .iter()
        .position(|(choice, _, _)| *choice == way)
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::pull_way;
    use model::contract::FeatureSettings;

    #[test]
    fn sync_pull_with__with_pull_rebase_off_the_menu_marks_pull_autostash_as_the_default() {
        let settings = FeatureSettings {
            pull_rebase: false,
            ..FeatureSettings::default()
        };
        let marked = views::graph::PULL_WAYS
            .get(pull_way(&settings))
            .map(|(_, label, _)| *label);
        assert_eq!(marked, Some(views::text::header::PULL_AUTOSTASH));
        let both = views::graph::PULL_WAYS.get(pull_way(&FeatureSettings::default()));
        assert_eq!(both.map(|(choice, ..)| *choice), Some("rebase-autostash"));
    }

    #[test]
    fn the_ways_of_the_menu_are_the_choices_of_pull_with() {
        let menu: Vec<&str> = views::graph::PULL_WAYS.iter().map(|(choice, ..)| *choice).collect();
        let feature: Vec<&str> = features::sync::WAYS.iter().map(|(choice, ..)| *choice).collect();
        assert_eq!(menu, feature);
    }
}
