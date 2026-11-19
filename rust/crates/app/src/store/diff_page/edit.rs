//! The working tree side of a Diff page edited where it is shown (`diff.edit`): its text is that of the File page of
//! the file, shown or held out of sight; the rows follow each edit at once, the diff is computed again a moment after
//! typing stops, and `Ctrl+S` runs `file-view.save`. **Revert this change** (`diff.revert-change`) is one more edit of
//! that text, saved at once, which `Ctrl+Z` takes back, and so is **Revert lines** (`diff.revert-lines`).

use std::ops::Range;
use std::path::Path;
use std::time::Duration;

use model::diff::{DiffChange, DiffSide};
use ui::editor::{Change, Command, Cursor};
use views::diff::{CaretSide, DiffCaret, DiffEditIntent, DiffPageId, DiffVersion, FileDiff, PageLine};
use views::file_view::FilePageId;

use super::{DiffPage, follow::follow_text, lines, page_mut, read_file_again};
use crate::persisted::same_folder;
use crate::store::feature::Said;
use crate::store::file_page::{self, HeldFile};
use crate::store::message::Effect;
use crate::store::state::State;

/// Said when **Revert this change** finds the text of the file other than the page shows it.
const CHANGED: &str = "The file changed since the diff was shown. Try again.";

/// Where the caret of a page is: on a line of a side that is not edited, or in the text of the working tree side of a
/// file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageCaret {
    /// A line chosen, by its row among the rows of its file.
    Line {
        /// The file.
        file: usize,
        /// The side.
        side: CaretSide,
        /// The row.
        row: usize,
    },
    /// A place, or a selection, in the text of a file.
    Text {
        /// The file.
        file: usize,
        /// The caret and the selection.
        cursor: Cursor,
    },
}

/// Something asked of the sides of a page.
pub fn reduce_edit(state: &mut State, id: DiffPageId, intent: DiffEditIntent) -> Vec<Effect> {
    match intent {
        DiffEditIntent::Text { file, commands, at } => edit(state, (id, file), &commands, at),
        DiffEditIntent::Line { file, side, row } => {
            if let Some(page) = page_mut(state, id) {
                page.caret = Some(PageCaret::Line { file, side, row });
                page.selection = None;
            }
            Vec::new()
        }
        DiffEditIntent::Recompute => recompute(state, id),
        DiffEditIntent::Save => save(state, id),
        DiffEditIntent::Revert(file, change) => revert(state, (id, file), Reverted::Change(change)),
        DiffEditIntent::Select { file, side, row } => {
            super::select::select(state, id, (file, side, row));
            Vec::new()
        }
        DiffEditIntent::ClearSelection => {
            if let Some(page) = page_mut(state, id) {
                page.selection = None;
            }
            Vec::new()
        }
        DiffEditIntent::RevertLines { file, rows } => {
            if let Some(page) = page_mut(state, id) {
                page.selection = None;
            }
            revert(state, (id, file), Reverted::Rows(rows))
        }
    }
}

/// The path of the working tree side of a file of a page, when the page edits it.
fn edited_path(page: &DiffPage, file: usize) -> Option<String> {
    let is_text = matches!(page.diffs.get(file), Some(FileDiff::Text(_)));
    let paths = page.target.files.get(file)?;
    (page.target.right == DiffVersion::WorkingTree && is_text).then(|| paths.right.clone())
}

/// The File page that holds the text of a file a page edits, held out of sight from the text the page shows when no
/// File page has it.
fn document_for(state: &mut State, (id, file): (DiffPageId, usize)) -> Option<FilePageId> {
    let page = state.diff_pages.pages.iter().find(|page| page.id == id)?;
    let path = edited_path(page, file)?;
    if let Some(found) = file_page::document_of(state, &page.top, &path) {
        return Some(found);
    }
    let Some(FileDiff::Text(text)) = page.diffs.get(file) else {
        return None;
    };
    let bytes = text.right.clone().into_bytes();
    let held = HeldFile {
        folder: page.folder.clone(),
        top: page.top.clone(),
        generation: page.generation,
    };
    file_page::hold(state, held, (&path, &bytes))
}

