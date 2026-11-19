//! The list of all the labels of a commit, under the pointer (`graph.label-overflow`): when a row does not show all its
//! labels whole, resting on one of them opens under it the list of all of them, each with its icon and its color, each
//! doing what it does on the row: a click selects the commit, a double click checks a branch out or does what
//! `branch.open` does, a right click opens its menu, and resting on it brings out its history.

use std::time::Duration;

use model::CommitId;
use model::graph::Label;
use ui::layout::{Insets, inset};
use ui::{Key, List, NamedKey, Popover, PopoverSide, Rect, Selection, SelectionMode, Sense, Steering, Ui};

use super::commit_menu::{double_click_label, open_row_menu};
use super::intent::{GraphIntent, HoverTarget, Selected};
use super::labels::{hover_target, pill};
use super::pull_request_badge::{after_label, badge_room};
use super::state::HistoryState;
use crate::intent::Intent;
use crate::text::timeline as words;

/// How long the pointer rests on a label of a row before the list of all its labels opens (Q642).
pub const OPEN_AFTER: Duration = Duration::from_millis(300);

/// How wide the list is at most; a longer label is shortened in it.
const MOST_WIDTH: f32 = 360.0;

/// The list of the labels of a commit, open.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LabelList {
    /// The commit.
    pub commit: CommitId,
    /// The place of the label it opened from among those drawn on the row, `+N` last.
    pub from: usize,
}

/// The list open, and where it hangs from.
#[derive(Clone, Copy, Debug)]
pub struct LabelOverflow<'s, 'a> {
    /// The timeline.
    pub history: &'s HistoryState<'a>,
    /// The list.
    pub list: &'s LabelList,
    /// The row of its commit, from the top.
    pub shown: usize,
    /// The label it opened from, which it never covers.
    pub opener: Rect,
    /// All the labels drawn on the row: the pointer on them keeps the list open.
    pub labels: Rect,
}

/// What the list reports.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LabelListReport {
    /// The intents.
    pub intents: Vec<Intent>,
    /// The label of the list under the pointer, with since when it rests there.
    pub label: Option<(HoverTarget, Duration)>,
}

/// The list of all the labels of a commit under the label it opened from, or above it when there is more room there; it
/// asks to close when the pointer leaves it and the labels of the row, on `Escape`, or on a press elsewhere.
///
/// Feature: graph.label-overflow
pub fn label_overflow_view(state: &LabelOverflow<'_, '_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    label_list_drawn(state, ui).intents
}

/// The list drawn, with what it reports.
pub(crate) fn label_list_drawn(state: &LabelOverflow<'_, '_>, ui: &mut Ui<'_>) -> LabelListReport {
    let history = state.history;
    let labels = history
        .layout
        .labels
        .get(&state.list.commit)
        .map_or(&[][..], Vec::as_slice);
    let color = history.layout.cells.get(state.shown).map_or(0, |cell| cell.color);
    let theme = *ui.theme();
    let sizes = theme.sizes;
    let widest = labels
        .iter()
        .map(|label| {
            let at = history.tag_marks.map(|marks| (marks, &state.list.commit));
            ui.badge_width(&pill(label, at).badge(color)) + badge_room(ui, history, label, color)
        })
        .fold(0.0_f32, f32::max);
    let padding = sizes.spacing.small;
    let width = (widest + 2.0 * (sizes.spacing.medium + padding) + sizes.scrollbar).min(MOST_WIDTH);
    let height = sizes.row_height * crate::line::rows_of(labels.len()) + 2.0 * padding;
    let popover = Popover {
        anchor: state.opener,
        size: (width, height),
        name: words::LABELS,
        side: PopoverSide::BelowOrAbove,
    };
    let key = ("label list", state.list.commit.as_str());
    let answer = ui.popover(key, &popover, |ui, rect, _| {
        list_rows(ui, state, (labels, color), inset(rect, Insets::all(padding)))
    });
    let (mut report, is_done, menu) = answer.inner;
    // The menu is opened out of the list, where the timeline looks for the menus of its rows.
    if let Some(index) = menu {
        open_row_menu(history, ui, state.shown, Some(&(state.list.commit.clone(), index)));
    }
    let is_left = ui.pointer().is_none_or(|point| {
        let between = between(state.opener, answer.rect);
        !answer.rect.contains(point) && !state.labels.contains(point) && !between.contains(point)
    });
    let is_escaped = ui
        .key_presses()
        .iter()
        .any(|key| key.key == Key::Named(NamedKey::Escape));
    if is_done || menu.is_some() || is_escaped || answer.is_dismissed || (is_left && !answer.is_new) {
        report.intents.push(Intent::Graph(GraphIntent::CloseLabels(ui.now())));
    }
    report
}

