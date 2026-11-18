//! The Changes page beside what it opens, as the store keeps it (`changes.beside-file`): a file of the list opens its
//! Diff page or its File page on the right of the list, in place of the one shown there; `Alt+Down` and `Alt+Up` show
//! the next and the previous file of the list; and once the repository is read again, the place follows its file: a
//! diff shows what **Open changes** gives it now, and a file gone from the list leaves the place.

use std::path::PathBuf;

use model::changes::{ChangeRow, OpenAction, OpenOnClick};
use model::details::{FilePath, file_tree};
use views::changes::{BesideFile, BesideFileState, BesideShown, beside_request};
use views::details::FileLayout;
use views::diff::{DiffPageId, DiffTarget, FileStep};
use views::file_view::{FilePageId, FilePageState, FileTarget, FileViewIntent};
use views::page::{Page, PageState};

use super::diff_page::{self, Owner};
use super::message::Effect;
use super::state::State;
use crate::persisted::same_folder;

/// The file shown beside the list of the Changes page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Beside {
    /// The page that shows it.
    pub place: Place,
    /// The path of the file shown, which the list marks; none when the Diff page shows every file.
    pub path: Option<String>,
    /// The folder of the repository it is of.
    pub folder: PathBuf,
    /// The read of the repository it was last followed after.
    pub generation: u64,
    /// The number of the last time another file was shown, which brings its row into sight once.
    pub serial: u64,
}

/// The page beside the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A Diff page, shown in the Changes page rather than as a page of its own.
    Diff(DiffPageId),
    /// A File page, which is listed among the pages open only when it was before, or once it has edits not saved and
    /// is replaced.
    File(FilePageId),
}

/// A file of the list, or every file, opened beside the list in place of what was shown there.
pub fn open(state: &mut State, file: BesideFile) -> Vec<Effect> {
    match file {
        BesideFile::Diff(target) => open_diff(state, *target),
        BesideFile::File(path) => open_file(state, &path),
    }
}

/// Shows a diff beside the list: in the Diff page there, in place of what it showed, or in a new one; a File page shown
/// there before is forgotten unless it has edits not saved.
fn open_diff(state: &mut State, target: DiffTarget) -> Vec<Effect> {
    let Some((folder, generation)) = state.shown_read().map(|read| (read.folder.clone(), read.generation)) else {
        return Vec::new();
    };
    let path = match target.files.as_slice() {
        [file] => Some(file.right.clone()),
        _ => None,
    };
    let kept = match state.changes.beside.as_ref().map(|beside| beside.place) {
        Some(Place::Diff(id)) if state.diff_pages.pages.iter().any(|page| page.id == id) => Some(id),
        Some(Place::Diff(_) | Place::File(_)) | None => None,
    };
    let (id, effects) = if let Some(id) = kept {
        (id, retarget(state, id, target, generation))
    } else {
        leave(state);
        let effects = diff_page::open_page(state, target, Owner::Changes);
        let opened = state
            .diff_pages
            .pages
            .iter()
            .rev()
            .find(|page| page.owner() == Owner::Changes);
        let Some(id) = opened.map(|page| page.id) else {
            return effects;
        };
        (id, effects)
    };
    keep(state, (Place::Diff(id), path), (folder, generation));
    super::pages::show_page(state, Page::Changes);
    effects
}

/// Shows another target in the Diff page beside the list, unless it shows it already; marks the page read after the
/// read of the repository its files are read in.
fn retarget(state: &mut State, id: DiffPageId, target: DiffTarget, generation: u64) -> Vec<Effect> {
    if state
        .diff_pages
        .pages
        .iter()
        .any(|page| page.id == id && page.target == target)
    {
        return Vec::new();
    }
    let effects = diff_page::beside::retarget(state, id, target);
    if let Some(page) = state.diff_pages.pages.iter_mut().find(|page| page.id == id) {
        page.generation = generation;
    }
    effects
}

