//! The Merge page (`merge-view.page`): a file in conflict in three columns, the current side, the result and the
//! incoming side, with the base in a fourth at the left when it is shown (`merge-view.show-base`); a band in each gutter
//! for each region, with the buttons that accept a side (`merge-view.take-side`) or what is proposed
//! (`merge-view.accept-proposals`); the bar that goes from region to region and says how many conflicts are left
//! (`merge-view.navigate`) and accepts what needs no choice (`merge-view.accept-simple`); and **Mark as resolved** at
//! the bottom (`merge-view.mark-resolved`).
//!
//! The page is a list of rows of one height, a line of code, with the columns put level in each row, so that they
//! scroll together and only the rows in sight are drawn.

mod bar;
mod columns;
mod data;
mod gutter;
mod question;
mod rows;

use model::merge_view::{Choice, MergeLayout};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, List, Reveal, RevealPlace, SelectionMode, Steering, Ui};

use crate::intent::Intent;
use crate::text::merge_view as words;

pub use bar::{accept_proposals_view, accept_simple_view, navigate_view, show_base_view};
pub use data::{MergeText, Offers, TextLines};
pub use gutter::{GutterSide, TakeSideState, take_side_view};
pub use question::{LargeMergeQuestion, large_merge_question_view};

/// The number of a Merge page, unique in its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergePageId(pub u64);

/// The labels over the columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergeLabels<'a> {
    /// **Current — The branch checked out**.
    pub current: &'a str,
    /// **src/app.rs — Result**.
    pub result: &'a str,
    /// **Incoming — The branch being merged**.
    pub incoming: &'a str,
}

/// What a Merge page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergePageState<'a> {
    /// Which page it is.
    pub id: MergePageId,
    /// The labels over the columns.
    pub labels: MergeLabels<'a>,
    /// The texts and their regions; none while the first read runs.
    pub text: Option<&'a MergeText>,
    /// The rows.
    pub layout: &'a MergeLayout,
    /// Whether the base is shown in a fourth column.
    pub is_base_shown: bool,
    /// The region chosen with the arrows or `F7`, by its place.
    pub current: Option<usize>,
    /// A row to bring to the middle, once for each number.
    pub reveal: Option<(u64, usize)>,
}

/// Something asked of the Merge pages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergeIntent {
    /// Open the Merge page of a file in conflict, by its path from the top of the working tree, or show the one it has.
    Open(String),
    /// Open it, or show it, at the region of the result a line of the file on disk is in, from 1: **Resolve by hand** on
    /// a card of the review of the conflicts, and `conflicts.open-file` from the line number of a card.
    OpenAt(String, usize),
    /// Something asked of a page.
    Page(MergePageId, MergePageIntent),
    /// The answer to the question asked before a very large file: **Continue**, or anything else.
    AnswerLarge(bool),
}

/// Something asked of a Merge page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MergePageIntent {
    /// A button of a band: put a side, or both, in place of a region, by its place.
    Take(usize, Choice),
    /// The button with two ticks: accept every change that needs no choice.
    AcceptSimple,
    /// **✦** on a band: accept what is proposed for its conflict, by the place of the proposal.
    AcceptProposal(usize),
    /// The sparkle: accept every proposal.
    AcceptProposals,
    /// The clock: show the base, or hide it.
    ShowBase(bool),
    /// `F7`, or the arrow ▼.
    Next,
    /// `Shift+F7`, or the arrow ▲.
    Previous,
    /// `Ctrl+Z` in the page: take the last choice back.
    Undo,
    /// `Ctrl+Y` in the page: put it again.
    Redo,
    /// **Mark as resolved**.
    MarkResolved,
}

