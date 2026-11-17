//! The island between the branch rail and the inspector: the search, the banners and the commits, or the Diff
//! page beside the inspector (`diff.beside-inspector`).

use ui::layout::{Area, Insets, inset};
use ui::{Rect, Ui};

use crate::bisect::banner_view;
use crate::filter::{BranchPicker, bar_height, bar_view, picker_button_width, picker_view};
use crate::graph::{GraphState, HoverTarget, view as graph_view};
use crate::intent::Intent;
use crate::line::control_in;
use crate::search::{SearchBox, list_mode_view};

use super::{SEARCH_BOX_ROOM, TimelineState};

/// What the island between the branch rail and the inspector holds: the Diff page beside the inspector when a file of
/// it is open (`diff.beside-inspector`), else the search, the banners and the commits.
pub(super) fn main_island(
    state: &TimelineState<'_>,
    rail_hover: Option<(HoverTarget, std::time::Duration)>,
    ui: &mut Ui<'_>,
    rect: Rect,
    intents: &mut Vec<Intent>,
) {
    let TimelineState::Repository {
        graph,
        search,
        filter,
        beside,
        ..
    } = state
    else {
        return;
    };
    if let Some(beside) = beside {
        intents.extend(crate::diff::beside_inspector_view(beside, ui));
        return;
    }
    let main = search_row((&search.0, filter), ui, rect, intents);
    let mut main = Area::new(main);
    banners(state, ui, &mut main, intents);
    let graph = with_rail_hover(graph, rail_hover);
    intents.extend(ui.clip(main.rest(), |ui| graph_view(&graph, ui)));
}

/// The bars above the commits, each the height of one: the list mode of the search or what the timeline is narrowed
/// to, then the banner of an operation stopped, then that of a bisect.
pub(super) fn banners(state: &TimelineState<'_>, ui: &mut Ui<'_>, main: &mut Area, intents: &mut Vec<Intent>) {
    let TimelineState::Repository {
        bar,
        search,
        conflicts,
        bisect,
        worktree,
        ..
    } = state
    else {
        return;
    };
    if let Some(list) = &search.1 {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| list_mode_view(list, ui)));
    } else if let Some(bar) = bar {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| bar_view(bar, ui)));
    }
    if let Some(banner) = worktree {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| crate::worktree::banner(banner, ui)));
    }
    if let Some(banner) = conflicts {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| crate::conflicts::banner_view(banner, ui)));
    }
    if let Some(banner) = bisect {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| banner_view(banner, ui)));
    }
}

/// The timeline with the reference of the branch rail the pointer rests on, whose history it brings out.
pub(super) fn with_rail_hover<'a>(
    graph: &GraphState<'a>,
    hover: Option<(HoverTarget, std::time::Duration)>,
) -> GraphState<'a> {
    match (graph, hover) {
        (GraphState::Listed(history), Some(hover)) => {
            let mut history = history.as_ref().clone();
            history.rail_hover = Some(hover);
            GraphState::Listed(Box::new(history))
        }
        _ => graph.clone(),
    }
}

/// The branch filter and the search box at the top of the timeline; gives what is left under them, for the bar of what
/// the timeline is narrowed to and the commits.
pub(super) fn search_row(
    (search_box, filter): (&SearchBox<'_>, &BranchPicker<'_>),
    ui: &mut Ui<'_>,
    main: Rect,
    intents: &mut Vec<Intent>,
) -> Rect {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::new(main);
    let top = area.top(theme.sizes.control_height + 2.0 * spacing.medium);
    let row = inset(top, Insets::symmetric(spacing.medium, spacing.medium));
    let mut parts = Area::with_gap(row, spacing.medium);
    // The filter gives up its words where the search box would have less than its room beside them.
    let words = picker_button_width(filter, ui);
    let width = if row.width - words < SEARCH_BOX_ROOM {
        theme.sizes.control_height
    } else {
        words
    };
    let cell = control_in(ui, parts.left(width), width);
    intents.extend(ui.clip(cell, |ui| picker_view(filter, ui)));
    intents.extend(ui.clip(parts.rest(), |ui| crate::search::box_view(search_box, ui)));
    area.rest()
}
