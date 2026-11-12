//! The Diff page (`diff.page`): two versions of a file side by side, kept level, with every change marked in the gutter
//! between them; the bar of the page, which goes from change to change (`diff.navigate`) and folds the unchanged lines
//! (`diff.collapse-unchanged`); and the gutter, which is dragged to make one side wider (`diff.resize-sides`).
//!
//! The page is a list of rows of one height, a line of code, so that a diff of 100,000 lines draws only the lines in
//! sight; both sides are drawn in the same rows, so they scroll together.

mod bar;
mod columns;
mod data;
mod question;
mod rows;
mod targets;

use std::time::Duration;

use ui::layout::Area;
use ui::{AccessRole, List, Reveal, RevealPlace, SelectionMode, Steering, Ui};

use crate::intent::Intent;
use crate::text::diff as words;

pub use bar::{collapse_unchanged_view, navigate_view};
pub use columns::{LEAST_WIDTH, resize_sides_view};
pub use data::{ChangePlace, DiffFile, DiffTarget, DiffText, DiffVersion, FileDiff, Hint, PageLine};
pub use question::{LargeDiffQuestion, large_diff_question_view};
pub use targets::{changes_file, commit_file, commit_files, commit_sides, listed_file, untracked_file};

/// The number of a Diff page, unique in its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiffPageId(pub u64);

/// What a Diff page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffPageState<'a> {
    /// Which page it is.
    pub id: DiffPageId,
    /// Its title in the trail.
    pub title: &'a str,
    /// The names over the two columns.
    pub names: (&'a str, &'a str),
    /// The files, one after the other.
    pub files: &'a [DiffFile],
    /// How far the diff of each file is.
    pub diffs: &'a [FileDiff],
    /// The rows of the page.
    pub lines: &'a [PageLine],
    /// Where each change is, in order.
    pub changes: &'a [ChangePlace],
    /// The change chosen with the arrows or `F7`, by its place in `changes`.
    pub current: Option<usize>,
    /// Whether the unchanged rows are folded.
    pub is_collapsed: bool,
    /// The width of the left side as dragged, in whole logical pixels; none for two sides of the same width.
    pub left_width: Option<u32>,
    /// A row of the page to bring to the middle, once for each number.
    pub reveal: Option<(u64, usize)>,
    /// What the bar says for a moment, and until when.
    pub hint: Option<(Hint, Duration)>,
}

/// Something asked of the Diff pages.
#[derive(Clone, Debug, PartialEq)]
pub enum DiffIntent {
    /// Open the Diff page of these files between these versions, or show the one there is.
    Open(Box<DiffTarget>),
    /// Something asked of a page.
    Page(DiffPageId, DiffPageIntent),
    /// The answer to the question asked before a very large diff: **Continue**, or anything else.
    AnswerLarge(bool),
}

/// Something asked of a Diff page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffPageIntent {
    /// `F7`, or the arrow ▼, at a moment of the frames.
    Next(Duration),
    /// `Shift+F7`, or the arrow ▲, at a moment of the frames.
    Previous(Duration),
    /// The fold button of the bar: fold the unchanged rows, or show them all.
    Collapse(bool),
    /// The bar of a fold clicked: show its rows, by the file and its first row.
    Unfold(usize, usize),
    /// The gutter dragged: the width of the left side in whole logical pixels, or none after a double click.
    Resize(Option<u32>),
}

/// The Diff page, over the rectangle its caller clipped it to: its bar, the names of its columns, and its rows.
///
/// Feature: diff.page
pub fn page_view(state: &DiffPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let mut intents = Vec::new();
    ui.region(("diff page", state.id), (rect, AccessRole::Main, words::PAGE), |ui| {
        let mut area = Area::new(rect);
        let bar = area.top(theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium);
        intents.extend(ui.clip(bar, |ui| bar::bar_view(state, ui)));
        let height = rows::row_height(ui);
        let names = area.top(height + theme.sizes.spacing.small);
        let list_rect = area.rest();
        let columns = columns::Columns::of(list_rect, state.left_width, ui);
        rows::names(ui, names, (state, &columns));
        intents.extend(lines(state, ui, (list_rect, &columns)));
        intents.extend(ui.clip(list_rect, |ui| resize_sides_view(state, ui)));
    });
    intents
}

/// The rows of the page, as a list that draws those in sight, with the marks of the changes beside its scroll bar.
fn lines(state: &DiffPageState<'_>, ui: &mut Ui<'_>, (rect, columns): (ui::Rect, &columns::Columns)) -> Vec<Intent> {
    let height = rows::row_height(ui);
    let list = List {
        name: words::LINES,
        row_count: state.lines.len(),
        row_height: height,
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
    let _ = ui.steered_list(("diff rows", state.id), (rect, &list), &steering, |ui, row| {
        intents.extend(rows::row(ui, state, (row, columns)));
    });
    rows::scroll_marks(ui, state, rect);
    intents
}
