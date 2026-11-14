//! The Diff page (`diff.page`): two versions of a file side by side, kept level, with every change marked in the gutter
//! between them; the bar of the page, which goes from change to change (`diff.navigate`) and folds the unchanged lines
//! (`diff.collapse-unchanged`); and the gutter, which is dragged to make one side wider (`diff.resize-sides`).
//!
//! The page is a list of rows of one height, a line of code, so that a diff of 100,000 lines draws only the lines in
//! sight; both sides are drawn in the same rows, so they scroll together.

mod bar;
mod caret;
mod columns;
mod data;
mod edit;
mod gutter;
mod question;
mod rows;
mod targets;

use std::time::Duration;

use ui::layout::Area;
use ui::{AccessRole, List, SelectionMode, Steering, Ui};

use crate::intent::Intent;
use crate::text::diff as words;

pub use bar::{collapse_unchanged_view, navigate_view, open_file_view};
pub use columns::{LEAST_WIDTH, resize_sides_view};
pub use data::{
    CaretSide, ChangePlace, DiffCaret, DiffFile, DiffTarget, DiffText, DiffVersion, FileDiff, Hint, PageLine,
};
pub use edit::{DrawnSides, RECOMPUTE_AFTER, edit_view, open_at_line_view};
pub use question::{LargeDiffQuestion, large_diff_question_view};
pub use targets::{
    changes_file, changes_files, commit_file, commit_files, commit_sides, file_sides, listed_file, untracked_file,
};

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
    /// The versions of the two sides, which decide the buttons of the gutter and of the bar.
    pub versions: (&'a DiffVersion, &'a DiffVersion),
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
    /// A row of the page to bring into sight, once for each number: a change gone to, to the middle; the line of the
    /// caret, scrolled the least.
    pub reveal: Option<ui::Reveal>,
    /// What the bar says for a moment, and until when.
    pub hint: Option<(Hint, Duration)>,
    /// Whether the file of the working tree it shows has edits not saved in a File page (`file-view.edit`).
    pub is_unsaved: bool,
    /// Where the caret is.
    pub caret: Option<DiffCaret<'a>>,
    /// When the text was last edited in this page and the diff not computed again since (`diff.edit`).
    pub edited_at: Option<Duration>,
    /// Whether the rows were moved by an edit and the diff not computed again since: the buttons of the gutter wait.
    pub is_stale: bool,
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
    /// Something asked of the sides of a page (`diff.edit`, `diff.revert-change`).
    Edit(DiffPageId, DiffEditIntent),
}

/// Something asked of the sides of a Diff page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffEditIntent {
    /// What the keys and the pointer asked of the text of the working tree side of a file, in order, at a moment of
    /// the frames.
    Text {
        /// The file, by its place in the page.
        file: usize,
        /// What was asked.
        commands: Vec<ui::editor::Command>,
        /// When.
        at: Duration,
    },
    /// A line chosen on a side that is not edited, by its row among the rows of its file.
    Line {
        /// The file.
        file: usize,
        /// The side.
        side: CaretSide,
        /// The row.
        row: usize,
    },
    /// The diff computed again with the text as it is, a moment after typing stopped.
    Recompute,
    /// `Ctrl+S`: the files of the working tree of the page with edits not saved written.
    Save,
    /// **Revert this change** (`diff.revert-change`): a file, and a change among its changes.
    Revert(usize, usize),
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
        let (row_intents, drawn) = lines(state, ui, (list_rect, &columns));
        intents.extend(row_intents);
        let sides = DrawnSides::new(state, (columns, list_rect), drawn);
        intents.extend(ui.clip(list_rect, |ui| edit_view(&sides, ui)));
        intents.extend(ui.clip(list_rect, |ui| resize_sides_view(state, ui)));
    });
    intents
}

/// The rows of the page, as a list that draws those in sight, with the marks of the changes beside its scroll bar; and
/// the rows drawn, by their place and where they are.
fn lines(
    state: &DiffPageState<'_>,
    ui: &mut Ui<'_>,
    (rect, columns): (ui::Rect, &columns::Columns),
) -> (Vec<Intent>, Vec<(usize, ui::Rect)>) {
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
        reveal: state.reveal,
        left_to_view: &[],
    };
    let mut intents = Vec::new();
    let mut drawn = Vec::new();
    let _ = ui.steered_list(("diff rows", state.id), (rect, &list), &steering, |ui, row| {
        drawn.push((row.index, row.rect));
        intents.extend(rows::row(ui, state, (row, columns)));
    });
    rows::scroll_marks(ui, state, rect);
    (intents, drawn)
}