/// The keys and the pointer on the working tree side of a file: the text edited with the caret of the page, the rows
/// of the pages that show the file following it, and the line of the caret brought into sight.
fn edit(state: &mut State, (id, file): (DiffPageId, usize), commands: &[Command], at: Duration) -> Vec<Effect> {
    let Some(document) = document_for(state, (id, file)) else {
        return Vec::new();
    };
    let given = match state
        .diff_pages
        .pages
        .iter()
        .find(|page| page.id == id)
        .and_then(|page| page.caret.clone())
    {
        Some(PageCaret::Text {
            file: caret_file,
            cursor,
        }) if caret_file == file => cursor,
        _ => Cursor::default(),
    };
    let Some(edited) = file_page::edit_with(state, document, commands, Some(given)) else {
        return Vec::new();
    };
    if let Some(page) = page_mut(state, id) {
        page.caret = Some(PageCaret::Text {
            file,
            cursor: edited.cursor,
        });
        page.selection = None;
    }
    if !edited.changes.is_empty() {
        follow_document(
            state,
            (edited.top.as_path(), edited.path.as_str()),
            (&edited.changes, Some((id, at))),
        );
    }
    reveal_caret(state, id);
    Vec::new()
}

/// The text selected in the side of a page that is edited, when there is a selection there (`file-search.open`).
#[must_use]
pub fn selected_text(state: &State, id: DiffPageId) -> Option<String> {
    let page = state.diff_pages.pages.iter().find(|page| page.id == id)?;
    let Some(PageCaret::Text { file, cursor }) = &page.caret else {
        return None;
    };
    let path = edited_path(page, *file)?;
    let document = file_page::document_of(state, &page.top, &path)?;
    let held = state.file_pages.pages.iter().find(|held| held.id == document)?;
    Some(held.document()?.buffer().slice(cursor.range()).to_owned())
}

/// Brings the line of the caret of a page into sight, scrolled the least.
fn reveal_caret(state: &mut State, id: DiffPageId) {
    let serial = state.diff_pages.serial + 1;
    let Some(page) = state.diff_pages.pages.iter().find(|page| page.id == id) else {
        return;
    };
    let Some(PageCaret::Text { file, cursor }) = page.caret.clone() else {
        return;
    };
    let Some(line) = edited_path(page, file)
        .and_then(|path| file_page::document_of(state, &page.top, &path))
        .and_then(|document| state.file_pages.pages.iter().find(|held| held.id == document))
        .and_then(file_page::FilePage::document)
        .map(|document| document.buffer().position(cursor.head).line)
    else {
        return;
    };
    let Some(FileDiff::Text(text)) = page.diffs.get(file) else {
        return;
    };
    let row = text
        .aligned
        .rows
        .iter()
        .position(|aligned| aligned.right.as_ref().is_some_and(|right| right.line == line));
    let place = row.and_then(|row| page.lines.iter().position(|shown| *shown == PageLine::Row(file, row)));
    if let (Some(place), Some(page)) = (place, page_mut(state, id)) {
        page.caret_reveal = Some((serial, place));
        state.diff_pages.serial = serial;
    }
}

/// Makes the rows of every open page that shows a file of a working tree on its right side follow its text as an
/// edit left it: the carets in the text follow the changes, and the page the edit was made in, when one was, computes
/// the diff again a moment after the last edit.
pub fn follow_document(
    state: &mut State,
    (top, path): (&Path, &str),
    (changes, by): (&[Change], Option<(DiffPageId, Duration)>),
) {
    let Some(right) = file_page::document_of(state, top, path)
        .and_then(|document| state.file_pages.pages.iter().find(|page| page.id == document))
        .and_then(file_page::FilePage::document)
        .map(|document| String::from_utf8_lossy(&document.buffer().to_bytes()).into_owned())
    else {
        return;
    };
    let pages =
        state.diff_pages.pages.iter_mut().filter(|page| {
            page.is_open && page.target.right == DiffVersion::WorkingTree && same_folder(&page.top, top)
        });
    for page in pages {
        let is_by = by.is_some_and(|(id, _)| id == page.id);
        let files: Vec<usize> = page
            .target
            .files
            .iter()
            .enumerate()
            .filter(|(_, file)| file.right == path)
            .map(|(index, _)| index)
            .collect();
        for file in files {
            follow_file(page, file, &right);
            if let (
                Some(PageCaret::Text {
                    file: caret_file,
                    cursor,
                }),
                false,
            ) = (page.caret.as_mut(), is_by)
                && *caret_file == file
            {
                for change in changes {
                    cursor.follow(change);
                }
            }
        }
        if let Some((_, at)) = by.filter(|_| is_by) {
            page.edited_at = Some(at);
        }
    }
}

