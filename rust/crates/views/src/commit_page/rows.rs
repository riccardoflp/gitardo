//! The rows of the page of a commit: its head as a block of rows, a row for each file, the heading of the changes, and
//! for each diff shown its header, its hunk headers and its lines, or the note that says why it has none.

use model::commit_page::{FilePatch, PatchLines};
use ui::{Rect, Ui};

use super::diff::{self, Drawn};
use super::head::{self, PageHead};
use super::{CommitPageState, PageData, PageTarget};
use crate::blocks::{block_top, rows_for};
use crate::details::{FileRow, FilesState};
use crate::intent::Intent;

/// How many rows the header of a diff takes.
pub(crate) const HEADER_ROWS: usize = 2;
/// How many rows the note of a diff with no lines takes.
pub(crate) const NOTE_ROWS: usize = 2;
/// How many rows the heading of the changes takes.
const CHANGES_ROWS: usize = 3;

/// A row of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageRow<'a> {
    /// A row of the head, drawn once from the first of them in sight.
    Head,
    /// A directory or a file of the list.
    File(FileRow<'a>),
    /// **Show all N files…**.
    ShowAll,
    /// **N untracked files**, over the untracked files of a stash.
    UntrackedHeading,
    /// An untracked file of a stash.
    Untracked(usize),
    /// A row of the heading of the changes, by its first row.
    Changes(usize),
    /// A row of the header of the diff of a file, by its first row.
    DiffHeader(usize, usize),
    /// The header of a hunk of a file.
    Hunk(usize, usize),
    /// A line of a hunk of a file.
    Line(usize, usize, usize),
    /// A row of the note of a diff with no lines, by its first row.
    Note(usize, usize),
    /// A row of what the page says when the lines cannot be matched to the files, by its first row.
    Unmatched(usize),
}

/// The page laid out for a frame.
pub(crate) struct Page<'a> {
    /// What it shows.
    pub(crate) state: &'a CommitPageState<'a>,
    /// Its commit.
    pub(crate) data: &'a PageData,
    /// Its files.
    pub(crate) files: &'a FilesState<'a>,
    /// Its rows.
    pub(crate) rows: Vec<PageRow<'a>>,
    /// The height of each row.
    pub(crate) height: f32,
}

/// The rows of a page, for its width.
pub(crate) fn page_rows<'a>(
    state: &CommitPageState<'a>,
    data: &'a PageData,
    (ui, width): (&Ui<'_>, f32),
    files: &[FileRow<'a>],
) -> Vec<PageRow<'a>> {
    let height = super::row_height(ui);
    let head = PageHead { state, data };
    let mut rows = vec![PageRow::Head; rows_for(head::head_height(ui, &head, width), height)];
    rows.extend(files.iter().copied().map(PageRow::File));
    if !state.is_all_files_shown && data.detail.files.len() > crate::details::FILE_CAP {
        rows.push(PageRow::ShowAll);
    }
    if !data.detail.untracked_files.is_empty() {
        rows.push(PageRow::UntrackedHeading);
        rows.extend((0..data.detail.untracked_files.len()).map(PageRow::Untracked));
    }
    let changes = rows.len();
    rows.extend((0..CHANGES_ROWS).map(|_| PageRow::Changes(changes)));
    if state.shows_changes {
        diff_rows(state, data, &mut rows);
    }
    rows
}

/// The rows of the diffs of the files shown.
fn diff_rows<'a>(state: &CommitPageState<'a>, data: &'a PageData, rows: &mut Vec<PageRow<'a>>) {
    let patches = match &data.patch {
        PatchLines::Files(patches) => patches,
        PatchLines::Unmatched => {
            let start = rows.len();
            rows.extend((0..NOTE_ROWS).map(|_| PageRow::Unmatched(start)));
            return;
        }
    };
    let shown = if state.is_all_files_shown {
        patches.len()
    } else {
        patches.len().min(crate::details::FILE_CAP)
    };
    for (index, patch) in patches.iter().enumerate().take(shown) {
        let start = rows.len();
        rows.extend((0..HEADER_ROWS).map(|_| PageRow::DiffHeader(index, start)));
        if state.folds.is_folded(index) {
            continue;
        }
        if patch.hunks.is_empty() {
            let note = rows.len();
            rows.extend((0..NOTE_ROWS).map(|_| PageRow::Note(index, note)));
            continue;
        }
        for (hunk_index, hunk) in patch.hunks.iter().enumerate() {
            rows.push(PageRow::Hunk(index, hunk_index));
            rows.extend((0..hunk.lines.len()).map(|line| PageRow::Line(index, hunk_index, line)));
        }
    }
}