/// Shows a file of the working tree beside the list in its File page; the one shown there before is forgotten unless
/// it has edits not saved, which keep it open in its own page, and a file that has its own page already keeps it.
fn open_file(state: &mut State, path: &str) -> Vec<Effect> {
    let Some((folder, generation)) = state.shown_read().map(|read| (read.folder.clone(), read.generation)) else {
        return Vec::new();
    };
    let listed = state.pages.open.clone();
    let effects = super::file_page::reduce(state, FileViewIntent::Open(FileTarget::working_tree(path)));
    let Some(Page::File(id)) = state.page_shown() else {
        return effects;
    };
    if !listed.contains(&Page::File(id)) {
        state.pages.open.retain(|page| *page != Page::File(id));
    }
    if state.changes.beside.as_ref().map(|beside| beside.place) != Some(Place::File(id)) {
        leave(state);
    }
    keep(state, (Place::File(id), Some(path.to_owned())), (folder, generation));
    super::pages::show_page(state, Page::Changes);
    effects
}

/// Keeps what is shown beside the list, with a new number that brings its row into sight.
fn keep(state: &mut State, (place, path): (Place, Option<String>), (folder, generation): (PathBuf, u64)) {
    let serial = state.changes.beside.as_ref().map_or(0, |beside| beside.serial) + 1;
    state.changes.beside = Some(Beside {
        place,
        path,
        folder,
        generation,
        serial,
    });
}

/// Takes the file away from beside the list: its Diff page is forgotten, its File page too unless it has edits not
/// saved, which keep it open in its own page.
pub fn leave(state: &mut State) {
    if let Some(Place::File(id)) = state.changes.beside.take().map(|beside| beside.place) {
        super::files_page::keep_if_unsaved(state, id);
    }
}

/// Whether a Diff page is the one beside the list of the Changes page.
#[must_use]
pub fn shows_diff(state: &State, id: DiffPageId) -> bool {
    state
        .changes
        .beside
        .as_ref()
        .is_some_and(|beside| beside.place == Place::Diff(id))
}

/// The File page beside the list, while there is one.
#[must_use]
pub fn shown_file(state: &State) -> Option<FilePageId> {
    match state.changes.beside.as_ref()?.place {
        Place::File(id) => Some(id),
        Place::Diff(_) => None,
    }
}

/// `Alt+Down` or `Alt+Up` beside the list: the next or the previous file in the order the list shows them, opened as
/// the file shown is, its diff or the file itself; the first file when every file is shown.
pub fn step(state: &mut State, step: FileStep) -> Vec<Effect> {
    let Some(beside) = state.changes.beside.as_ref() else {
        return Vec::new();
    };
    let action = match beside.place {
        Place::Diff(_) => OpenAction::OpenChanges,
        Place::File(_) => OpenAction::OpenFile,
    };
    let rows = rows_in_order(state);
    let shown = beside
        .path
        .as_deref()
        .and_then(|path| rows.iter().position(|row| row.path == path));
    let next = match (shown, step) {
        (Some(place), FileStep::Next) => rows.get(place + 1),
        (Some(place), FileStep::Previous) => place.checked_sub(1).and_then(|place| rows.get(place)),
        (None, _) => rows.first(),
    };
    match next.and_then(|row| beside_request(row, action, OpenOnClick::File)) {
        Some(file) => open(state, file),
        None => Vec::new(),
    }
}

/// The rows of the list in the order the page shows them: by path in a list, folder by folder in a tree.
fn rows_in_order(state: &State) -> Vec<ChangeRow> {
    let Some(list) = super::changes::listed(state) else {
        return Vec::new();
    };
    match state.persisted.file_layouts.changes {
        FileLayout::List => list.rows,
        FileLayout::Tree => {
            let paths: Vec<FilePath> = list.rows.iter().map(|row| FilePath::new(row.path.clone())).collect();
            let mut order = Vec::new();
            diff_page::beside::tree_files(&file_tree(&paths), &mut order);
            order
                .into_iter()
                .filter_map(|index| list.rows.get(index).cloned())
                .collect()
        }
    }
}

