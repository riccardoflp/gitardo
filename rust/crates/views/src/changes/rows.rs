//! The lists of the Changes page: **Conflicts**, each file with no box; then **Changes**, with the box on its header and
//! the buttons that act on every file, those the width does not hold in **…**, and each file with its box; **No
//! changes** when both are empty. A row has no button: what is done to its file is in its menu (`changes.row-menu`).

use model::changes::{FileStatus, OpenAction, StagedBox};
use model::contract::Target;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Button, ButtonKind, Choice, Description, Icon, Label, List, Point, Rect, Reveal, RevealPlace,
    SelectionMode, Sense, Steering, TextKind, TextRun, Tone, Ui,
};

use super::ChangesRepository;
use super::actions::{buttons_kept, conflict_actions, file_actions, heading_actions, open_page, run, staying};
use super::menu::{RowMenuState, more_menu, row_menu};
use crate::conflicts::{OpenFileState, open_file_row};
use crate::details::{CappedList, FileRow, FilesOwner, FilesState, directory_row, file_rows};
use crate::intent::Intent;
use crate::text::changes as words;

/// A row of the lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChangesRow<'a> {
    /// **Conflicts** and their count.
    ConflictsHeading,
    /// A file in conflict, by its place.
    Conflict(usize),
    /// **Changes**, their count, the box on the header and the buttons for every file.
    ChangesHeading,
    /// A directory or a file of the list.
    File(FileRow<'a>),
    /// **No changes**.
    NoChanges,
}

/// The files of the list as the lists of files of the window lay them out, every one shown.
fn files_state<'a>(state: &ChangesRepository<'a>) -> FilesState<'a> {
    FilesState {
        files: state.files,
        tree: state.tree,
        layout: state.layout,
        folded: state.folded,
        capped: CappedList {
            owner: FilesOwner::Changes,
            count: state.files.len(),
            is_all_shown: true,
        },
    }
}

/// How many rows the lists have, their headings included.
pub(super) fn row_count(state: &ChangesRepository<'_>) -> usize {
    changes_rows(state, &file_rows(&files_state(state))).len()
}

/// The lists over a rectangle, as rows that scroll.
pub(super) fn view(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let files = files_state(state);
    let rows = changes_rows(state, &file_rows(&files));
    let list = List {
        name: words::PAGE,
        row_count: rows.len(),
        row_height: ui.theme().sizes.row_height,
        selection_mode: SelectionMode::Read,
        lead: 0.0,
    };
    let reveal = state
        .marked
        .and_then(|path| rows.iter().position(|row| row_path(state, row) == Some(path)))
        .map(|row| Reveal {
            serial: state.serial,
            row,
            place: RevealPlace::Nearest,
        });
    let steering = Steering {
        reveal,
        ..Steering::default()
    };
    let mut intents = Vec::new();
    let _ = ui.steered_list("changes rows", (rect, &list), &steering, |ui, row| {
        let Some(shown) = rows.get(row.index) else {
            return;
        };
        intents.extend(draw_row(state, ui, (row.rect, *shown)));
    });
    intents
}

/// The path of the file of a row of **Changes**; none for the other rows.
fn row_path<'a>(state: &ChangesRepository<'a>, row: &ChangesRow<'_>) -> Option<&'a str> {
    match row {
        ChangesRow::File(FileRow::File(index, _)) => state.list.rows.get(*index).map(|row| row.path.as_str()),
        ChangesRow::ConflictsHeading
        | ChangesRow::Conflict(_)
        | ChangesRow::ChangesHeading
        | ChangesRow::File(FileRow::Directory { .. })
        | ChangesRow::NoChanges => None,
    }
}