/// The Merge page, over the rectangle its caller clipped it to: its bar, the labels of its columns, its rows, and
/// **Mark as resolved** under the result.
///
/// Feature: merge-view.page
pub fn page_view(state: &MergePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let mut intents = Vec::new();
    ui.region(("merge page", state.id), (rect, AccessRole::Main, words::PAGE), |ui| {
        let mut area = Area::new(rect);
        let bar = area.top(theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium);
        intents.extend(ui.clip(bar, |ui| bar::bar_view(state, ui)));
        let height = rows::row_height(ui);
        let labels = area.top(height + theme.sizes.spacing.small);
        let bottom = area.bottom(theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium);
        let list_rect = area.rest();
        let columns = columns::Columns::of(list_rect, state.is_base_shown, ui);
        rows::labels(ui, labels, (state, &columns));
        match state.text {
            Some(text) => intents.extend(lines(state, text, ui, (list_rect, &columns))),
            None => rows::loading(ui, list_rect),
        }
        intents.extend(mark_resolved(state, ui, (bottom, &columns)));
        intents.extend(keys(state, ui));
    });
    intents
}

/// The rows of the page, as a list that draws those in sight, with a mark for each region beside its scroll bar.
fn lines(
    state: &MergePageState<'_>,
    text: &MergeText,
    ui: &mut Ui<'_>,
    (rect, columns): (ui::Rect, &columns::Columns),
) -> Vec<Intent> {
    let list = List {
        name: words::LINES,
        row_count: state.layout.rows.len(),
        row_height: rows::row_height(ui),
        selection_mode: SelectionMode::Read,
        lead: 0.0,
    };
    let steering = Steering {
        selection: None,
        reveal: state.reveal.map(|(serial, row)| Reveal {
            serial,
            row,
            place: RevealPlace::Center,
        }),
        left_to_view: &[],
    };
    let mut intents = Vec::new();
    let _ = ui.steered_list(("merge rows", state.id), (rect, &list), &steering, |ui, row| {
        intents.extend(rows::row(ui, (state, text), (row, columns)));
    });
    rows::scroll_marks(ui, (state, text), rect);
    intents
}

/// **Mark as resolved**, at the bottom of the result.
fn mark_resolved(
    state: &MergePageState<'_>,
    ui: &mut Ui<'_>,
    (rect, columns): (ui::Rect, &columns::Columns),
) -> Vec<Intent> {
    let theme = *ui.theme();
    let line = ui::Rect::new(rect.x, rect.y, rect.width, theme.sizes.border);
    ui.separator(line, ui::Axis::Horizontal);
    let under = ui::Rect::new(columns.result.x, rect.y, columns.result.width, rect.height);
    let cell = inset(
        under,
        Insets::symmetric(theme.sizes.spacing.medium, theme.sizes.spacing.medium),
    );
    let button = Button::primary(words::MARK_RESOLVED).with_tooltip(words::MARK_RESOLVED_TOOLTIP);
    let (width, height) = ui.button_size(&button);
    let place_of = place(cell, (width.min(cell.width), height), (Align::End, Align::Center));
    let is_ready = state.text.is_some();
    let button = if is_ready { button } else { button.disabled() };
    if ui.button(("merge mark resolved", state.id), place_of, &button).clicked {
        return vec![Intent::MergeView(MergeIntent::Page(
            state.id,
            MergePageIntent::MarkResolved,
        ))];
    }
    Vec::new()
}

/// The keys of the page that no widget used: `Ctrl+Z` takes the last choice back, `Ctrl+Y` and `Ctrl+Shift+Z` put it
/// again.
fn keys(state: &MergePageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    for key in ui.unclaimed_keys() {
        let ui::Key::Character(character) = key.key else {
            continue;
        };
        if !key.modifiers.command() {
            continue;
        }
        let asked = match (character.to_ascii_lowercase(), key.modifiers.shift()) {
            ('z', false) => MergePageIntent::Undo,
            ('z', true) | ('y', false) => MergePageIntent::Redo,
            _ => continue,
        };
        intents.push(Intent::MergeView(MergeIntent::Page(state.id, asked)));
    }
    intents
}