impl Page<'_> {
    /// The diff of a file, by its place.
    pub(crate) fn patch(&self, index: usize) -> Option<&FilePatch> {
        match &self.data.patch {
            PatchLines::Files(patches) => patches.get(index),
            PatchLines::Unmatched => None,
        }
    }

    /// The row a target of the page starts at.
    pub(crate) fn row_of(&self, target: PageTarget) -> usize {
        match target {
            PageTarget::Top => 0,
            PageTarget::Diff(file) => self
                .rows
                .iter()
                .position(|row| matches!(row, PageRow::DiffHeader(index, _) if *index == file))
                .unwrap_or(0),
        }
    }

    /// The text find in page sees in a row: the message on the first row of the head, the path of a file and of a
    /// diff, and the text of each line.
    pub(crate) fn row_text(&self, index: usize) -> String {
        match self.rows.get(index) {
            Some(PageRow::Head) if index == 0 => format!("{}\n{}", self.data.title, self.data.text),
            Some(PageRow::File(FileRow::File(file, _))) => self.path_of(*file),
            Some(PageRow::DiffHeader(file, start)) if *start == index => self.path_of(*file),
            Some(PageRow::Line(file, hunk, line)) => self
                .patch(*file)
                .and_then(|patch| patch.hunks.get(*hunk))
                .and_then(|hunk| hunk.lines.get(*line))
                .map_or_default(|line| line.text.clone()),
            _ => String::new(),
        }
    }

    /// The path of a file of the commit.
    fn path_of(&self, file: usize) -> String {
        self.data
            .detail
            .files
            .get(file)
            .map_or_default(|file| file.path.clone())
    }

    /// Draws a row, and the block it starts, and gives what was asked.
    pub(crate) fn draw_row(&self, ui: &mut Ui<'_>, row: &ui::Row, drawn: &mut Drawn) -> Vec<Intent> {
        let is_first = drawn.note(row);
        let block = |start: usize| block_top(row, start, is_first, self.height);
        match self.rows.get(row.index).copied() {
            Some(PageRow::Head) => block(0).map_or_default(|top| {
                head::draw_head(
                    ui,
                    &PageHead {
                        state: self.state,
                        data: self.data,
                    },
                    top,
                )
            }),
            Some(PageRow::Changes(start)) => block(start).map_or_default(|top| head::changes(ui, self, top)),
            Some(PageRow::DiffHeader(file, start)) => {
                drawn.header(
                    file,
                    row.rect.y - crate::number::float_of_count(row.index - start) * self.height,
                );
                block(start).map_or_default(|top| diff::header(ui, self, (file, self.block(top, HEADER_ROWS))))
            }
            Some(PageRow::Note(file, start)) => {
                block(start).map_or_default(|top| diff::note(ui, self, (Some(file), self.block(top, NOTE_ROWS))))
            }
            Some(PageRow::Unmatched(start)) => {
                block(start).map_or_default(|top| diff::note(ui, self, (None, self.block(top, NOTE_ROWS))))
            }
            Some(other) => diff::simple_row(ui, self, (row, other)),
            None => Vec::new(),
        }
    }

    /// The rectangle of a block of rows from its top.
    fn block(&self, top: Rect, rows: usize) -> Rect {
        Rect::new(
            top.x,
            top.y,
            top.width,
            crate::number::float_of_count(rows) * self.height,
        )
    }
}
