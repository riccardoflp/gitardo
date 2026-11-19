//! The lines selected in a Diff page (`diff.select-lines`, D78): rows of one file, on either side, drawn with the
//! background of a selection on both sides and in the gutter, and staged, unstaged or reverted from the context menu of
//! a side or with `Alt+S`, `Alt+U` and `Alt+R`.
//!
//! On a side that is not edited, a click chooses a line and `Shift` with a click, an arrow or a drag selects the rows
//! from it; on the side that is edited, the rows are those of the text selected.

use std::ops::Range;

use model::contract::{FeatureId, Target};
use model::diff::{AlignedRow, DiffChange, RowLine};
use ui::{Icon, Key, KeyEvent, MenuEntry, Modifiers, NamedKey, Ui};

use super::edit::{DrawnSides, side_rect};
use super::{
    CaretSide, DiffCaret, DiffEditIntent, DiffIntent, DiffPageState, DiffText, DiffVersion, FileDiff, PageLine,
};
use crate::feature::FeatureIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::diff as words;

/// Rows of a file selected on a side that is not edited: from the row the selection started on to the row it reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowSelection {
    /// The file, by its place in the page.
    pub file: usize,
    /// The row it started on.
    pub anchor: usize,
    /// The row it reaches, where `Shift` with an arrow goes on from.
    pub head: usize,
}

impl RowSelection {
    /// The rows, from the first to the last.
    #[must_use]
    pub fn rows(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head) + 1
    }
}

/// What is done with the lines selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinesAction {
    /// **Stage lines** (`diff.stage-lines`).
    Stage,
    /// **Unstage lines** (`diff.unstage-lines`).
    Unstage,
    /// **Revert lines** (`diff.revert-lines`).
    Revert,
}

impl LinesAction {
    /// Its operation.
    fn feature(self) -> FeatureId {
        match self {
            LinesAction::Stage => FeatureId("diff.stage-lines"),
            LinesAction::Unstage => FeatureId("diff.unstage-lines"),
            LinesAction::Revert => FeatureId("diff.revert-lines"),
        }
    }

    /// Its icon in the context menu.
    pub(super) fn icon(self) -> Icon {
        match self {
            LinesAction::Stage => icons::ADD,
            LinesAction::Unstage => icons::UNSTAGE,
            LinesAction::Revert => icons::UNDO,
        }
    }

    /// Its name in the context menu.
    pub(super) fn label(self) -> &'static str {
        match self {
            LinesAction::Stage => words::STAGE_LINES,
            LinesAction::Unstage => words::UNSTAGE_LINES,
            LinesAction::Revert => words::REVERT_LINES,
        }
    }

    /// Its keys, as the menu shows them.
    pub(super) fn keys(self) -> &'static str {
        match self {
            LinesAction::Stage => words::STAGE_LINES_KEYS,
            LinesAction::Unstage => words::UNSTAGE_LINES_KEYS,
            LinesAction::Revert => words::REVERT_LINES_KEYS,
        }
    }

    /// The action of a key: `Alt` with `S`, `U` or `R` (`Ctrl+Option` on macOS, which holds `Alt` too).
    fn of_key(key: &KeyEvent) -> Option<LinesAction> {
        if !key.modifiers.contains(Modifiers::ALT) || key.modifiers.shift() {
            return None;
        }
        match key.key {
            Key::Character('s' | 'S') => Some(LinesAction::Stage),
            Key::Character('u' | 'U') => Some(LinesAction::Unstage),
            Key::Character('r' | 'R') => Some(LinesAction::Revert),
            _ => None,
        }
    }
}

/// The sides of a page as its list drew them, whose selected rows are drawn.
#[derive(Clone, Copy, Debug)]
pub struct LineSelectionState<'a> {
    /// The sides.
    pub sides: &'a DrawnSides<'a>,
}

/// The rows selected drawn with the background of a selection on both sides and in the gutter, over the colors of the
/// changes.
///
/// Feature: diff.select-lines
pub fn select_lines_view(state: &LineSelectionState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let sides = state.sides;
    let page = sides.state;
    let Some((file, rows)) = selected_rows(page) else {
        return Vec::new();
    };
    let color = ui.theme().colors.text_selection.with_alpha(0x50);
    for (page_line, rect) in &sides.rows {
        let Some(PageLine::Row(row_file, row)) = page.lines.get(*page_line) else {
            continue;
        };
        if *row_file != file || !rows.contains(row) {
            continue;
        }
        for column in [sides.columns.left, sides.columns.gutter, sides.columns.right] {
            ui.fill_rect(side_rect(column, *rect), color);
        }
    }
    Vec::new()
}

