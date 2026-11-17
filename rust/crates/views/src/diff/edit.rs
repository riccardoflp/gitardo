//! The sides of a Diff page as places to type and to point at: the working tree side is edited where it is shown
//! (`diff.edit`), with a caret, a selection, the keys of the editor, the clipboard and `Ctrl+S`; a line of either side is
//! chosen by a click; and `F4` or **Jump to source** opens the file in the File page at the line of the caret
//! (`diff.open-at-line`).
//!
//! The rows of the page are drawn by its list; each side has one widget of its own over them, declared after the list,
//! which takes the pointer and the keyboard and draws the caret and the selection over the rows in sight.

use std::time::Duration;

use model::diff::AlignedRow;
use ui::editor::{Command, Position};
use ui::{AccessRole, Description, Key, MenuEntry, Modifiers, NamedKey, Point, Rect, Sense, TextKind, Ui};

use super::caret::draw_caret;
use super::columns::Columns;
use super::{
    CaretSide, DiffCaret, DiffEditIntent, DiffIntent, DiffPageState, DiffText, DiffVersion, FileDiff, PageLine,
};
use crate::file_view::{FileTarget, FileViewIntent};
use crate::icons;
use crate::intent::Intent;
use crate::text::diff as words;
use crate::text::file_view as file_words;

/// How long after the last key typed the diff is computed again.
pub const RECOMPUTE_AFTER: Duration = Duration::from_millis(200);

/// The byte order mark, which a side shows at the start of its first line and the text of the file leaves out.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// The sides of a page as its list drew them: where the columns are, and the rows in sight, each by its place among
/// the rows of the page and where it is.
#[derive(Clone, Debug)]
pub struct DrawnSides<'a> {
    /// The page.
    pub(super) state: &'a DiffPageState<'a>,
    /// The columns.
    pub(super) columns: Columns,
    /// The rows in sight.
    pub(super) rows: Vec<(usize, Rect)>,
    /// The rectangle of the rows.
    list: Rect,
}

impl<'a> DrawnSides<'a> {
    /// The sides of a page whose list drew these rows.
    pub(crate) fn new(
        state: &'a DiffPageState<'a>,
        (columns, list): (Columns, Rect),
        rows: Vec<(usize, Rect)>,
    ) -> Self {
        DrawnSides {
            state,
            columns,
            rows,
            list,
        }
    }
}

/// An entry of the context menu of a side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MenuChoice {
    /// Cut.
    Cut,
    /// Copy.
    Copy,
    /// Paste, which comes with `Ctrl+V` only (Q214).
    Paste,
    /// **Jump to source**.
    JumpToSource,
}

/// The two sides of a page over the rows its list drew: what the pointer and the keys ask of them, the caret and the
/// selection drawn, the diff computed again a moment after typing stops, and `Ctrl+S`.
///
/// Feature: diff.edit
pub fn edit_view(sides: &DrawnSides<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let state = sides.state;
    let mut intents = Vec::new();
    for side in [CaretSide::Left, CaretSide::Right] {
        intents.extend(side_view(sides, ui, side));
    }
    if let Some(edited) = state.edited_at {
        let due = edited + RECOMPUTE_AFTER;
        if ui.now() >= due {
            intents.push(edit_intent(state, DiffEditIntent::Recompute));
        } else {
            ui.request_frame_at(due);
        }
    }
    let saves = ui
        .unclaimed_keys()
        .into_iter()
        .any(|key| matches!(key.key, Key::Character('s' | 'S')) && key.modifiers.command() && !key.modifiers.shift());
    if saves && state.is_unsaved && state.versions.1 == &DiffVersion::WorkingTree {
        intents.push(edit_intent(state, DiffEditIntent::Save));
    }
    intents.extend(open_at_line_view(state, ui));
    intents
}

/// Whether the right side of a file is edited: the working tree, read as text.
fn is_editable(state: &DiffPageState<'_>, file: usize) -> bool {
    *state.versions.1 == DiffVersion::WorkingTree && matches!(state.diffs.get(file), Some(FileDiff::Text(_)))
}