/// The rows of one file of a page following its text, and the folds clicked open following its rows.
fn follow_file(page: &mut DiffPage, file: usize, right: &str) {
    let Some(FileDiff::Text(text)) = page.diffs.get_mut(file) else {
        return;
    };
    let Some(map) = follow_text(text, right.to_owned()) else {
        return;
    };
    if let Some(unfolded) = page.unfolded.get_mut(file) {
        *unfolded = unfolded
            .iter()
            .filter_map(|row| map.get(*row).copied().flatten())
            .collect();
    }
    page.is_stale = true;
    lines::rebuild(page);
}

/// The diff computed again with the text as it is, for the files of a page and every other page that shows them.
fn recompute(state: &mut State, id: DiffPageId) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    page.edited_at = None;
    let top = page.top.clone();
    let paths: Vec<String> = (0..page.target.files.len())
        .filter_map(|file| edited_path(page, file))
        .collect();
    let mut effects = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        if !paths.get(..index).unwrap_or_default().contains(path) {
            effects.extend(read_file_again(state, &top, path));
        }
    }
    effects
}

/// `Ctrl+S`: `file-view.save` on the first file of the page with edits not saved.
fn save(state: &mut State, id: DiffPageId) -> Vec<Effect> {
    let Some(page) = state.diff_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let unsaved = (0..page.target.files.len())
        .filter_map(|file| edited_path(page, file))
        .filter_map(|path| file_page::document_of(state, &page.top, &path))
        .find(|document| {
            state
                .file_pages
                .pages
                .iter()
                .any(|held| held.id == *document && held.is_unsaved())
        });
    unsaved.map_or_else(Vec::new, |document| file_page::save(state, document, false))
}

/// **Revert this change**: the lines of the change in the text of the file replaced by those the left side has, as one
/// edit that `Ctrl+Z` takes back, and the file saved with `file-view.save`; refused when the text is not the one the
/// page shows.
fn revert(state: &mut State, (id, file): (DiffPageId, usize), change: Reverted) -> Vec<Effect> {
    let Some(document) = document_for(state, (id, file)) else {
        return Vec::new();
    };
    let Some(commands) = revert_commands(state, (id, file), (document, change)) else {
        state.operations.said = Some(Said::error(CHANGED.to_owned()));
        return Vec::new();
    };
    let Some(edited) = file_page::edit_with(state, document, &commands, Some(Cursor::default())) else {
        return Vec::new();
    };
    if let Some(page) = page_mut(state, id) {
        page.caret = Some(PageCaret::Text {
            file,
            cursor: edited.cursor,
        });
    }
    follow_document(
        state,
        (edited.top.as_path(), edited.path.as_str()),
        (&edited.changes, None),
    );
    let mut effects = file_page::save(state, document, false);
    effects.extend(read_file_again(state, &edited.top, &edited.path));
    effects
}

/// What is reverted: a change, by its place among the changes of its file, or the rows selected.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Reverted {
    /// **Revert this change**.
    Change(usize),
    /// **Revert lines**.
    Rows(Range<usize>),
}