/// The rows selected, of one file: those of the selection of a side that is not edited, or the rows of the lines of
/// the text selected on the side that is edited, a selection inside one line taking its row.
#[must_use]
pub fn selected_rows(state: &DiffPageState<'_>) -> Option<(usize, Range<usize>)> {
    if let Some(selection) = state.selection {
        return Some((selection.file, selection.rows()));
    }
    let Some(DiffCaret::Text { file, buffer, cursor }) = state.caret else {
        return None;
    };
    if !cursor.has_selection() {
        return None;
    }
    let Some(FileDiff::Text(text)) = state.diffs.get(file) else {
        return None;
    };
    let (start, end) = (
        buffer.position(cursor.range().start),
        buffer.position(cursor.range().end),
    );
    // A selection that ends at the start of a line does not take that line.
    let last = if end.column == 0 && end.line > start.line {
        end.line - 1
    } else {
        end.line
    };
    let row_of = |line: usize| {
        text.aligned
            .rows
            .iter()
            .position(|row| row.right.as_ref().is_some_and(|right| right.line == line))
    };
    Some((file, row_of(start.line)?..row_of(last)? + 1))
}

/// What the lines selected of a file offer, each where its card offers it: **Stage lines** for the working tree
/// against `HEAD` or the index, **Unstage lines** against `HEAD`, neither for a renamed file; **Revert lines** for the
/// working tree. None for a file in conflict, whose base is on the left.
pub(super) fn offered(state: &DiffPageState<'_>, file: usize) -> Vec<LinesAction> {
    let (left, right) = state.versions;
    if *left == DiffVersion::Base || !matches!(state.diffs.get(file), Some(FileDiff::Text(_))) {
        return Vec::new();
    }
    let is_renamed = state.files.get(file).is_none_or(|paths| paths.left != paths.right);
    let mut actions = Vec::new();
    if !is_renamed && *right == DiffVersion::WorkingTree && matches!(left, DiffVersion::Head | DiffVersion::Index) {
        actions.push(LinesAction::Stage);
    }
    if !is_renamed && *left == DiffVersion::Head && matches!(right, DiffVersion::Index | DiffVersion::WorkingTree) {
        actions.push(LinesAction::Unstage);
    }
    if *right == DiffVersion::WorkingTree {
        actions.push(LinesAction::Revert);
    }
    actions
}

/// The entries of the context menu for the lines selected: each action the page offers, and whether it can be chosen,
/// which it can when some row selected is of a change and the rows are not waiting for the diff to be computed again.
pub(super) fn menu_actions(state: &DiffPageState<'_>) -> Vec<(LinesAction, bool)> {
    let Some((file, rows)) = selected_rows(state) else {
        return Vec::new();
    };
    let has_change = match state.diffs.get(file) {
        Some(FileDiff::Text(text)) => has_changed_row(&text.aligned.rows, &rows),
        _ => false,
    };
    offered(state, file)
        .into_iter()
        .map(|action| (action, has_change && !state.is_stale))
        .collect()
}

/// The entries of the context menu of a side for the lines selected, after a separator, each greyed out when it cannot
/// be chosen; none without lines selected.
pub(super) fn menu_entries<C: Copy>(
    state: &DiffPageState<'_>,
    choice: impl Fn(LinesAction) -> C,
) -> Vec<MenuEntry<'static, C>> {
    let actions = menu_actions(state);
    if actions.is_empty() {
        return Vec::new();
    }
    let mut entries = vec![MenuEntry::Separator];
    for (action, is_on) in actions {
        let item = icons::action(action.icon(), action.label(), choice(action)).with_shortcut(action.keys());
        entries.push(MenuEntry::Item(if is_on { item } else { item.disabled() }));
    }
    entries
}

/// Whether some of the rows is of a change.
fn has_changed_row(rows: &[AlignedRow], selected: &Range<usize>) -> bool {
    rows.get(selected.clone())
        .is_some_and(|rows| rows.iter().any(|row| row.change.is_some()))
}

/// What an action asks with the lines selected: the operation run on them, the selection cleared; nothing when the
/// page does not offer it or no row selected is of a change.
pub(super) fn act(state: &DiffPageState<'_>, action: LinesAction) -> Vec<Intent> {
    let Some((file, rows)) = selected_rows(state) else {
        return Vec::new();
    };
    let can = menu_actions(state).contains(&(action, true));
    let Some(FileDiff::Text(text)) = state.diffs.get(file) else {
        return Vec::new();
    };
    if !can {
        return Vec::new();
    }
    let edit = |intent| Intent::Diff(DiffIntent::Edit(state.id, intent));
    if action == LinesAction::Revert {
        return vec![edit(DiffEditIntent::RevertLines { file, rows })];
    }
    let target = Target::DiffChange(Box::new(selection_change(state, (file, text), &rows)));
    vec![
        Intent::Feature(FeatureIntent::Run(action.feature(), target)),
        edit(DiffEditIntent::ClearSelection),
    ]
}

