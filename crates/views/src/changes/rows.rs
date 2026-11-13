//! The lists of the Changes page: **Conflicts**, each file with no box and the buttons that resolve it; then **Changes**,
//! with the box on its header, the buttons that act on every file, and each file with its box and its buttons; **No
//! changes** when both are empty.

use model::changes::{ChangeRow, FileStatus, OpenAction, OpenInput, OpenOnClick, OpenTarget, StagedBox};
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use model::state::ChangeKind;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    Button, ButtonKind, Choice, Icon, Label, List, Point, Rect, SelectionMode, Sense, TextKind, TextRun, Tone, Ui,
};

use super::{ChangesIntent, ChangesRepository};
use crate::conflicts::{OpenFileState, open_file_view};
use crate::details::{CappedList, FileLayout, FileRow, FilesIntent, FilesOwner, FilesState, directory_row, file_rows};
use crate::feature::FeatureIntent;
use crate::file_view::{FileTarget, FileViewIntent};
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

/// The lists over a rectangle, as rows that scroll.
pub(super) fn view(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let files = FilesState {
        files: state.files,
        tree: state.tree,
        layout: state.layout,
        folded: state.folded,
        capped: CappedList {
            owner: FilesOwner::Changes,
            count: state.files.len(),
            is_all_shown: true,
        },
    };
    let rows = changes_rows(state, &file_rows(&files));
    let list = List {
        name: words::PAGE,
        row_count: rows.len(),
        row_height: ui.theme().sizes.row_height,
        selection_mode: SelectionMode::Read,
        lead: 0.0,
    };
    let mut intents = Vec::new();
    let _ = ui.list("changes rows", rect, &list, |ui, row| {
        let Some(shown) = rows.get(row.index) else {
            return;
        };
        intents.extend(draw_row(state, ui, (row.rect, *shown)));
    });
    intents
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

/// A file in conflict: its name, with no box, which opens it where it can be resolved (`conflicts.open-file`), and the
/// buttons that resolve it: **Mark as resolved** (`conflicts.mark-resolved`) and the two sides (`conflicts.take-side`).
fn conflict_row(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, (rect, index): (Rect, usize)) -> Vec<Intent> {
    let Some(file) = state.list.conflicts.get(index) else {
        return Vec::new();
    };
    let mut intents = Vec::new();
    let path = file.path.as_str();
    let mut area = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let take = |side: &str| {
        let mut answers = Answers::new();
        answers.insert(AnswerKey("side"), Answer::Choice(side.to_owned()));
        let feature = FeatureId("conflicts.take-side");
        Intent::Feature(FeatureIntent::RunWith(
            feature,
            Target::File(file.path.clone()),
            answers,
        ))
    };
    for (icon, said, intent) in [
        (
            Icon::Plus,
            words::MARK_AS_RESOLVED,
            run("conflicts.mark-resolved", Target::File(file.path.clone())),
        ),
        (Icon::ArrowDown, words::TAKE_INCOMING, take("incoming")),
        (Icon::ArrowUp, words::TAKE_CURRENT, take("current")),
    ] {
        if icon_button(ui, &mut area, ((said, path), icon, said), !state.is_running) {
            intents.push(intent);
        }
    }
    let start = status_start(ui, area.rest(), ("!", ui.theme().colors.removed, 0));
    let opened = OpenFileState { file };
    intents.extend(ui.clip(start, |ui| open_file_view(&opened, ui)));
    intents
}

/// The heading of the changed files: the box that stages or unstages every one, **Changes** and its count, and the
/// buttons for every file.
fn changes_heading(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let mut area = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let can_change = !state.is_running;
    if icon_button(
        ui,
        &mut area,
        ("discard all", Icon::Close, words::DISCARD_ALL),
        can_change,
    ) {
        intents.push(run("changes.discard-all", Target::Changes));
    }
    if state.list.has_unsaved() {
        let _ = icon_button(ui, &mut area, ("save all", Icon::Check, words::SAVE_ALL), false);
    }
    if icon_button(ui, &mut area, ("stash all", Icon::Stash, words::STASH_ALL), can_change) {
        intents.push(run("stash.push", Target::Head));
    }
    if state.list.has_staged()
        && icon_button(
            ui,
            &mut area,
            ("stash staged", Icon::Stash, words::STASH_STAGED),
            can_change,
        )
    {
        intents.push(run("stash.push-staged", Target::Head));
    }
    let _ = icon_button(ui, &mut area, ("open all", Icon::Eye, words::OPEN_ALL_CHANGES), false);
    let (layout, icon, said) = match state.layout {
        FileLayout::List => (FileLayout::Tree, Icon::Folder, words::VIEW_AS_TREE),
        FileLayout::Tree => (FileLayout::List, Icon::Minus, words::VIEW_AS_LIST),
    };
    if icon_button(ui, &mut area, ("layout", icon, said), true) {
        intents.push(Intent::Changes(ChangesIntent::Files(FilesIntent::Layout(layout))));
    }
    let header_cell = inset(area.rest(), Insets::symmetric(ui.theme().sizes.spacing.medium, 0.0));
    let said = words::with_count(words::CHANGES, state.list.rows.len());
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

/// A changed file: its box, its letter, its name with its old name and a dot when it has edits not saved, and its
/// buttons.
fn file_row(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, (rect, index, depth): (Rect, usize, usize)) -> Vec<Intent> {
    let Some(row) = state.list.rows.get(index) else {
        return Vec::new();
    };
    let mut intents = Vec::new();
    let can_change = !state.is_running;
    let path = row.path.as_str();
    let mut area = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    for (said, icon, intent) in file_buttons(row) {
        let is_enabled = can_change && intent.is_some();
        if icon_button(ui, &mut area, ((said, path), icon, said), is_enabled) {
            intents.extend(intent);
        }
    }
    let indent = crate::details::indent_of(ui, depth);
    let left = area.rest();
    let indented = Rect::new(left.x + indent, left.y, (left.width - indent).max(0.0), left.height);
    let side = ui.theme().sizes.control_height;
    let mut parts = Area::with_gap(indented, ui.theme().sizes.spacing.medium);
    let check = parts.left(side);
    let response = ui.choice(("file box", path), check, &file_box(row.staged, path, can_change));
    if row.staged == StagedBox::Disabled {
        ui.tooltip(&response, words::NOT_SAVED);
    }
    if response.clicked {
        let feature = if row.staged == StagedBox::Ticked {
            "changes.unstage"
        } else {
            "changes.stage"
        };
        intents.push(run(feature, Target::File(row.path.clone())));
    }
    let named = parts.rest();
    let response = ui.interact(ui.id(("file name", path)), named, Sense::CLICK | Sense::HOVER);
    if response.hovered {
        let theme = *ui.theme();
        ui.fill_rounded_rect(named, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let (letter, color) = letter_of(row.status, ui);
    let name = status_start(ui, named, (letter, color, 0));
    file_name(state, ui, (name, index));
    if response.clicked {
        intents.extend(open_page(row, OpenAction::Click, state.on_click));
    }
    intents
}

/// What opens the page `changes.open` decides on, when it is the Diff page or the File page: the Merge page comes
/// later.
fn open_page(row: &ChangeRow, action: OpenAction, on_click: OpenOnClick) -> Option<Intent> {
    let request = model::changes::open(&OpenInput {
        file: OpenTarget::Changed(row),
        action,
        on_click,
    });
    if let model::changes::PageRequest::File(path) = &request {
        return Some(open_file(path));
    }
    let target = crate::diff::changes_file(&request)?;
    Some(Intent::Diff(crate::diff::DiffIntent::Open(Box::new(target))))
}

/// What opens the File page of a file of the working tree (`file-view.page`).
fn open_file(path: &str) -> Intent {
    Intent::FileView(FileViewIntent::Open(FileTarget::working_tree(path)))
}

/// What saves a file with edits not saved in its File page (`file-view.save`).
fn save_file(path: &str) -> Intent {
    Intent::FileView(FileViewIntent::Save(path.to_owned()))
}

/// The buttons of a file, from the right, each with what it asks; those whose operations come later ask nothing and
/// are disabled. A file only an editor has changed has **Save** and **Compare with the saved file**; the others
/// **Discard changes**, **Stash this file…**, **Save** when not saved, **Open changes**, and **Open file** when the
/// file is still on disk.
fn file_buttons(row: &ChangeRow) -> Vec<(&'static str, Icon, Option<Intent>)> {
    if row.status == FileStatus::Unsaved {
        return vec![
            (words::SAVE, Icon::Check, Some(save_file(&row.path))),
            (words::COMPARE_WITH_SAVED, Icon::Eye, None),
        ];
    }
    let discard = run("changes.discard", Target::File(row.path.clone()));
    let mut buttons = vec![
        (words::DISCARD, Icon::Close, Some(discard)),
        (
            words::STASH_FILE,
            Icon::Stash,
            Some(run("stash.push-file", Target::Files(vec![row.path.clone()]))),
        ),
    ];
    if row.is_unsaved {
        buttons.push((words::SAVE, Icon::Check, Some(save_file(&row.path))));
    }
    let open_changes = open_page(row, OpenAction::OpenChanges, OpenOnClick::File);
    buttons.push((words::OPEN_CHANGES, Icon::Eye, open_changes));
    if row.status != FileStatus::Changed(ChangeKind::Deleted) {
        buttons.push((words::OPEN_FILE, Icon::File, Some(open_file(&row.path))));
    }
    buttons
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

/// The start of a row: a letter in its color and the icon of a file, indented to a depth; gives the rest of the row.
fn status_start(ui: &mut Ui<'_>, rect: Rect, (letter, color, depth): (&str, ui::Color, usize)) -> Rect {
    let theme = *ui.theme();
    let indent = crate::details::indent_of(ui, depth);
    let mut parts = Area::with_gap(
        Rect::new(rect.x + indent, rect.y, (rect.width - indent).max(0.0), rect.height),
        theme.sizes.spacing.medium,
    );
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
    ui.icon_in(parts.left(theme.sizes.icon), Icon::File, Tone::Muted);
    parts.rest()
}

/// A button that is only an icon, at the right of an area, which says its name in its tooltip; says whether it was
/// clicked. One that cannot be used is drawn disabled.
fn icon_button(
    ui: &mut Ui<'_>,
    area: &mut Area,
    (key, icon, said): (impl std::hash::Hash, Icon, &str),
    is_enabled: bool,
) -> bool {
    let side = ui.theme().sizes.control_height;
    let mut button = Button::icon(icon, said).kind(ButtonKind::Icon);
    if !is_enabled {
        button = button.disabled();
    }
    let cell = place(area.right(side), (side, side), (Align::Start, Align::Center));
    ui.button(key, cell, &button).clicked
}

/// The intent that runs an operation of the catalog on what it is invoked on.
fn run(feature: &'static str, target: Target) -> Intent {
    Intent::Feature(FeatureIntent::Run(FeatureId(feature), target))
}