/// The rows of the lists, from the rows of the files laid out.
fn changes_rows<'a>(state: &ChangesRepository<'_>, files: &[FileRow<'a>]) -> Vec<ChangesRow<'a>> {
    let mut rows = Vec::new();
    if !state.list.conflicts.is_empty() {
        rows.push(ChangesRow::ConflictsHeading);
        rows.extend((0..state.list.conflicts.len()).map(ChangesRow::Conflict));
    }
    if state.list.rows.is_empty() {
        if state.list.conflicts.is_empty() {
            rows.push(ChangesRow::NoChanges);
        }
        return rows;
    }
    rows.push(ChangesRow::ChangesHeading);
    rows.extend(files.iter().copied().map(ChangesRow::File));
    rows
}

/// A row of the lists.
fn draw_row(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, (rect, row): (Rect, ChangesRow<'_>)) -> Vec<Intent> {
    match row {
        ChangesRow::ConflictsHeading => {
            let said = words::with_count(words::CONFLICTS, state.list.conflicts.len());
            let _ = ui.label(rect, &Label::new(&said).kind(TextKind::Strong));
            Vec::new()
        }
        ChangesRow::Conflict(index) => conflict_row(state, ui, (rect, index)),
        ChangesRow::ChangesHeading => changes_heading(state, ui, rect),
        ChangesRow::File(file @ FileRow::Directory { .. }) => directory_row(ui, rect, (FilesOwner::Changes, &file)),
        ChangesRow::File(FileRow::File(index, depth)) => file_row(state, ui, (rect, index, depth)),
        ChangesRow::NoChanges => {
            let _ = ui.label(rect, &Label::new(words::NO_CHANGES).tone(Tone::Muted));
            Vec::new()
        }
    }
}

/// A file in conflict: its letter, its icon and its name, with no box and no button; a click on it opens it where it
/// can be resolved (`conflicts.open-file`), and its menu has what resolves it (`changes.row-menu`).
fn conflict_row(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, (rect, index): (Rect, usize)) -> Vec<Intent> {
    let Some(file) = state.list.conflicts.get(index) else {
        return Vec::new();
    };
    let start = status_start(ui, rect, (words::CONFLICTED, ui.theme().colors.removed, &file.path));
    let opened = OpenFileState { file };
    let (mut intents, row) = ui.clip(start, |ui| open_file_row(opened, ui));
    let menu = RowMenuState {
        path: &file.path,
        actions: &conflict_actions(state, file),
        is_running: state.is_running,
        row,
    };
    intents.extend(row_menu(&menu, ui));
    intents
}

/// The heading of the changed files: the box that stages or unstages every one, **Changes** and its count, and at its
/// right the buttons for every file; when the width does not hold them all, those that leave last stay and **…** has
/// the others (Q762).
fn changes_heading(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let spacing = ui.theme().sizes.spacing;
    let mut area = Area::with_gap(rect, spacing.small);
    let can_change = !state.is_running;
    let said = words::with_count(words::CHANGES, state.list.rows.len());
    let actions = heading_actions(state);
    let kept = buttons_kept(actions.len(), button_places(ui, rect.width, &said));
    let on_heading = staying(&actions, kept);
    if on_heading.len() < actions.len() {
        let cell = icon_cell(ui, &mut area);
        let more = (actions.as_slice(), on_heading.as_slice(), state.is_running);
        intents.extend(more_menu(ui, cell, more));
    }
    for action in on_heading.iter().filter_map(|index| actions.get(*index)) {
        let is_enabled = action.is_offered(state.is_running);
        if icon_button(ui, &mut area, (action.words, action.icon, action.words), is_enabled) {
            intents.extend(action.intent.clone());
        }
    }
    let header_cell = inset(area.rest(), Insets::symmetric(spacing.medium, 0.0));
    let header = match state.list.header {
        StagedBox::Ticked => Choice::check_box(&said, true),
        StagedBox::HalfTicked => Choice::half_ticked(&said),
        StagedBox::Empty => Choice::check_box(&said, false),
        StagedBox::Disabled => Choice::check_box(&said, false).disabled(),
    };
    let header = if can_change { header } else { header.disabled() };
    if ui.choice("all files", header_cell, &header).clicked {
        let feature = if state.list.header == StagedBox::Ticked {
            "changes.unstage-all"
        } else {
            "changes.stage-all"
        };
        intents.push(run(feature, Target::Changes));
    }
    intents
}

/// How many buttons a heading of a width has places for, beside its box, its name and its count.
fn button_places(ui: &Ui<'_>, width: f32, said: &str) -> usize {
    let sizes = ui.theme().sizes;
    let named = sizes.control_height + sizes.spacing.medium * 3.0 + ui.measure(said, ui.text_style(TextKind::Body));
    let place = sizes.control_height + sizes.spacing.small;
    crate::number::count_of_float(((width - named + sizes.spacing.small) / place).floor())
}

/// A changed file: its box, its letter, its icon and its name with its old name and a dot when it has edits not
/// saved, and no button. A click on the row opens what `changes.openOnClick` says, the row takes the keyboard, and its
/// menu has what is done to the file (`changes.row-menu`).
fn file_row(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, (rect, index, depth): (Rect, usize, usize)) -> Vec<Intent> {
    let Some(row) = state.list.rows.get(index) else {
        return Vec::new();
    };
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let can_change = !state.is_running;
    let path = row.path.as_str();
    let is_marked = state.marked == Some(path);
    let id = ui.id(("file name", path));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::HOVER | Sense::FOCUS);
    if is_marked {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_selection);
    } else if response.contains_pointer {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    ui.describe(
        id,
        rect,
        &Description::new(AccessRole::Button, path).selected(is_marked),
    );
    let indent = crate::details::indent_of(ui, depth);
    let indented = Rect::new(rect.x + indent, rect.y, (rect.width - indent).max(0.0), rect.height);
    let mut parts = Area::with_gap(indented, theme.sizes.spacing.medium);
    let check = parts.left(theme.sizes.control_height);
    let ticked = ui.choice(("file box", path), check, &file_box(row.staged, path, can_change));
    if row.staged == StagedBox::Disabled {
        ui.tooltip(&ticked, words::NOT_SAVED);
    }
    if ticked.clicked {
        let feature = if row.staged == StagedBox::Ticked {
            "changes.unstage"
        } else {
            "changes.stage"
        };
        intents.push(run(feature, Target::File(row.path.clone())));
    }
    let (letter, color) = letter_of(row.status, ui);
    let name = status_start(ui, parts.rest(), (letter, color, &row.path));
    file_name(state, ui, (name, index));
    if response.clicked {
        intents.extend(open_page(row, OpenAction::Click, state.on_click));
    }
    // The right button on the box opens the menu of the row too.
    let menu = RowMenuState {
        path,
        actions: &file_actions(row),
        is_running: state.is_running,
        row: ui::Response {
            secondary_clicked: response.secondary_clicked || ticked.secondary_clicked,
            ..response
        },
    };
    intents.extend(row_menu(&menu, ui));
    intents
}

/// The box of a file: ticked, half ticked or empty as much of it is staged; greyed out for a file only an editor has
/// changed, and while an operation runs. Its name for the screen reader is the path.
fn file_box(staged: StagedBox, path: &str, can_change: bool) -> Choice<'_> {
    let choice = match staged {
        StagedBox::Ticked => Choice::check_box(path, true),
        StagedBox::HalfTicked => Choice::half_ticked(path),
        StagedBox::Empty => Choice::check_box(path, false),
        StagedBox::Disabled => return Choice::check_box(path, false).label_hidden().disabled(),
    };
    let choice = choice.label_hidden();
    if can_change { choice } else { choice.disabled() }
}