/// The widget of one side over the rows: the pointer, the keys, the menu, and the caret drawn.
fn side_view(sides: &DrawnSides<'_>, ui: &mut Ui<'_>, side: CaretSide) -> Vec<Intent> {
    let state = sides.state;
    let column = match side {
        CaretSide::Left => sides.columns.left,
        CaretSide::Right => sides.columns.right,
    };
    let rect = Rect::new(column.x, sides.list.y, column.width, sides.list.height);
    let id = ui.id(("diff side", state.id, side == CaretSide::Right));
    let (response, events) = ui.interact_with_events(id, rect, Sense::CLICK | Sense::DRAG | Sense::FOCUS);
    let name = match side {
        CaretSide::Left => state.names.0,
        CaretSide::Right => state.names.1,
    };
    ui.describe(id, rect, &Description::new(AccessRole::TextInput, name));
    let mut intents = Vec::new();
    for event in events {
        intents.extend(side_event(sides, ui, (side, id), event));
    }
    let is_any_editable = (0..state.files.len()).any(|file| is_editable(state, file));
    if response.hovered && side == CaretSide::Right && is_any_editable {
        ui.set_cursor(ui::Cursor::Text);
    }
    intents.extend(menu(sides, ui, (side, &response)));
    ui.clip(sides.list, |ui| draw_caret(sides, ui, (side, response.has_focus)));
    intents
}

/// What one event asks of a side: a press places the caret or chooses a line, a drag selects, the keys and the text
/// typed or pasted edit the working tree side.
fn side_event(
    sides: &DrawnSides<'_>,
    ui: &mut Ui<'_>,
    (side, id): (CaretSide, ui::WidgetId),
    event: ui::WidgetEvent,
) -> Vec<Intent> {
    let state = sides.state;
    match event {
        ui::WidgetEvent::Pressed {
            button,
            position,
            modifiers,
            click_count,
        } => pressed(sides, ui, side, (button, position, modifiers, click_count)),
        ui::WidgetEvent::Dragged { position, .. } if side == CaretSide::Right => {
            let Some((file, at)) = text_position(sides, ui, position) else {
                return Vec::new();
            };
            text_commands(state, file, vec![Command::PlaceAt(at, true)], ui.now())
        }
        ui::WidgetEvent::Key(key) if side == CaretSide::Right => typed_key(sides, ui, id, key),
        ui::WidgetEvent::Text(text) if side == CaretSide::Right => {
            let typed: String = text.chars().filter(|character| !character.is_control()).collect();
            caret_commands(
                state,
                ui,
                (!typed.is_empty())
                    .then_some(Command::Type(typed))
                    .into_iter()
                    .collect(),
            )
        }
        ui::WidgetEvent::Paste(text) if side == CaretSide::Right => {
            caret_commands(state, ui, vec![Command::Paste(text)])
        }
        // The other events do not change a side.
        _ => Vec::new(),
    }
}

/// A press on a side: on the working tree side, the caret placed, a word selected by a double click and a line by a
/// triple; on a side that is not edited, its line chosen. A press of the button of menus places the caret where the
/// selection is not.
fn pressed(
    sides: &DrawnSides<'_>,
    ui: &mut Ui<'_>,
    side: CaretSide,
    (button, position, modifiers, click_count): (ui::PointerButton, Point, Modifiers, u32),
) -> Vec<Intent> {
    let state = sides.state;
    let Some((page_line, _)) = row_at(sides, position.y) else {
        return Vec::new();
    };
    let Some(PageLine::Row(file, row)) = state.lines.get(page_line) else {
        return Vec::new();
    };
    if side == CaretSide::Left || !is_editable(state, *file) {
        let line = DiffEditIntent::Line {
            file: *file,
            side,
            row: *row,
        };
        return vec![edit_intent(state, line)];
    }
    let Some((file, at)) = text_position(sides, ui, position) else {
        return Vec::new();
    };
    let command = match (button, click_count) {
        (ui::PointerButton::Primary, 1) => Command::PlaceAt(at, modifiers.shift()),
        (ui::PointerButton::Primary, 2) => Command::SelectWord(at),
        (ui::PointerButton::Primary, _) => Command::SelectLine(at),
        (ui::PointerButton::Secondary, _) if !is_in_selection(state, file, at) => Command::PlaceAt(at, false),
        _ => return Vec::new(),
    };
    text_commands(state, file, vec![command], ui.now())
}

/// Whether a place of a file is inside the selection of the caret.
fn is_in_selection(state: &DiffPageState<'_>, file: usize, at: Position) -> bool {
    match state.caret {
        Some(DiffCaret::Text {
            file: caret_file,
            buffer,
            cursor,
        }) if caret_file == file => cursor.range().contains(&buffer.offset(at)),
        _ => false,
    }
}

