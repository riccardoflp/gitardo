//! The timeline (`graph.view`): the commits read by `graph.read-commits`, a row each with its cell of the graph, the
//! labels after the message, the author and the date under it and the hash at the end; the draft on top, the stashes
//! and the folded merges among them; the names of the branches scrolled past at the top, the marks beside the scroll
//! bar and the edge of the graph.

use model::graph::Row;
use ui::layout::{Align, Insets, inset, place};
use ui::{AccessRole, Label, List, Rect, Reveal, Selection, SelectionMode, Steering, Tone, Ui};

use super::commit_menu::{commit_menu, double_click_label, menu_key, open_row_menu};
use super::draft::{draft_entry_intent, draft_menu};
use super::edge::{GraphEdgeState, resize_graph_view};
use super::hover::{HoverState, hover_history_view};
use super::intent::GraphIntent;
use super::keyboard::{LEFT_TO_VIEW, keyboard_view};
use super::load_more::{LoadMoreState, load_more_view};
use super::marks::{ScrollMarks, scroll_marks_view};
use super::row::{RowContext, RowReport, draw_row, select_row, selected_row};
use super::stashes::{stash_intent, stash_menu};
use super::state::{GraphState, HistoryState, Reading};
use super::sticky::{sticky_names, sticky_names_drawn};
use crate::drag::{DropState, Dropped, drop_on_row, drop_view, start_row_drag};
use crate::intent::Intent;
use crate::line::INSPECTOR_OVER_BELOW;
use crate::selection::{Several, bar_height, bar_view, menu_view, open_menu, select_view};
use crate::text::timeline as words;

/// The key of the list of commits.
pub(crate) const HISTORY_KEY: &str = "history";
/// The width of the strip of the marks beside the scroll bar.
const MARKS_WIDTH: f32 = 4.0;
/// How opaque the veil over the rows listed is while the commits of another history are read, out of 255.
const VEIL: u8 = 120;

/// The timeline, over the rectangle its caller clipped it to.
///
/// Feature cards that fill it further: `selection.select` (several commits selected), `filter.bar`, `drag.drop`,
/// `commit-page.open` (what a double click and `Enter` open), `branch.open` (a double click on a label).
///
/// Feature: graph.view
pub fn view(state: &GraphState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let padding = theme.sizes.spacing.large + theme.sizes.spacing.small;
    let inner = inset(
        rect,
        Insets {
            left: padding,
            top: padding,
            right: padding,
            bottom: 0.0,
        },
    );
    ui.region(
        "timeline region",
        (rect, AccessRole::Main, words::TIMELINE),
        |ui| match state {
            GraphState::Loading => {
                say(ui, inner, words::LOADING_COMMITS);
                Vec::new()
            }
            GraphState::Listed(history) if history.layout.is_empty() => {
                let said = if history.has_no_commits {
                    words::NO_COMMITS_YET
                } else {
                    words::NO_COMMITS_TO_SHOW
                };
                say(ui, inner, said);
                Vec::new()
            }
            GraphState::Listed(history) => timeline(history, ui, inner),
        },
    )
}

/// A sentence in the middle of the timeline, in place of the commits.
fn say(ui: &mut Ui<'_>, rect: Rect, words: &str) {
    let height = ui.theme().sizes.row_height;
    let line = place(rect, (rect.width, height), (Align::Center, Align::Center));
    let _ = ui.label(line, &Label::new(words).tone(Tone::Muted).align(Align::Center));
}

