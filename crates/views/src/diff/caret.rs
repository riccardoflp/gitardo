//! The caret and the selection of a side of a Diff page over the rows in sight (`diff.edit`): on the working tree side,
//! the selected part of each line and the caret; on a line chosen by a click, the line marked.

use ui::{Rect, Ui};

use super::edit::{DrawnSides, code_advance, mark_length, side_rect};
use super::{CaretSide, DiffCaret, DiffText, FileDiff, PageLine};

/// The selection and the caret of a side over the rows in sight: on the working tree side, the selected part of each
/// line and, while the side has the keyboard, the caret; on a line chosen, the line marked.
pub(super) fn draw_caret(sides: &DrawnSides<'_>, ui: &mut Ui<'_>, (side, has_focus): (CaretSide, bool)) {
    let state = sides.state;
    let Some(caret) = state.caret else {
        return;
    };
    let Some(FileDiff::Text(text)) = state.diffs.get(caret.file()) else {
        return;
    };
    let column = match side {
        CaretSide::Left => sides.columns.left,
        CaretSide::Right => sides.columns.right,
    };
    let colors = ui.theme().colors;
    for (page_line, rect) in &sides.rows {
        let Some(PageLine::Row(file, row)) = state.lines.get(*page_line) else {
            continue;
        };
        if *file != caret.file() {
            continue;
        }
        let cell = side_rect(column, *rect);
        match caret {
            DiffCaret::Line {
                side: chosen,
                row: chosen_row,
                ..
            } if chosen == side && chosen_row == *row => {
                ui.fill_rect(cell, colors.text_selection.with_alpha(0x50));
            }
            DiffCaret::Text { buffer, cursor, .. } if side == CaretSide::Right => {
                let line = text.aligned.rows.get(*row).and_then(|aligned| aligned.right.as_ref());
                if let Some(line) = line {
                    let place = LinePlace {
                        text,
                        line: line.line,
                        cell,
                    };
                    draw_text_caret(ui, &place, (buffer, cursor, has_focus));
                }
            }
            _ => {}
        }
    }
}

/// A line of the right side as it is drawn: its text, its number, and the side of its row.
struct LinePlace<'a> {
    /// The two sides of its file.
    text: &'a DiffText,
    /// The line.
    line: usize,
    /// The side of its row.
    cell: Rect,
}

/// The selected part of a line of the working tree side, and the caret when it is on the line and the side has the
/// keyboard.
fn draw_text_caret(
    ui: &mut Ui<'_>,
    place: &LinePlace<'_>,
    (buffer, cursor, has_focus): (&ui::editor::Buffer, ui::editor::Cursor, bool),
) {
    let colors = ui.theme().colors;
    let cell = super::rows::text_cell(ui, place.cell);
    let advance = code_advance(ui);
    let shown = place.text.right_line(place.line);
    let mark = mark_length(place.text, place.line);
    let x_of = |column: usize| {
        let before = shown.get(..(column + mark).min(shown.len())).unwrap_or_default();
        let columns: f32 = before
            .chars()
            .map(|character| if character == '\t' { 4.0 } else { 1.0 })
            .sum();
        cell.x + columns * advance
    };
    let (start, end) = (
        buffer.position(cursor.range().start),
        buffer.position(cursor.range().end),
    );
    if cursor.has_selection() && (start.line..=end.line).contains(&place.line) {
        let left = if start.line == place.line {
            x_of(start.column)
        } else {
            cell.x
        };
        let right = if end.line == place.line {
            x_of(end.column)
        } else {
            x_of(shown.len()) + advance
        };
        ui.fill_rect(
            Rect::new(left, cell.y, (right - left).max(0.0), cell.height),
            colors.text_selection,
        );
    }
    let head = buffer.position(cursor.head);
    if has_focus && ui.is_window_focused() && head.line == place.line {
        let caret = Rect::new(
            x_of(head.column),
            cell.y,
            ui.theme().sizes.border.max(1.0) * 2.0,
            cell.height,
        );
        ui.set_caret(caret);
        ui.fill_rect(caret, colors.foreground);
    }
}
