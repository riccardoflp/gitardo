//! The results of the Search page: the names and the lines found, each a row, the letters and the text that matched
//! marked; the row under the keyboard, which the arrows move; and the line that counts what was found.

use std::ops::Range;

use model::file_search::{Mode, NameFound, ResultRow, TextMatch, counts};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Description, FileOrFolder, Key, KeyEvent, Label, List, Modifiers, NamedKey, Rect, Reveal, RevealPlace,
    Row, SelectionMode, Sense, Steering, TextKind, Tone, Ui,
};

use super::{FileSearchIntent, SearchPageState, Status};
use crate::text::file_search as words;

/// How many results `Page Up` and `Page Down` pass.
const PAGE: usize = 10;

/// The widest number of a line, in digits: the room kept for the numbers of the lines.
const NUMBER_DIGITS: &str = "00000";

/// The row a key brings under the keyboard: the result after or before, ten further with `Page Down` and `Page Up`;
/// the headings are passed over. None for another key, or when the keyboard stays where it is.
pub(super) fn moved_by(state: &SearchPageState<'_>, key: KeyEvent) -> Option<usize> {
    let (is_forward, by) = match (key.key, key.modifiers) {
        (Key::Named(NamedKey::ArrowDown), Modifiers::NONE) => (true, 1),
        (Key::Named(NamedKey::ArrowUp), Modifiers::NONE) => (false, 1),
        (Key::Named(NamedKey::PageDown), Modifiers::NONE) => (true, PAGE),
        (Key::Named(NamedKey::PageUp), Modifiers::NONE) => (false, PAGE),
        _ => return None,
    };
    let results: Vec<usize> = state
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.is_result())
        .map(|(index, _)| index)
        .collect();
    let last = results.len().checked_sub(1)?;
    let place = state.lit.and_then(|lit| results.iter().position(|row| *row == lit));
    let next = match (place, is_forward) {
        (None, true) => 0,
        (None, false) => return None,
        (Some(place), true) => place.saturating_add(by).min(last),
        (Some(place), false) => place.saturating_sub(by),
    };
    results.get(next).copied().filter(|row| Some(*row) != state.lit)
}

/// The line under the results: how many lines were found and in how many files, how many names in **Files** and
/// **Folders**, and **Searching…** while the text is searched.
pub(super) fn count_line(state: &SearchPageState<'_>, ui: &mut Ui<'_>, rect: Rect) {
    let found = state.found;
    let (lines, files) = counts(found);
    let said = match (state.status, state.search.mode) {
        (Status::Reading | Status::Failed(_), _) => String::new(),
        (_, Mode::Files) => words::files_found(found.files.len(), found.files_total),
        (_, Mode::Folders) => words::folders_found(found.folders.len(), found.folders_total),
        (Status::Searching, _) if lines == 0 => words::SEARCHING.to_owned(),
        (_, Mode::Text | Mode::All) if state.search.query.is_empty() => String::new(),
        (_, Mode::Text | Mode::All) => words::lines_found(lines, files, found.more_text),
    };
    let _ = ui.label(rect, &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted));
}

/// The results over a rectangle: the list of the rows, or what is said while the files are listed, when they cannot
/// be, before anything is typed in **Text**, and when nothing was found.
pub(super) fn results_view(state: &SearchPageState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<FileSearchIntent> {
    let theme = *ui.theme();
    let said = match state.status {
        Status::Failed(error) => Some(words::failed(error)),
        _ if !state.rows.is_empty() => None,
        Status::Reading => Some(words::READING.to_owned()),
        _ if state.search.query.is_empty() && state.search.mode == Mode::Text => Some(words::TYPE_TO_SEARCH.to_owned()),
        Status::Searching => Some(words::SEARCHING.to_owned()),
        Status::Done => Some(words::NO_RESULTS.to_owned()),
    };
    if let Some(said) = said {
        let line = Rect::new(rect.x, rect.y, rect.width, theme.sizes.row_height.min(rect.height));
        let _ = ui.label(line, &Label::new(&said).tone(Tone::Muted));
        return Vec::new();
    }
    let list = List {
        name: words::RESULTS,
        row_count: state.rows.len(),
        row_height: theme.sizes.row_height,
        selection_mode: SelectionMode::Read,
        lead: 0.0,
    };
    let steering = Steering {
        reveal: state.lit.map(|row| Reveal {
            serial: state.reveal,
            row,
            place: RevealPlace::Nearest,
        }),
        ..Steering::default()
    };
    let mut asked = Vec::new();
    let _ = ui.steered_list("search results", (rect, &list), &steering, |ui, row| {
        if let Some(result) = state.rows.get(row.index) {
            asked.extend(row_view(state, ui, row, *result));
        }
    });
    asked
}

/// A row of the results: a heading, or a result lit while it is under the keyboard or the pointer. A click brings it
/// under the keyboard, and a double click opens it.
fn row_view(state: &SearchPageState<'_>, ui: &mut Ui<'_>, row: &Row, result: ResultRow) -> Option<FileSearchIntent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    if let ResultRow::Heading(section) = result {
        let rect = inset(row.rect, Insets::symmetric(spacing.medium, 0.0));
        let heading = Label::new(words::section(section))
            .kind(TextKind::Detail)
            .tone(Tone::Muted);
        let _ = ui.label(rect, &heading);
        return None;
    }
    let id = ui.id(("search row", row.index));
    let response = ui.interact(id, row.rect, Sense::CLICK | Sense::HOVER);
    let is_lit = state.lit == Some(row.index);
    if is_lit {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_selection);
    } else if response.contains_pointer {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let inner = inset(row.rect, Insets::symmetric(spacing.medium, 0.0));
    let found = state.found;
    let said = match result {
        ResultRow::Folder(index) => name_row(ui, inner, (found.folders.get(index)?, false, None)),
        ResultRow::File(index) => name_row(ui, inner, (found.files.get(index)?, true, None)),
        ResultRow::TextFile { first, lines } => {
            let name = NameFound {
                path: found.text.get(first)?.path.clone(),
                marks: Vec::new(),
            };
            name_row(ui, inner, (&name, true, Some(lines)))
        }
        ResultRow::Line(index) => line_row(ui, inner, found.text.get(index)?),
        ResultRow::Heading(_) => return None,
    };
    let description = Description::new(AccessRole::ListItem, &said).selected(is_lit);
    ui.describe(id, row.rect, &description);
    if response.double_clicked {
        Some(FileSearchIntent::Open(row.index))
    } else if response.clicked && !is_lit {
        Some(FileSearchIntent::Light(row.index))
    } else {
        None
    }
}