/// The commits listed, with everything over and beside them.
fn timeline(state: &HistoryState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let edge = GraphEdgeState {
        lanes: state.layout.lane_count,
        wanted: state.graph_width,
    };
    let fit = edge.fit(rect.width);
    let context = RowContext {
        state,
        graph_width: fit.width,
        lane_width: fit.lane_width,
        is_narrow: ui.window().width < INSPECTOR_OVER_BELOW,
    };
    let several = Several::of(state);
    let (mut intents, list_rect) = selection_bar(state, ui, rect, several.as_ref());
    let (history_intents, report, visible) = history(state, ui, (list_rect, &context), several.as_ref());
    intents.extend(history_intents);
    let rect = list_rect;
    if state.reading != Reading::Nothing {
        ui.running_bar(rect);
    }
    if state.reading == Reading::Again {
        ui.fill_rect(rect, theme.colors.page_background.with_alpha(VEIL));
    }
    let names = sticky_names(state.layout, state.rows, visible.start);
    let sticky_rect = Rect::new(
        rect.x + fit.width + theme.sizes.spacing.large,
        rect.y,
        (rect.width - fit.width - 2.0 * theme.sizes.spacing.large).max(0.0),
        ui.badge_height() + 2.0 * theme.sizes.spacing.small,
    );
    let sticky = ui.clip(sticky_rect, |ui| sticky_names_drawn(&names, ui));
    intents.extend(sticky.intents);
    let hover = HoverState {
        current: state.hover,
        left_at: state.hover_left_at,
        under_pointer: report.label.or(sticky.label).or_else(|| state.rail_hover.clone()),
        now: ui.now(),
    };
    intents.extend(hover_history_view(&hover, ui));
    intents.extend(load_more_view(&load_more_state(state, visible.end), ui));
    intents.extend(keyboard_view(state, ui));
    let marks = ScrollMarks {
        total: state.layout.len() + usize::from(state.has_more),
        checked_out: state
            .layout
            .checked_out
            .map(|row| (row, state.layout.cells.get(row).map_or(0, |cell| cell.color))),
        selected: selected_row(state),
        found: state.found,
        row_height: theme.sizes.timeline_row_height,
    };
    let strip = Rect::new(
        rect.right() - theme.sizes.scrollbar - MARKS_WIDTH,
        rect.y,
        MARKS_WIDTH,
        rect.height,
    );
    intents.extend(ui.clip(strip, |ui| scroll_marks_view(&marks, ui)));
    intents.extend(ui.clip(rect, |ui| resize_graph_view(&edge, ui)));
    intents
}

/// The bar of a selection of several over the commits, when there is one, and the rectangle left to them.
fn selection_bar(
    state: &HistoryState<'_>,
    ui: &mut Ui<'_>,
    rect: Rect,
    several: Option<&Several>,
) -> (Vec<Intent>, Rect) {
    let Some(several) = several else {
        return (Vec::new(), rect);
    };
    let theme = *ui.theme();
    let height = bar_height(ui);
    let gap = theme.sizes.spacing.small;
    let bar = Rect::new(rect.x, rect.y, rect.width - theme.sizes.scrollbar, height);
    let intents = ui.clip(bar, |ui| bar_view(state, several, ui, bar));
    let commits = Rect::new(
        rect.x,
        rect.y + height + gap,
        rect.width,
        (rect.height - height - gap).max(0.0),
    );
    (intents, commits)
}