/// The edit that reverts a change or the rows selected: the part of the text that differs once they are reverted,
/// selected and replaced; none when the text of the file is not the one the page shows.
fn revert_commands(
    state: &State,
    (id, file): (DiffPageId, usize),
    (document, change): (FilePageId, Reverted),
) -> Option<Vec<Command>> {
    let page = state.diff_pages.pages.iter().find(|page| page.id == id)?;
    let Some(FileDiff::Text(text)) = page.diffs.get(file) else {
        return None;
    };
    let buffer = state
        .file_pages
        .pages
        .iter()
        .find(|held| held.id == document)
        .and_then(file_page::FilePage::document)?
        .buffer();
    let bytes = buffer.to_bytes();
    if page.is_stale || String::from_utf8_lossy(&bytes) != text.right {
        return None;
    }
    let (left_lines, right_lines) = match &change {
        Reverted::Change(change) => {
            let lines = text.changes.get(*change)?;
            (lines.left.clone(), lines.right.clone())
        }
        Reverted::Rows(rows) => views::diff::lines_of_rows(text, rows),
    };
    let shown = DiffChange {
        path: String::new(),
        left: DiffSide::Empty,
        right: DiffSide::WorkingTree,
        left_lines,
        right_lines,
        left_text: Vec::new(),
        right_text: Vec::new(),
    };
    let reverted = match change {
        Reverted::Change(_) => features::diff::reverted(&bytes, text.left.as_bytes(), &shown),
        Reverted::Rows(_) => features::diff::reverted_lines(&bytes, text.left.as_bytes(), &shown)?,
    };
    let after = ui::editor::Buffer::from_bytes(&reverted).ok()?;
    let (before, after) = (buffer.text(), after.text());
    let prefix = common_prefix(before, after);
    let suffix = common_suffix(before.get(prefix..)?, after.get(prefix..)?);
    let replacement = after.get(prefix..after.len() - suffix)?.to_owned();
    Some(vec![
        Command::PlaceAt(buffer.position(prefix), false),
        Command::PlaceAt(buffer.position(before.len() - suffix), true),
        Command::Paste(replacement),
    ])
}

/// How many bytes two texts start with that are the same, at the end of a character.
fn common_prefix(one: &str, other: &str) -> usize {
    one.char_indices()
        .zip(other.chars())
        .find(|((_, first), second)| first != second)
        .map_or_else(|| one.len().min(other.len()), |((index, _), _)| index)
}

/// How many bytes two texts end with that are the same, at the start of a character.
fn common_suffix(one: &str, other: &str) -> usize {
    one.chars()
        .rev()
        .zip(other.chars().rev())
        .take_while(|(first, second)| first == second)
        .map(|(character, _)| character.len_utf8())
        .sum()
}

/// The caret of a page as it is drawn: a line chosen, or the place in the text of the file with the text itself.
#[must_use]
pub fn caret_state<'a>(state: &'a State, page: &'a DiffPage) -> Option<DiffCaret<'a>> {
    match page.caret.clone()? {
        PageCaret::Line { file, side, row } => Some(DiffCaret::Line { file, side, row }),
        PageCaret::Text { file, cursor } => {
            let path = edited_path(page, file)?;
            let document = file_page::document_of(state, &page.top, &path)?;
            let buffer = state
                .file_pages
                .pages
                .iter()
                .find(|held| held.id == document)
                .and_then(file_page::FilePage::document)?
                .buffer();
            let length = buffer.text().len();
            let cursor = Cursor {
                anchor: cursor.anchor.min(length),
                head: cursor.head.min(length),
                goal: cursor.goal,
            };
            Some(DiffCaret::Text { file, buffer, cursor })
        }
    }
}

/// Where a page brings its rows into sight: the change gone to last, in the middle, or the line of the caret, scrolled
/// the least, whichever was asked last.
#[must_use]
pub fn reveal_state(page: &DiffPage) -> Option<ui::Reveal> {
    let centered = page.reveal.map(|(serial, row)| ui::Reveal {
        serial,
        row,
        place: ui::RevealPlace::Center,
    });
    let nearest = page.caret_reveal.map(|(serial, row)| ui::Reveal {
        serial,
        row,
        place: ui::RevealPlace::Nearest,
    });
    match (centered, nearest) {
        (Some(centered), Some(nearest)) => Some(if nearest.serial > centered.serial {
            nearest
        } else {
            centered
        }),
        (centered, nearest) => centered.or(nearest),
    }
}