/// A file or a folder: its icon, its name, then the folder it is in, less bright, the letters that matched marked;
/// and, for a file with lines found, how many at the right. Gives what the row says to the screen reader.
fn name_row(ui: &mut Ui<'_>, rect: Rect, (found, is_file, lines): (&NameFound, bool, Option<usize>)) -> String {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium);
    let (folder, name) = found.path.rsplit_once('/').unwrap_or(("", &found.path));
    let name_start = found.path.len() - name.len();
    let icon = if is_file {
        FileOrFolder::File(name)
    } else {
        FileOrFolder::Folder(name)
    };
    let side = theme.sizes.icon;
    ui.file_icon_in(
        place(parts.left(side), (side, side), (Align::Start, Align::Center)),
        icon,
        Tone::Muted,
    );
    if let Some(lines) = lines {
        let count = words::lines_of_file(lines);
        let width = ui.measure(&count, ui.text_style(TextKind::Detail));
        let _ = ui.label(
            parts.right(width),
            &Label::new(&count).kind(TextKind::Detail).tone(Tone::Muted),
        );
    }
    let in_name = letters(&found.path, &found.marks, name_start..found.path.len());
    let name_width = ui.measure(name, ui.text_style(TextKind::Body)).min(parts.rest().width);
    marked_label(
        ui,
        parts.left(name_width),
        (name, TextKind::Body, Tone::Normal),
        &in_name,
    );
    let in_folder = letters(&found.path, &found.marks, 0..folder.len());
    marked_label(ui, parts.rest(), (folder, TextKind::Detail, Tone::Muted), &in_folder);
    found.path.clone()
}

/// A line of text found: its number, then its text in the font of code, every match marked. Gives what the row says
/// to the screen reader.
fn line_row(ui: &mut Ui<'_>, rect: Rect, line: &TextMatch) -> String {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium);
    let _ = parts.left(theme.sizes.icon);
    let number = line.line.to_string();
    let number_width = ui
        .measure(NUMBER_DIGITS, ui.text_style(TextKind::Code))
        .max(ui.measure(&number, ui.text_style(TextKind::Code)));
    let _ = ui.label(
        parts.left(number_width),
        &Label::new(&number)
            .kind(TextKind::Code)
            .tone(Tone::Muted)
            .align(Align::End),
    );
    marked_label(
        ui,
        parts.rest(),
        (&line.text, TextKind::Code, Tone::Normal),
        &line.marks,
    );
    words::line_said(&line.path, line.line, &line.text)
}

/// The letters marked of a part of a path, as ranges of that part, those side by side as one.
fn letters(path: &str, marks: &[usize], part: Range<usize>) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    for at in marks.iter().copied().filter(|at| part.contains(at)) {
        let length = path
            .get(at..)
            .and_then(|rest| rest.chars().next())
            .map_or(1, char::len_utf8);
        let (start, end) = (at - part.start, at - part.start + length);
        match ranges.last_mut() {
            Some(last) if last.end == start => last.end = end,
            _ => ranges.push(start..end),
        }
    }
    ranges
}

/// A line of text with some of it marked as what a search found: the color of a match behind those parts.
fn marked_label(ui: &mut Ui<'_>, rect: Rect, (text, kind, tone): (&str, TextKind, Tone), marks: &[Range<usize>]) {
    let style = ui.text_style(kind);
    let height = ui.line_height(style);
    let color = ui.theme().colors.find_match;
    for mark in marks {
        let (Some(before), Some(marked)) = (text.get(..mark.start), text.get(mark.clone())) else {
            continue;
        };
        let x = rect.x + ui.measure(before, style);
        let width = ui.measure(marked, style).min(rect.right() - x);
        if width <= 0.0 {
            break;
        }
        ui.fill_rect(
            Rect::new(x, rect.y + (rect.height - height) * 0.5, width, height),
            color,
        );
    }
    let _ = ui.label(rect, &Label::new(text).kind(kind).tone(tone));
}
