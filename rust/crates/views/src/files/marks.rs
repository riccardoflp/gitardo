//! How a row of the tree of the Files page shows that a file differs from the last commit (`file-view.files-changes`):
//! its name in the color of its state and the letter of the Changes page at the right end of its row, a file deleted
//! dimmed and struck through, and the dot of a folder holding such a file.

use model::changes::{FolderMark, MarkedFile, TreeMark};
use model::state::ChangeKind;
use ui::layout::{Align, Area, place};
use ui::{AccessRole, Color, Description, Point, Rect, Response, TextKind, TextRun, Tone, Ui};

use crate::text::changes as letters;
use crate::text::files as words;

/// How much of its opacity a deleted file keeps, out of 255.
pub(super) const DIMMED: u8 = 150;

/// Whether a mark is of a file deleted, which is dimmed.
pub(super) fn is_deleted(file: Option<&MarkedFile>) -> bool {
    file.is_some_and(|file| file.mark == TreeMark::Changed(ChangeKind::Deleted))
}

/// The name of a file that differs from the last commit: the letter of its state at the right end of the row, its name
/// in the color of the state, struck through when it is deleted; the screen reader hears the state after the name, and
/// the tooltip of the row names the old name of a file renamed or copied.
pub(super) fn marked_file(ui: &mut Ui<'_>, (rect, response): (Rect, &Response), (name, file): (&str, &MarkedFile)) {
    let (letter, color) = letter_of(file.mark, ui);
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let style = ui.text_style(TextKind::Hash);
    let cell = parts.right(ui.measure("M", style));
    let line = place(
        cell,
        (cell.width, ui.line_height(style)),
        (Align::Center, Align::Center),
    );
    let _ = ui.text(&TextRun {
        text: letter,
        style,
        color,
        origin: Point::new(line.x, line.y),
        max_width: None,
    });
    let named = parts.rest();
    let style = ui.text_style(TextKind::Body);
    let (width, height) = (ui.measure(name, style).min(named.width), ui.line_height(style));
    let placed = place(named, (width, height), (Align::Start, Align::Center));
    let _ = ui.text(&TextRun {
        text: name,
        style,
        color,
        origin: Point::new(placed.x, placed.y),
        max_width: Some(named.width),
    });
    if is_deleted(Some(file)) {
        let strike = Rect::new(placed.x, placed.y + height / 2.0, width, ui.theme().sizes.border);
        ui.fill_rect(strike, color);
    }
    let said = words::with_state(name, &state_said(file));
    let id = ui.id(("files name", name, rect.y.to_bits()));
    ui.describe(id, named, &Description::new(AccessRole::Label, &said));
    if let Some(original) = &file.original_path {
        let is_copy = file.mark == TreeMark::Changed(ChangeKind::Copied);
        ui.tooltip(response, &words::tooltip_from(is_copy, original));
    } else if ui.measure(name, style) > named.width {
        ui.tooltip(response, name);
    }
}

/// The name of a folder holding a file that differs from the last commit, with a dot at the right end of its row in the
/// color of the most important state under it; the screen reader hears that it has changes.
pub(super) fn marked_folder(ui: &mut Ui<'_>, rect: Rect, (name, mark): (&str, FolderMark)) {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.small);
    let cell = parts.right(ui.measure("M", ui.text_style(TextKind::Hash)));
    let radius = theme.sizes.spacing.small.max(2.0) * 0.75;
    let center = Point::new(cell.x + cell.width / 2.0, cell.y + cell.height / 2.0);
    ui.fill_circle(center, radius, folder_color(mark, ui));
    let named = parts.rest();
    let style = ui.text_style(TextKind::Body);
    let placed = place(
        named,
        (ui.measure(name, style).min(named.width), ui.line_height(style)),
        (Align::Start, Align::Center),
    );
    let _ = ui.text(&TextRun {
        text: name,
        style,
        color: Tone::Normal.color(&theme),
        origin: Point::new(placed.x, placed.y),
        max_width: Some(named.width),
    });
    let said = words::with_state(name, words::HAS_CHANGES);
    let id = ui.id(("files folder name", name, rect.y.to_bits()));
    ui.describe(id, named, &Description::new(AccessRole::Label, &said));
}

/// The letter of a state and its color, as the Changes page draws them.
fn letter_of(mark: TreeMark, ui: &Ui<'_>) -> (&'static str, Color) {
    let colors = ui.theme().colors;
    match mark {
        TreeMark::Changed(kind) => crate::details::status(kind, ui),
        TreeMark::Untracked => (letters::UNTRACKED, colors.added),
        TreeMark::Conflicted => (letters::CONFLICTED, colors.removed),
    }
}

/// The color of the dot of a folder: the color of the letter of its most important state.
fn folder_color(mark: FolderMark, ui: &Ui<'_>) -> Color {
    let colors = ui.theme().colors;
    match mark {
        FolderMark::Added => colors.added,
        FolderMark::Modified => colors.modified,
        FolderMark::Deleted | FolderMark::Conflicted => colors.removed,
    }
}

/// What the screen reader hears of the state of a file, after its name.
fn state_said(file: &MarkedFile) -> String {
    let said = match file.mark {
        TreeMark::Changed(ChangeKind::Added) => words::ADDED,
        TreeMark::Changed(ChangeKind::Modified) => words::MODIFIED,
        TreeMark::Changed(ChangeKind::Deleted) => words::DELETED,
        TreeMark::Changed(ChangeKind::TypeChanged) => words::TYPE_CHANGED,
        TreeMark::Changed(kind @ (ChangeKind::Renamed | ChangeKind::Copied)) => {
            let is_copy = kind == ChangeKind::Copied;
            return match &file.original_path {
                Some(original) => words::from_path(
                    if is_copy {
                        words::COPIED_FROM
                    } else {
                        words::RENAMED_FROM
                    },
                    original,
                ),
                None => (if is_copy { words::COPIED } else { words::RENAMED }).to_owned(),
            };
        }
        TreeMark::Untracked => words::UNTRACKED,
        TreeMark::Conflicted => words::CONFLICTED,
    };
    said.to_owned()
}