/// The column between the label a list opened from and the list, over the gap between them, which the pointer crosses
/// without closing the list.
fn between(opener: Rect, list: Rect) -> Rect {
    let top = opener.y.min(list.y);
    let bottom = opener.bottom().max(list.bottom());
    Rect::new(opener.x, top, opener.width, bottom - top)
}

/// The labels in a list that scrolls when it is taller than its room, each doing what it does on the row; gives what
/// they report, whether a label was opened, and the label whose menu was asked for.
fn list_rows(
    ui: &mut Ui<'_>,
    state: &LabelOverflow<'_, '_>,
    (labels, color): (&[Label], usize),
    rect: Rect,
) -> (LabelListReport, bool, Option<usize>) {
    let history = state.history;
    let commit = &state.list.commit;
    let row_height = ui.theme().sizes.row_height;
    let list = List {
        name: words::LABELS,
        row_count: labels.len(),
        row_height,
        selection_mode: SelectionMode::Single,
        lead: 0.0,
    };
    // Nothing stays selected in the list: a click on a label selects its commit, as on the row.
    let none = Selection::default();
    let steering = Steering {
        selection: Some(&none),
        reveal: None,
        left_to_view: &[],
    };
    let mut report = LabelListReport::default();
    let response = ui.steered_list("labels", (rect, &list), &steering, |ui, row| {
        listed_label(ui, history, (commit, labels, color), row, &mut report);
    });
    if response.changed_selection.is_some() && history.selected != Some(&Selected::Commit(commit.clone())) {
        report
            .intents
            .push(Intent::Graph(GraphIntent::Select(Selected::Commit(commit.clone()))));
    }
    let mut is_done = false;
    if let Some(index) = response.activated {
        let pointed = (commit.clone(), index);
        let opened = double_click_label(history, Some(&pointed))
            .unwrap_or_else(|| vec![Intent::Graph(GraphIntent::Open(commit.clone()))]);
        report.intents.extend(opened);
        is_done = true;
    }
    (report, is_done, response.context_row)
}

/// A label of the list on its row, with the badge of its pull request; resting on the row brings out its history.
fn listed_label(
    ui: &mut Ui<'_>,
    history: &HistoryState<'_>,
    (commit, labels, color): (&CommitId, &[Label], usize),
    row: &ui::Row,
    report: &mut LabelListReport,
) {
    let Some(label) = labels.get(row.index) else {
        return;
    };
    let gap = ui.theme().sizes.spacing.medium;
    let place = Rect::new(
        row.rect.x + gap,
        row.rect.y,
        (row.rect.width - 2.0 * gap).max(0.0),
        row.rect.height,
    );
    let at = history.tag_marks.map(|marks| (marks, commit));
    let drawn = ui.badge(place, &pill(label, at).badge(color));
    let end = place.right();
    let _next = after_label(
        ui,
        history,
        (labels, row.index),
        (drawn, color, end),
        &mut report.intents,
    );
    let id = ui.id(("listed label", row.index));
    let hovered = ui.interact(id, row.rect, Sense::HOVER);
    if let Some(since) = ui.hovered_since(&hovered) {
        report.label = Some((hover_target(ui, commit), since));
    }
}
