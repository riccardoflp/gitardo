//! The Changes page beside what it opens (`changes.beside-file`): a file of the list opens its Diff page or its File
//! page on the right, and the page stays on the left as a column, the file shown marked, so that a review goes from
//! file to file without coming back to the list; `Alt+Down` and `Alt+Up` show the next and the previous file.

use model::changes::{ChangeRow, OpenAction, OpenInput, OpenOnClick, OpenTarget, PageRequest};
use ui::layout::Insets;
use ui::{AccessRole, Axis, Key, Modifiers, NamedKey, Rect, Split, Splitter, Ui};

use super::{ChangesIntent, ChangesState, page_body};
use crate::diff::{DiffPageState, DiffTarget, FileStep};
use crate::file_view::FilePageState;
use crate::graph::{GraphIntent, INSPECTOR_MOST, Pane, STEP};
use crate::intent::Intent;
use crate::line::INSPECTOR_OVER_BELOW;
use crate::repo::no_repository_view;
use crate::text::changes as words;

/// Where the column of the Changes page starts, and where a double click on its line puts it back.
const COLUMN_START: f32 = 360.0;
/// The least width of the column.
const COLUMN_LEAST: f32 = 240.0;
/// The least width of the page beside the column.
const SHOWN_LEAST: f32 = 240.0;

/// What a file of the Changes page opens beside the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BesideFile {
    /// A Diff page: of one file, or of every file of the list.
    Diff(Box<DiffTarget>),
    /// The File page of a file of the working tree, by its path from the top.
    File(String),
}

/// The page shown beside the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BesideShown<'a> {
    /// The Diff page of a file, or of every file.
    Diff(Box<DiffPageState<'a>>),
    /// The File page of a file.
    File(Box<FilePageState<'a>>),
}

/// The Changes page with a file opened beside its list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BesideFileState<'a> {
    /// The Changes page, as the column, with the file shown marked.
    pub changes: ChangesState<'a>,
    /// The page of the file shown.
    pub shown: BesideShown<'a>,
    /// The width of the column as dragged, in whole logical pixels; none where it starts.
    pub column_width: Option<u16>,
}

impl<'a> BesideFileState<'a> {
    /// The title of the page shown, for the trail.
    #[must_use]
    pub fn title(&self) -> &'a str {
        match &self.shown {
            BesideShown::Diff(diff) => diff.title,
            BesideShown::File(file) => file.title,
        }
    }

    /// Whether the page shown has edits not saved.
    #[must_use]
    pub fn is_unsaved(&self) -> bool {
        match &self.shown {
            BesideShown::Diff(diff) => diff.is_unsaved,
            BesideShown::File(file) => file.is_unsaved,
        }
    }
}

/// What a file of the list opens beside it, from what `changes.open` decides: the Diff page or the File page; none for
/// the Merge page, which a file in conflict opens in place of the page, or for a side the Diff page cannot show yet.
#[must_use]
pub fn beside_request(row: &ChangeRow, action: OpenAction, on_click: OpenOnClick) -> Option<BesideFile> {
    let request = model::changes::open(&OpenInput {
        file: OpenTarget::Changed(row),
        action,
        on_click,
    });
    match &request {
        PageRequest::File(path) => Some(BesideFile::File(path.clone())),
        PageRequest::Diff { .. } => {
            crate::diff::changes_file(&request).map(|target| BesideFile::Diff(Box::new(target)))
        }
        PageRequest::Merge(_) => None,
    }
}

/// The Changes page with a file opened, over the rectangle its caller clipped it to: the column of the page on the
/// left, the line between that resizes it, and the Diff page or the File page on the right; in the narrow layout the
/// page of the file alone. `Alt+Down` and `Alt+Up` show the next and the previous file of the list.
///
/// Feature: changes.beside-file
pub fn beside_file_view(state: &BesideFileState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let mut intents = if ui.window().width < INSPECTOR_OVER_BELOW {
        shown_view(&state.shown, ui)
    } else {
        split(state, ui, rect)
    };
    for key in ui.unclaimed_keys() {
        let step = match (key.key, key.modifiers) {
            (Key::Named(NamedKey::ArrowDown), Modifiers::ALT) => FileStep::Next,
            (Key::Named(NamedKey::ArrowUp), Modifiers::ALT) => FileStep::Previous,
            _ => continue,
        };
        intents.push(Intent::Changes(ChangesIntent::StepFile(step)));
    }
    intents
}

/// The column and the page side by side, with the line between them that is dragged.
fn split(state: &BesideFileState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let line = ui.theme().sizes.border;
    let room = (rect.width - line).max(0.0);
    let most = rect.width * INSPECTOR_MOST;
    let width = state
        .column_width
        .map_or(COLUMN_START, f32::from)
        .clamp(COLUMN_LEAST, most.max(COLUMN_LEAST))
        .min(room);
    let column = Rect::new(rect.x, rect.y, width, rect.height);
    let shown = Rect::new(column.right() + line, rect.y, (room - width).max(0.0), rect.height);
    let mut intents = Vec::new();
    ui.region(
        "changes column",
        (column, AccessRole::Complementary, words::COLUMN),
        |ui| {
            intents.extend(ui.clip(column, |ui| column_view(&state.changes, ui)));
        },
    );
    ui.separator(Rect::new(column.right(), rect.y, line, rect.height), Axis::Vertical);
    intents.extend(ui.clip(shown, |ui| shown_view(&state.shown, ui)));
    let splitter = Splitter {
        split: Split::Columns,
        default_size: COLUMN_START,
        least_first: COLUMN_LEAST,
        least_second: SHOWN_LEAST.min(room),
        size: Some(width),
        step: STEP,
    };
    let response = ui.splitter_across("changes column line", rect, &splitter, line);
    if response.is_reset {
        intents.push(Intent::Graph(GraphIntent::SetPane(Pane::ChangesColumn, None)));
    } else if response.is_changed {
        intents.push(Intent::Graph(GraphIntent::SetPane(
            Pane::ChangesColumn,
            Some(response.size),
        )));
    }
    intents
}

/// The Changes page as a column: the same parts as the whole page, with narrower margins.
fn column_view(state: &ChangesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    match state {
        ChangesState::NoRepository(state) => no_repository_view(state, ui),
        ChangesState::Repository(repository) => {
            let spacing = ui.theme().sizes.spacing;
            page_body(repository, ui, Insets::symmetric(spacing.medium, spacing.medium))
        }
    }
}

/// The page of the file shown.
fn shown_view(shown: &BesideShown<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    match shown {
        BesideShown::Diff(diff) => crate::diff::page_view(diff, ui),
        BesideShown::File(file) => crate::file_view::page_view(file, ui),
    }
}