/// The name of a file of the list, its old name after a staged rename, and a dot when it has edits not saved.
fn file_name(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, (rect, index): (Rect, usize)) {
    let (Some(file), Some(row)) = (state.files.get(index), state.list.rows.get(index)) else {
        return;
    };
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let renamed = row.original_path.as_deref().map(words::renamed_from);
    let tail = match (row.is_unsaved, renamed) {
        (true, Some(renamed)) => format!("{} {renamed}", words::UNSAVED),
        (true, None) => words::UNSAVED.to_owned(),
        (false, renamed) => renamed.unwrap_or_default(),
    };
    if !tail.is_empty() {
        let width = ui.measure(&tail, ui.text_style(TextKind::Detail)).min(rect.width * 0.5);
        let _ = ui.label(
            parts.right(width),
            &Label::new(&tail).kind(TextKind::Detail).tone(Tone::Muted),
        );
    }
    crate::details::file_name(ui, parts.rest(), (file, state.layout));
}

/// The letter of how a file is changed, and its color.
fn letter_of(status: FileStatus, ui: &Ui<'_>) -> (&'static str, ui::Color) {
    match status {
        FileStatus::Changed(kind) => crate::details::status(kind, ui),
        FileStatus::Untracked => (words::UNTRACKED, ui.theme().colors.added),
        FileStatus::Unsaved => (words::UNSAVED, ui.theme().colors.muted_foreground),
    }
}

/// The start of a row: a letter in its color and the icon of its file (`file-view.file-icons`); gives the rest of the
/// row.
fn status_start(ui: &mut Ui<'_>, rect: Rect, (letter, color, path): (&str, ui::Color, &str)) -> Rect {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium);
    let style = ui.text_style(TextKind::Hash);
    let cell = parts.left(ui.measure("M", style));
    let line = place(cell, (cell.width, ui.line_height(style)), (Align::Start, Align::Center));
    let run = TextRun {
        text: letter,
        style,
        color,
        origin: Point::new(line.x, line.y),
        max_width: None,
    };
    let _ = ui.text(&run);
    let name = path.rsplit('/').next().unwrap_or(path);
    ui.file_icon_in(parts.left(theme.sizes.icon), ui::FileOrFolder::File(name), Tone::Muted);
    parts.rest()
}

/// The place of a button that is only an icon, cut from the right of an area.
fn icon_cell(ui: &Ui<'_>, area: &mut Area) -> Rect {
    let side = ui.theme().sizes.control_height;
    place(area.right(side), (side, side), (Align::Start, Align::Center))
}

/// A button that is only an icon, at the right of an area, which says its name in its tooltip; says whether it was
/// clicked. One that cannot be used is drawn disabled.
fn icon_button(
    ui: &mut Ui<'_>,
    area: &mut Area,
    (key, icon, said): (impl std::hash::Hash, Icon, &str),
    is_enabled: bool,
) -> bool {
    let mut button = Button::icon(icon, said).kind(ButtonKind::Icon);
    if !is_enabled {
        button = button.disabled();
    }
    let cell = icon_cell(ui, area);
    ui.button(key, cell, &button).clicked
}
