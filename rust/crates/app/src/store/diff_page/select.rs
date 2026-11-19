//! The rows selected on a side of a Diff page that is not edited (`diff.select-lines`): from the line chosen to the row
//! `Shift` with a click, an arrow or a drag reaches, cleared by a click elsewhere, `Escape`, or a change of the diff that
//! takes away a row of it.

use model::diff::AlignedRow;
use views::diff::{CaretSide, DiffPageId, FileDiff, PageLine, RowSelection};

use super::edit::PageCaret;
use super::{DiffPage, page_mut};
use crate::store::state::State;

/// The rows selected, and those rows as they were when selected, to tell when the diff takes one of them away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageSelection {
    /// The rows.
    pub rows: RowSelection,
    /// What the rows were.
    pub held: Vec<AlignedRow>,
}

/// The rows selected from the line chosen of the file, or from the row itself when none is, to a row; the line chosen
/// follows to that row and is brought into sight.
pub(super) fn select(state: &mut State, id: DiffPageId, (file, side, row): (usize, CaretSide, usize)) {
    let serial = state.diff_pages.serial + 1;
    let Some(page) = page_mut(state, id) else {
        return;
    };
    let anchor = match (&page.selection, &page.caret) {
        (Some(selection), _) if selection.rows.file == file => selection.rows.anchor,
        (_, Some(PageCaret::Line { file: chosen, row, .. })) if *chosen == file => *row,
        _ => row,
    };
    let rows = RowSelection {
        file,
        anchor,
        head: row,
    };
    page.selection = Some(PageSelection {
        held: held_rows(page, &rows),
        rows,
    });
    page.caret = Some(PageCaret::Line { file, side, row });
    if let Some(place) = page.lines.iter().position(|shown| *shown == PageLine::Row(file, row)) {
        page.caret_reveal = Some((serial, place));
        state.diff_pages.serial = serial;
    }
}

/// The rows of a selection as the diff of its file has them now.
fn held_rows(page: &DiffPage, rows: &RowSelection) -> Vec<AlignedRow> {
    match page.diffs.get(rows.file) {
        Some(FileDiff::Text(text)) => text.aligned.rows.get(rows.rows()).unwrap_or_default().to_vec(),
        _ => Vec::new(),
    }
}

/// The selection cleared when the diff of its file no longer has its rows as they were.
pub(super) fn keep_if_same(page: &mut DiffPage) {
    let is_same = page
        .selection
        .as_ref()
        .is_some_and(|selection| held_rows(page, &selection.rows) == selection.held);
    if !is_same {
        page.selection = None;
    }
}
