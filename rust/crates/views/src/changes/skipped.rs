//! The files hidden from git, under the changes (`changes.skipped-list`): a heading **Hidden from git** with their
//! count, which folds, and a row for each file, which opens its diff between the last commit and the disk.

use model::contract::Target;
use model::state::SkippedFile;
use ui::layout::{Align, Area, place};
use ui::{AccessRole, Button, ButtonKind, Description, Label, MenuEntry, Rect, Sense, TextKind, Tone, Ui};

use super::actions::{RowAction, run};
use super::menu::{RowMenuState, row_menu};
use super::{BesideFile, ChangesIntent};
use crate::diff::{DiffFile, DiffTarget, DiffVersion};
use crate::icons;
use crate::intent::Intent;
use crate::text::changes as words;

/// What the group of the files hidden from git is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkippedListState<'a> {
    /// The files hidden from git, as `changes.read-skipped` read them.
    pub files: &'a [SkippedFile],
    /// Whether the group is folded.
    pub is_folded: bool,
    /// Whether an operation runs: the items that change something cannot be chosen.
    pub is_running: bool,
}

/// A row of the group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkippedRow {
    /// **Hidden from git** and their count.
    Heading,
    /// A file, by its place.
    File(usize),
}

/// The rows of the group: none with no file hidden, the heading alone while it is folded.
#[must_use]
pub fn skipped_rows(state: &SkippedListState<'_>) -> Vec<SkippedRow> {
    if state.files.is_empty() {
        return Vec::new();
    }
    let mut rows = vec![SkippedRow::Heading];
    if !state.is_folded {
        rows.extend((0..state.files.len()).map(SkippedRow::File));
    }
    rows
}

/// A row of the group of the files hidden from git, over its rectangle: the heading, which a click folds, with **…**
/// for **Show all to git again**; or a file, with its folder and how it is hidden, which a click opens between the last
/// commit and the disk, and whose menu has **Open changes**, **Open file** and **Show its changes to git again**.
///
/// Feature: changes.skipped-list
pub fn skipped_list(state: &SkippedListState<'_>, ui: &mut Ui<'_>, (rect, row): (Rect, SkippedRow)) -> Vec<Intent> {
    match row {
        SkippedRow::Heading => heading(state, ui, rect),
        SkippedRow::File(index) => state
            .files
            .get(index)
            .map_or_else(Vec::new, |file| file_row(state, ui, (rect, file))),
    }
}

/// **Hidden from git** and the count, a chevron that says whether it is folded, and **…** at its right.
fn heading(state: &SkippedListState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let sizes = ui.theme().sizes;
    let mut area = Area::with_gap(rect, sizes.spacing.small);
    let side = sizes.control_height;
    let more = place(area.right(side), (side, side), (Align::Start, Align::Center));
    let button = Button::icon(icons::MORE, words::MORE_FOR_HIDDEN).kind(ButtonKind::Icon);
    let item = icons::action(icons::SHOW, words::SHOW_ALL_TO_GIT, ());
    let item = if state.is_running { item.disabled() } else { item };
    if ui
        .menu_button("hidden heading menu", more, &button, &[MenuEntry::Item(item)])
        .is_some()
    {
        intents.push(run("changes.unskip", Target::Changes));
    }
    let said = words::with_count(words::HIDDEN_FROM_GIT, state.files.len());
    let icon = if state.is_folded {
        icons::FOLDED
    } else {
        icons::UNFOLDED
    };
    let folding = Button::icon(icon, &said).kind(ButtonKind::Icon);
    let chevron = place(area.left(side), (side, side), (Align::Start, Align::Center));
    let clicked = ui.button("hidden heading fold", chevron, &folding).clicked;
    let name_cell = area.rest();
    let named = ui.interact(ui.id("hidden heading name"), name_cell, Sense::CLICK);
    let _ = ui.label(name_cell, &Label::new(&said).kind(TextKind::Strong));
    if clicked || named.clicked {
        intents.push(Intent::Changes(ChangesIntent::FoldHidden));
    }
    intents
}

/// A file hidden from git: its icon, its name, its folder, and how it is hidden in the small text of details.
fn file_row(state: &SkippedListState<'_>, ui: &mut Ui<'_>, (rect, file): (Rect, &SkippedFile)) -> Vec<Intent> {
    let theme = *ui.theme();
    let path = file.path.as_str();
    let id = ui.id(("hidden file", path));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::HOVER | Sense::FOCUS);
    if response.contains_pointer {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    ui.describe(id, rect, &Description::new(AccessRole::Button, path));
    ui.tooltip(&response, words::HIDDEN_TOOLTIP);
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium);
    let how = if file.how.is_skip_worktree() {
        words::SKIP_WORKTREE
    } else {
        words::ASSUMED_UNCHANGED
    };
    let width = ui.measure(how, ui.text_style(TextKind::Detail));
    let _ = ui.label(
        parts.right(width),
        &Label::new(how).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
    ui.file_icon_in(parts.left(theme.sizes.icon), ui::FileOrFolder::File(name), Tone::Muted);
    let name_width = ui.measure(name, ui.text_style(TextKind::Body)).min(parts.rest().width);
    let _ = ui.label(parts.left(name_width), &Label::new(name));
    if !folder.is_empty() {
        let _ = ui.label(
            parts.rest(),
            &Label::new(folder).kind(TextKind::Detail).tone(Tone::Muted),
        );
    }
    let diff = Intent::Changes(ChangesIntent::OpenBeside(BesideFile::Diff(Box::new(DiffTarget::file(
        DiffVersion::Head,
        DiffVersion::WorkingTree,
        DiffFile::same(path),
    )))));
    let mut intents = Vec::new();
    if response.clicked {
        intents.push(diff.clone());
    }
    let open_file = Intent::Changes(ChangesIntent::OpenBeside(BesideFile::File(path.to_owned())));
    let actions = [
        RowAction::shows(words::OPEN_CHANGES, icons::CHANGES, Some(diff)),
        RowAction::shows(words::OPEN_FILE, icons::OPEN_FILE, Some(open_file)),
        RowAction::changes(
            words::SHOW_TO_GIT,
            icons::SHOW,
            run("changes.unskip", Target::File(path.to_owned())),
        ),
    ];
    let menu = RowMenuState {
        path,
        actions: &actions,
        is_running: state.is_running,
        row: response,
    };
    intents.extend(row_menu(&menu, ui));
    intents
}