/// The list of the rows, steered by the selection of the store, with the menus of its rows.
fn history(
    state: &HistoryState<'_>,
    ui: &mut Ui<'_>,
    (rect, context): (Rect, &RowContext<'_>),
    several: Option<&Several>,
) -> (Vec<Intent>, RowReport, std::ops::Range<usize>) {
    let list = List {
        name: words::HISTORY,
        row_count: state.layout.len() + usize::from(state.has_more),
        row_height: ui.theme().sizes.timeline_row_height,
        selection_mode: SelectionMode::Single,
        lead: context.graph_width,
    };
    let selected = selected_row(state);
    let selection = steered_selection(state, selected, several);
    let steering = Steering {
        selection: Some(&selection),
        reveal: state
            .reveal
            .and_then(|(serial, place)| selected.map(|row| Reveal { serial, row, place })),
        left_to_view: &LEFT_TO_VIEW,
    };
    let mut report = RowReport::default();
    let mut dropped = None;
    let response = ui.steered_list(HISTORY_KEY, (rect, &list), &steering, |ui, row| {
        draw_row(ui, context, row, &mut report);
        dropped = drop_on_row(state, ui, row).or(dropped.take());
    });
    let mut intents = std::mem::take(&mut report.intents);
    let chosen = response.changed_selection.as_ref().and_then(Selection::cursor);
    let is_several = several.is_some();
    if let Some(intent) = chosen
        .filter(|row| Some(*row) != selected || is_several)
        .and_then(|row| select_view(state, ui, row).or_else(|| select_row(state, row).map(Intent::Graph)))
    {
        intents.push(intent);
    }
    let pointed = report.pointed.take();
    if let Some(row) = response.activated {
        match double_click_label(state, pointed.as_ref()) {
            Some(intent) => intents.extend(intent),
            None => intents.extend(open_row(state, row)),
        }
    }
    if let Some(row) = response.context_row {
        match several.filter(|several| several.contains_row(state, row)) {
            Some(_) => open_menu(ui),
            None => open_row_menu(state, ui, row, pointed.as_ref()),
        }
    }
    if let Some(several) = several {
        intents.extend(menu_view(state, several, ui));
    }
    intents.extend(drags(state, ui, (&response, pointed.as_ref()), dropped));
    for row in response.visible_rows.clone() {
        intents.extend(row_menu(state, ui, row));
    }
    (intents, report, response.visible_rows)
}

/// The drag started from a row, and what was dropped on one with the menu it opens (`drag.drop`).
fn drags(
    state: &HistoryState<'_>,
    ui: &mut Ui<'_>,
    (response, pointed): (&ui::ListResponse, Option<&super::commit_menu::PointedLabel>),
    dropped: Option<Dropped>,
) -> Vec<Intent> {
    start_row_drag(state, ui, (response, pointed));
    let Some(repository) = state.references.map(|menus| menus.repository) else {
        return Vec::new();
    };
    let shown = DropState {
        dropped: dropped.as_ref().or(state.dropped),
        repository,
    };
    let mut intents = drop_view(&shown, ui);
    intents.extend(dropped.map(|dropped| Intent::Graph(GraphIntent::Dropped(dropped))));
    intents
}

/// The rows the list shows selected: the commits of a selection of several, with the keyboard on the one the
/// inspector shows, or the row selected alone.
fn steered_selection(state: &HistoryState<'_>, selected: Option<usize>, several: Option<&Several>) -> Selection {
    let Some(several) = several else {
        return selected.map_or_default(Selection::single);
    };
    let rows = several
        .commits
        .iter()
        .filter_map(|commit| state.layout.shown_of.get(commit).copied())
        .filter(|row| Some(*row) != selected);
    let mut selection = Selection::default();
    for row in rows.chain(selected) {
        selection.toggle(row);
    }
    selection
}

/// What a double click or `Enter` on a row opens: a commit or a stash as a page; the draft and a folded row open
/// nothing.
fn open_row(state: &HistoryState<'_>, shown: usize) -> Option<Intent> {
    let read = state.layout.read_of.get(shown)?;
    match (state.layout.folded.rows.get(shown)?, state.rows.get(*read)?) {
        (model::graph::ShownRow::Row { .. }, Row::Commit(commit) | Row::Stash { commit, .. }) => {
            Some(Intent::Graph(GraphIntent::Open(commit.id.clone())))
        }
        _ => None,
    }
}

/// The context menu of a row while it is open: the draft's, a stash's, or a commit's, on the commit or on one of
/// its labels.
fn row_menu(state: &HistoryState<'_>, ui: &mut Ui<'_>, shown: usize) -> Vec<Intent> {
    let key = menu_key(shown, None);
    let Some(row) = state.layout.read_of.get(shown).and_then(|read| state.rows.get(*read)) else {
        return Vec::new();
    };
    match row {
        Row::Uncommitted { .. } if ui.is_menu_open(key) => {
            let entries = draft_menu(&state.draft.unwrap_or_default(), state.commit_box.is_running);
            ui.menu(key, &entries)
                .and_then(draft_entry_intent)
                .into_iter()
                .collect()
        }
        Row::Stash { index, .. } if ui.is_menu_open(key) => ui
            .menu(key, &stash_menu())
            .map(|entry| stash_intent(entry, *index))
            .into_iter()
            .collect(),
        Row::Uncommitted { .. } | Row::Stash { .. } => Vec::new(),
        Row::Commit(commit) => commit_menu(state, ui, shown, commit),
    }
}

/// Where the list stands against the commits loaded.
fn load_more_state(state: &HistoryState<'_>, visible_end: usize) -> LoadMoreState {
    let draft = usize::from(state.draft.is_some());
    let loaded = state.rows.len().saturating_sub(draft);
    let last_shown = visible_end.checked_sub(1);
    let last_drawn = last_shown.map(|shown| {
        state
            .layout
            .read_of
            .get(shown)
            .map_or(loaded.saturating_sub(1), |read| read.saturating_sub(draft))
    });
    LoadMoreState {
        loaded,
        last_drawn,
        has_more: state.has_more,
        is_reading: state.reading != Reading::Nothing,
        asked_at: state.asked_at,
    }
}