/// The rows selected as the operations are given them: the lines of each side in those rows, and their text.
fn selection_change(state: &DiffPageState<'_>, (file, text): (usize, &DiffText), rows: &Range<usize>) -> DiffChange {
    let (left, right) = state.versions;
    let (left_lines, right_lines) = lines_of_rows(text, rows);
    DiffChange {
        path: state.files.get(file).map_or_default(|paths| paths.right.clone()),
        left: super::gutter::side_of(left),
        right: super::gutter::side_of(right),
        left_text: left_lines.clone().map(|line| text.left_line(line).to_owned()).collect(),
        right_text: right_lines
            .clone()
            .map(|line| text.right_line(line).to_owned())
            .collect(),
        left_lines,
        right_lines,
    }
}

/// The lines of each side in some rows of a file: from the first line in them to the last, or, where a side has only
/// filler rows there, no line at the place of the next line of that side.
#[must_use]
pub fn lines_of_rows(text: &DiffText, rows: &Range<usize>) -> (Range<usize>, Range<usize>) {
    let side = |pick: fn(&AlignedRow) -> Option<&RowLine>, count: usize| {
        let inside: Vec<usize> = text
            .aligned
            .rows
            .get(rows.clone())
            .unwrap_or_default()
            .iter()
            .filter_map(|row| pick(row).map(|line| line.line))
            .collect();
        if let (Some(first), Some(last)) = (inside.first(), inside.last()) {
            *first..*last + 1
        } else {
            let next = text
                .aligned
                .rows
                .iter()
                .skip(rows.end)
                .find_map(|row| pick(row).map(|line| line.line))
                .unwrap_or(count);
            next..next
        }
    };
    (
        side(|row| row.left.as_ref(), text.left_lines.len()),
        side(|row| row.right.as_ref(), text.right_lines.len()),
    )
}

/// The rows selected from the line chosen to a row of a side that is not edited.
pub(super) fn select_to(state: &DiffPageState<'_>, (file, side, row): (usize, CaretSide, usize)) -> Vec<Intent> {
    vec![Intent::Diff(DiffIntent::Edit(
        state.id,
        DiffEditIntent::Select { file, side, row },
    ))]
}

/// What a key asks of the lines selected while a side has the keyboard: `Escape` clears them, `Alt+S`, `Alt+U` and
/// `Alt+R` stage, unstage or revert them, and on a side that is not edited `Shift` with an arrow or a page key selects
/// the rows from the line chosen. None for a key it does not use.
pub(super) fn side_key(
    sides: &DrawnSides<'_>,
    (side, is_edited): (CaretSide, bool),
    key: &KeyEvent,
    ui: &Ui<'_>,
) -> Option<Vec<Intent>> {
    let state = sides.state;
    let page_rows = crate::number::count_of_float(sides.list.height / super::rows::row_height(ui)).max(1);
    let edit = |intent| vec![Intent::Diff(DiffIntent::Edit(state.id, intent))];
    if let Some(action) = LinesAction::of_key(key) {
        return selected_rows(state).map(|_| act(state, action));
    }
    if key.key == Key::Named(NamedKey::Escape) && key.modifiers.is_empty() {
        return match state.caret {
            _ if state.selection.is_some() => Some(edit(DiffEditIntent::ClearSelection)),
            Some(DiffCaret::Text { file, buffer, cursor }) if cursor.has_selection() => {
                let place = ui::editor::Command::PlaceAt(buffer.position(cursor.head), false);
                Some(edit(DiffEditIntent::Text {
                    file,
                    commands: vec![place],
                    at: ui.now(),
                }))
            }
            _ => None,
        };
    }
    if is_edited || key.modifiers != Modifiers::SHIFT {
        return None;
    }
    let step: isize = match key.key {
        Key::Named(NamedKey::ArrowDown) => 1,
        Key::Named(NamedKey::ArrowUp) => -1,
        Key::Named(NamedKey::PageDown) => isize::try_from(page_rows).unwrap_or(1),
        Key::Named(NamedKey::PageUp) => -isize::try_from(page_rows).unwrap_or(1),
        _ => return None,
    };
    let (file, from) = match (state.selection, state.caret) {
        (Some(selection), _) => (selection.file, selection.head),
        (None, Some(DiffCaret::Line { file, row, .. })) => (file, row),
        _ => return Some(Vec::new()),
    };
    let Some(FileDiff::Text(text)) = state.diffs.get(file) else {
        return Some(Vec::new());
    };
    let last = text.aligned.rows.len().saturating_sub(1);
    let row = from.saturating_add_signed(step).min(last);
    Some(edit(DiffEditIntent::Select { file, side, row }))
}