/// What a key asks of the working tree side while it has the keyboard: the keys of the editor of code, the selection
/// copied first when they copy.
fn typed_key(sides: &DrawnSides<'_>, ui: &mut Ui<'_>, id: ui::WidgetId, key: ui::KeyEvent) -> Vec<Intent> {
    let state = sides.state;
    let Some(DiffCaret::Text { buffer, cursor, .. }) = state.caret else {
        return Vec::new();
    };
    let row_height = super::rows::row_height(ui);
    let page = crate::number::count_of_float(sides.list.height / row_height).max(1);
    let asked = ui::key_commands(key, page, false);
    if asked.is_used {
        ui.claim_key(id, key);
    }
    if asked.copies {
        let copied = buffer.slice_as_file(cursor.range());
        if !copied.is_empty() {
            ui.copy_to_clipboard(copied);
        }
    }
    caret_commands(state, ui, asked.commands)
}

/// Commands for the file of the caret, when it is in a text.
fn caret_commands(state: &DiffPageState<'_>, ui: &Ui<'_>, commands: Vec<Command>) -> Vec<Intent> {
    match state.caret {
        Some(DiffCaret::Text { file, .. }) => text_commands(state, file, commands, ui.now()),
        _ => Vec::new(),
    }
}

/// What the keys and the pointer ask of the text of a file, at the moment of the frame.
fn text_commands(state: &DiffPageState<'_>, file: usize, commands: Vec<Command>, at: Duration) -> Vec<Intent> {
    if commands.is_empty() {
        return Vec::new();
    }
    vec![edit_intent(state, DiffEditIntent::Text { file, commands, at })]
}

/// Something asked of the editing of a page.
fn edit_intent(state: &DiffPageState<'_>, intent: DiffEditIntent) -> Intent {
    Intent::Diff(DiffIntent::Edit(state.id, intent))
}

/// The row in sight at a height of the window, the nearest when it is above or below them all.
fn row_at(sides: &DrawnSides<'_>, y: f32) -> Option<(usize, Rect)> {
    let found = sides.rows.iter().find(|(_, rect)| y >= rect.y && y < rect.bottom());
    let first = sides.rows.first().filter(|(_, rect)| y < rect.y);
    found.or(first).or(sides.rows.last()).copied()
}

/// The place of the text of the working tree side at a point: the line of its row, or the next line of the right side
/// for a filler row, and the character nearest to it.
fn text_position(sides: &DrawnSides<'_>, ui: &Ui<'_>, point: Point) -> Option<(usize, Position)> {
    let state = sides.state;
    let (page_line, rect) = row_at(sides, point.y)?;
    let Some(PageLine::Row(file, row)) = state.lines.get(page_line) else {
        return None;
    };
    let Some(FileDiff::Text(text)) = state.diffs.get(*file) else {
        return None;
    };
    let line = right_line_at_or_after(&text.aligned.rows, *row);
    let Some(line) = line else {
        let last = text.right_lines.len().saturating_sub(1);
        let column = text.right_line(last).len();
        return Some((*file, buffer_position(text, last, column)));
    };
    let cell = super::rows::text_cell(ui, side_rect(sides.columns.right, rect));
    let shown = text.right_line(line);
    let advance = code_advance(ui);
    let wanted = ((point.x - cell.x) / advance).max(0.0);
    let mut columns = 0.0_f32;
    for (index, character) in shown.char_indices() {
        let width = if character == '\t' { 4.0 } else { 1.0 };
        if wanted < columns + width * 0.5 {
            return Some((*file, buffer_position(text, line, index)));
        }
        columns += width;
    }
    Some((*file, buffer_position(text, line, shown.len())))
}

/// The line of the right side of a row, or of the first row after it that has one.
fn right_line_at_or_after(rows: &[AlignedRow], row: usize) -> Option<usize> {
    rows.iter()
        .skip(row)
        .find_map(|aligned| aligned.right.as_ref().map(|line| line.line))
}

/// The line of the right side of a row, or of the last row before it that has one: where the text of a removed line
/// used to be.
fn right_line_at_or_before(rows: &[AlignedRow], row: usize) -> Option<usize> {
    rows.iter()
        .take(row + 1)
        .rev()
        .find_map(|aligned| aligned.right.as_ref().map(|line| line.line))
}

/// A place of the text of the file from a line of the right side and a byte of it as the side shows it, which has the
/// byte order mark at the start of the first line that the text leaves out.
fn buffer_position(text: &DiffText, line: usize, column: usize) -> Position {
    Position {
        line,
        column: column.saturating_sub(mark_length(text, line)),
    }
}

