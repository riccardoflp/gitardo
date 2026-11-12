//! The timeline (`graph.view`): the commits read by `graph.read-commits`, a row each with its cell of the graph, the
//! labels after the message, the author and the date under it and the hash at the end; the draft on top, the stashes
//! and the folded merges among them; the names of the branches scrolled past at the top, the marks beside the scroll
//! bar and the edge of the graph.

use model::graph::Row;
use ui::layout::{Align, Insets, inset, place};
use ui::{AccessRole, Label, List, MenuAnchor, Rect, Reveal, Selection, SelectionMode, Steering, Tone, Ui};

use super::draft::draft_menu;
use super::edge::{GraphEdgeState, resize_graph_view};
use super::fold::{MergeEntry, merge_menu};
use super::hover::{HoverState, hover_history_view};
use super::intent::GraphIntent;
use super::keyboard::{LEFT_TO_VIEW, keyboard_view};
use super::load_more::{LoadMoreState, load_more_view};
use super::marks::{ScrollMarks, scroll_marks_view};
use super::row::{RowContext, RowReport, draw_row, select_row, selected_row};
use super::stashes::stash_menu;
use super::state::{GraphState, HistoryState, Reading};
use super::sticky::{sticky_names, sticky_names_drawn};
use crate::intent::Intent;
use crate::line::INSPECTOR_OVER_BELOW;
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
    let (mut intents, report, visible) = history(state, ui, (rect, &context));
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
        under_pointer: report.label.or(sticky.label),
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

/// The list of the rows, steered by the selection of the store, with the menus of its rows.
fn history(
    state: &HistoryState<'_>,
    ui: &mut Ui<'_>,
    (rect, context): (Rect, &RowContext<'_>),
) -> (Vec<Intent>, RowReport, std::ops::Range<usize>) {
    let list = List {
        name: words::HISTORY,
        row_count: state.layout.len() + usize::from(state.has_more),
        row_height: ui.theme().sizes.timeline_row_height,
        selection_mode: SelectionMode::Single,
        lead: context.graph_width,
    };
    let selected = selected_row(state);
    let selection = selected.map(Selection::single).unwrap_or_default();
    let steering = Steering {
        selection: Some(&selection),
        reveal: state
            .reveal
            .and_then(|(serial, place)| selected.map(|row| Reveal { serial, row, place })),
        left_to_view: &LEFT_TO_VIEW,
    };
    let mut report = RowReport::default();
    let response = ui.steered_list(HISTORY_KEY, (rect, &list), &steering, |ui, row| {
        draw_row(ui, context, row, &mut report);
    });
    let mut intents = std::mem::take(&mut report.intents);
    let chosen = response.changed_selection.as_ref().and_then(Selection::cursor);
    if let Some(intent) = chosen
        .filter(|row| Some(*row) != selected)
        .and_then(|row| select_row(state, row))
    {
        intents.push(Intent::Graph(intent));
    }
    if let Some(row) = response.activated {
        intents.extend(open_row(state, row));
    }
    if let Some(row) = response.context_row {
        ui.open_menu(("row menu", row), MenuAnchor::Pointer);
    }
    for row in response.visible_rows.clone() {
        intents.extend(row_menu(state, ui, row));
    }
    (intents, report, response.visible_rows)
}

/// What a double click or `Enter` on a row opens: a commit as a page; the draft, a stash and a folded row open
/// nothing.
fn open_row(state: &HistoryState<'_>, shown: usize) -> Option<Intent> {
    let read = state.layout.read_of.get(shown)?;
    match (state.layout.folded.rows.get(shown)?, state.rows.get(*read)?) {
        (model::graph::ShownRow::Row { .. }, Row::Commit(commit)) => {
            Some(Intent::Graph(GraphIntent::Open(commit.id.clone())))
        }
        _ => None,
    }
}

/// The context menu of a row while it is open: the draft's, a stash's, or a merge's with its fold.
fn row_menu(state: &HistoryState<'_>, ui: &mut Ui<'_>, shown: usize) -> Vec<Intent> {
    let key = ("row menu", shown);
    if !ui.is_menu_open(key) {
        return Vec::new();
    }
    let Some(row) = state.layout.read_of.get(shown).and_then(|read| state.rows.get(*read)) else {
        return Vec::new();
    };
    match row {
        Row::Uncommitted { .. } => {
            let _ = ui.menu(key, &draft_menu(&state.draft.unwrap_or_default()));
            Vec::new()
        }
        Row::Stash { .. } => {
            let _ = ui.menu(key, &stash_menu());
            Vec::new()
        }
        Row::Commit(commit) => {
            let is_folded = state.folds.folded.contains(&commit.id);
            let entries = merge_menu(state.layout.foldable.contains(&commit.id), is_folded);
            let intent = match ui.menu(key, &entries) {
                Some(MergeEntry::Fold) => GraphIntent::Fold(commit.id.clone()),
                Some(MergeEntry::Unfold) => GraphIntent::Unfold(commit.id.clone()),
                None => return Vec::new(),
            };
            vec![Intent::Graph(intent)]
        }
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