/// Follows the place once the repository is read again: it is left when its page is gone, when another repository is
/// shown, or when its file is no longer in the list; a diff of one file shows what **Open changes** gives it now, so
/// that a file staged wholly shows its staged changes; a diff of every file shows the files listed now, and is left
/// once the list is empty.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let Some(beside) = state.changes.beside.clone() else {
        return Vec::new();
    };
    let is_there = match beside.place {
        Place::Diff(id) => state.diff_pages.pages.iter().any(|page| page.id == id),
        Place::File(id) => state.file_pages.pages.iter().any(|page| page.id == id),
    };
    let read = state.shown_read().map(|read| (read.folder.clone(), read.generation));
    let Some((folder, generation)) = read.filter(|_| is_there) else {
        leave(state);
        return Vec::new();
    };
    if !same_folder(&folder, &beside.folder) {
        leave(state);
        return Vec::new();
    }
    if generation == beside.generation {
        return Vec::new();
    }
    if let Some(kept) = state.changes.beside.as_mut() {
        kept.generation = generation;
    }
    let rows = super::changes::listed(state).map_or_default(|list| list.rows);
    let row = match &beside.path {
        Some(path) => rows.iter().find(|row| row.path == *path),
        None => rows.first(),
    };
    let Some(row) = row else {
        leave(state);
        return Vec::new();
    };
    match (beside.place, &beside.path) {
        (Place::Diff(id), Some(_)) => match beside_request(row, OpenAction::OpenChanges, OpenOnClick::File) {
            Some(BesideFile::Diff(target)) => retarget(state, id, *target, generation),
            Some(BesideFile::File(_)) | None => Vec::new(),
        },
        (Place::Diff(id), None) => match views::diff::changes_files(&rows) {
            Some(target) => retarget(state, id, target, generation),
            None => Vec::new(),
        },
        (Place::File(_), _) => Vec::new(),
    }
}

/// The Changes page as it is drawn: with the page of its file beside the list while one is shown and open, the column
/// as wide as it was dragged, or the file alone while the side column shows the list in its **Changes** section
/// (`changes.section`); the File page with the blame of its lines.
#[must_use]
pub fn page_state<'a>(
    state: &'a State,
    data: &'a super::ChangesData,
    (blame, column_width): (&'a super::blame::BlameData, Option<f32>),
) -> PageState<'a> {
    let changes = data.page_state(state);
    let shown = match state.changes.beside.as_ref().map(|beside| beside.place) {
        Some(Place::Diff(id)) => state
            .diff_pages
            .pages
            .iter()
            .any(|page| page.id == id && page.is_open)
            .then(|| diff_page::page_state(state, id))
            .flatten()
            .map(|diff| BesideShown::Diff(Box::new(diff))),
        Some(Place::File(id)) => super::file_page::page_state(state, id).map(|file| {
            BesideShown::File(Box::new(FilePageState {
                blame: blame.page_blame(state, id),
                ..file
            }))
        }),
        None => None,
    };
    match shown {
        Some(shown) => PageState::ChangesBeside(Box::new(BesideFileState {
            changes,
            shown,
            column_width: column_width.map(super::files_page::whole_pixels),
            is_listed_aside: is_listed_aside(state),
        })),
        None => PageState::Changes(changes),
    }
}

/// Whether the side column shows **Changes** open, so that the list of the files is there: it is not put away, or it
/// is open over the page in the narrow layout, and the section is not folded.
fn is_listed_aside(state: &State) -> bool {
    let panes = super::timeline::workspace_panes(state);
    (!panes.is_rail_hidden || state.is_side_column_over) && !panes.folds.is_changes_folded
}