/// How long the byte order mark is at the start of a line of the right side: 3 bytes on the first line of a file that
/// has one, none otherwise.
pub(super) fn mark_length(text: &DiffText, line: usize) -> usize {
    if line == 0 && text.right.starts_with(BYTE_ORDER_MARK) {
        BYTE_ORDER_MARK.len_utf8()
    } else {
        0
    }
}

/// The width of a column of the font of code.
pub(super) fn code_advance(ui: &Ui<'_>) -> f32 {
    ui.measure("0000000000", ui.text_style(TextKind::Code)) / 10.0
}

/// One side of a row.
pub(super) fn side_rect(column: Rect, row: Rect) -> Rect {
    Rect::new(column.x, row.y, column.width, row.height)
}

/// The context menu of a side: cut, copy and paste on the working tree side, and **Jump to source**.
fn menu(sides: &DrawnSides<'_>, ui: &mut Ui<'_>, (side, response): (CaretSide, &ui::Response)) -> Vec<Intent> {
    let state = sides.state;
    let text = match state.caret {
        Some(DiffCaret::Text { buffer, cursor, .. }) if side == CaretSide::Right => Some((buffer, cursor)),
        _ => None,
    };
    let has_selection = text.is_some_and(|(_, cursor)| cursor.has_selection());
    let entry = |icon, label, (keys, choice), is_on: bool| {
        let item = icons::action(icon, label, choice).with_shortcut(keys);
        MenuEntry::Item(if is_on { item } else { item.disabled() })
    };
    let mut entries = Vec::new();
    if side == CaretSide::Right && *state.versions.1 == DiffVersion::WorkingTree {
        entries.extend([
            entry(
                icons::CUT,
                file_words::CUT,
                (file_words::CUT_KEYS, MenuChoice::Cut),
                has_selection,
            ),
            entry(
                icons::COPY,
                file_words::COPY,
                (file_words::COPY_KEYS, MenuChoice::Copy),
                has_selection,
            ),
            entry(
                icons::PASTE,
                file_words::PASTE,
                (file_words::PASTE_KEYS, MenuChoice::Paste),
                false,
            ),
            MenuEntry::Separator,
        ]);
    }
    entries.push(entry(
        icons::OPEN_FILE,
        words::JUMP_TO_SOURCE,
        (words::JUMP_TO_SOURCE_KEYS, MenuChoice::JumpToSource),
        true,
    ));
    let chosen = ui.context_menu(
        ("diff side menu", state.id, side == CaretSide::Right),
        response,
        &entries,
    );
    match (chosen, text) {
        (Some(MenuChoice::Cut | MenuChoice::Copy), Some((buffer, cursor))) => {
            ui.copy_to_clipboard(buffer.slice_as_file(cursor.range()));
            if chosen == Some(MenuChoice::Cut) {
                return caret_commands(state, ui, vec![Command::DeleteSelection]);
            }
            Vec::new()
        }
        (Some(MenuChoice::JumpToSource), _) => open_at_caret(state).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// `F4` on the page, while it has a caret: the File page of the file at the line of the caret.
///
/// Feature: diff.open-at-line
pub fn open_at_line_view(state: &DiffPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let pressed = ui
        .unclaimed_keys()
        .into_iter()
        .any(|key| key.key == Key::Named(NamedKey::Function(4)) && key.modifiers == Modifiers::NONE);
    if !pressed {
        return Vec::new();
    }
    open_at_caret(state).into_iter().collect()
}

/// The File page of the file of the caret at its line: from the working tree side, the line of the caret; from the
/// left side, the line of the right side on the same row, or for a removed line the nearest line of the right side
/// above it, or line 1.
fn open_at_caret(state: &DiffPageState<'_>) -> Option<Intent> {
    let caret = state.caret?;
    let paths = state.files.get(caret.file())?;
    let line = match caret {
        DiffCaret::Text { buffer, cursor, .. } => buffer.position(cursor.head).line,
        DiffCaret::Line { file, row, .. } => {
            let Some(FileDiff::Text(text)) = state.diffs.get(file) else {
                return None;
            };
            right_line_at_or_before(&text.aligned.rows, row).unwrap_or(0)
        }
    };
    let revision = match state.versions.1 {
        DiffVersion::Commit { commit, .. } => Some(commit.clone()),
        _ => None,
    };
    Some(Intent::FileView(FileViewIntent::Open(FileTarget {
        path: paths.right.clone(),
        revision,
        line: Some(line + 1),
    })))
}
