//! Drawing an editor of code: the numbers of the lines in sight, the selection and the marks behind their text, the
//! text in the colors of its tokens, and the caret. A line that wraps is drawn as rows, its number on the first.

use std::ops::Range;

use render::{Border, CodeKind, Color, Point, Rect, TextRun};

use super::{CodeEditor, CodeRows, Layout, blink};
use crate::frame::{TextKind, Ui};
use crate::interaction::Response;
use crate::memory::CodeEditorState;
use crate::number;

/// How many columns a character takes after so many columns: a tab goes to the next stop.
pub(crate) fn columns_of(character: char, before: f32, tab_width: usize) -> usize {
    editor::columns_of(character, number::index_at(f64::from(before), 1.0), tab_width)
}

/// How many columns the whole text of a line takes, its tabs going to their stops: where a view draws after it.
#[must_use]
pub fn code_columns(line: &str, tab_width: usize) -> f32 {
    columns_to(line, line.len(), tab_width)
}

/// How many columns the text of a line takes up to a byte.
fn columns_to(line: &str, end: usize, tab_width: usize) -> f32 {
    let mut columns = 0.0_f32;
    for character in line.get(..end).unwrap_or_default().chars() {
        columns += number::float_of_count(columns_of(character, columns, tab_width));
    }
    columns
}

/// A part of a line as it is drawn, its tabs made spaces, from a column.
fn shown(part: &str, from: f32, tab_width: usize) -> String {
    let mut columns = from;
    let mut text = String::with_capacity(part.len());
    for character in part.chars() {
        let width = columns_of(character, columns, tab_width);
        if character == '\t' {
            text.extend(std::iter::repeat_n(' ', width));
        } else {
            text.push(character);
        }
        columns += number::float_of_count(width);
    }
    text
}

/// A row of a line as it is drawn: the line, the bytes of it on the row, where the row is, and whether it is the last
/// row of the line, which takes its line break.
#[derive(Clone, Debug)]
struct RowPlace {
    /// The line, from 0.
    line: usize,
    /// The bytes of the line on this row.
    piece: Range<usize>,
    /// Where the row is, over the text.
    rect: Rect,
    /// Whether it is the last row of its line.
    is_last: bool,
}

impl Ui<'_> {
    /// Draws the lines of an editor in sight, and gives them with where the first of them starts, down: above the top
    /// of the text when that line, or the row of it in sight, is scrolled into.
    pub(crate) fn draw_code(
        &mut self,
        editor: &CodeEditor<'_>,
        (layout, rows): (Layout, &CodeRows),
        state: &CodeEditorState,
        response: &Response,
    ) -> (Range<usize>, f32) {
        let height = f64::from(layout.line_height);
        let offset = state.scroll.offset();
        let count = editor.buffer.line_count();
        let first_row = number::index_at(offset, height);
        let first = if first_row >= rows.count() {
            count
        } else {
            rows.line_at(first_row).0
        };
        let shown_rows = number::index_at(f64::from(layout.text.height), height) + 2;
        let last_row = first_row.saturating_add(shown_rows);
        let end = (first..count)
            .find(|line| rows.first_row(*line) >= last_row)
            .unwrap_or(count);
        let lines = first..end;
        let first_top = number::narrow(number::wide_of_count(rows.first_row(first)) * height - offset);
        let whole = Rect::new(
            layout.numbers.x,
            layout.text.y,
            layout.text.right() - layout.numbers.x,
            layout.text.height,
        );
        let caret_line = editor.buffer.position(editor.cursor.head).line;
        self.clip(whole, |ui| {
            let mut top = layout.text.y + first_top;
            for line in lines.clone() {
                ui.line_number(layout, (line, top), line == caret_line);
                let pieces = rows.pieces(editor.buffer.line(line));
                let last = pieces.len().saturating_sub(1);
                for (index, piece) in pieces.into_iter().enumerate() {
                    let rect = Rect::new(layout.text.x, top, layout.text.width, layout.line_height);
                    top += layout.line_height;
                    if rect.bottom() < layout.text.y || rect.y > layout.text.bottom() {
                        continue;
                    }
                    let place = RowPlace {
                        line,
                        piece,
                        rect,
                        is_last: index == last,
                    };
                    ui.backgrounds(editor, layout, &place);
                    ui.line_text(editor, layout, &place);
                }
            }
        });
        if response.has_focus && self.is_window_focused() {
            self.draw_code_caret(editor, (layout, rows), (state, first_top + layout.text.y, first));
        }
        (lines, layout.text.y + first_top)
    }

    /// The number of a line, right of its column, brighter on the line of the caret.
    fn line_number(&mut self, layout: Layout, (line, top): (usize, f32), is_caret_line: bool) {
        let colors = self.theme().colors;
        let style = self.text_style(TextKind::Code);
        let number = (line + 1).to_string();
        let width = self.measure(&number, style);
        let right = layout.numbers.right() - self.theme().sizes.spacing.medium;
        let run = TextRun {
            text: &number,
            style,
            color: if is_caret_line {
                colors.foreground
            } else {
                colors.muted_foreground
            },
            origin: Point::new(right - width, top + 2.0),
            max_width: None,
        };
        let _ = self.text(&run);
    }

    /// The parts of a row behind its text: what is selected, and what a search found, the current find outlined.
    fn backgrounds(&mut self, editor: &CodeEditor<'_>, layout: Layout, place: &RowPlace) {
        let colors = self.theme().colors;
        let range = editor.buffer.line_range(place.line);
        let selection = editor.cursor.range();
        if !selection.is_empty() {
            let _selected = self.span_background(editor, layout, place, (selection, colors.text_selection));
        }
        let first = editor.marks.partition_point(|mark| mark.end < range.start);
        for (index, mark) in editor.marks.iter().enumerate().skip(first) {
            if mark.start > range.end {
                break;
            }
            let rect = self.span_background(editor, layout, place, (mark.clone(), colors.find_match));
            if editor.current_mark == Some(index)
                && let Some(rect) = rect
            {
                let border = Border {
                    width: self.theme().sizes.focus_ring,
                    radius: 0.0,
                    color: colors.focus_ring,
                };
                self.border(rect, border);
            }
        }
    }

    /// Fills the part of a row a range of the text covers, the line break as a column after the last row of its line
    /// when the range covers it too.
    fn span_background(
        &mut self,
        editor: &CodeEditor<'_>,
        layout: Layout,
        place: &RowPlace,
        (span, color): (Range<usize>, Color),
    ) -> Option<Rect> {
        let line_start = editor.buffer.line_range(place.line).start;
        let row = line_start + place.piece.start..line_start + place.piece.end;
        let start = span.start.max(row.start);
        let end = span.end.min(row.end);
        let has_break = place.is_last && place.line + 1 < editor.buffer.line_count();
        let takes_break = has_break && span.end > row.end && span.start <= row.end;
        if start > end || (start == end && !takes_break) {
            return None;
        }
        let text = editor
            .buffer
            .line(place.line)
            .get(place.piece.clone())
            .unwrap_or_default();
        let left = columns_to(text, start - row.start, editor.tab_width);
        let right = columns_to(text, end - row.start, editor.tab_width) + if takes_break { 1.0 } else { 0.0 };
        let rect = Rect::new(
            layout.text.x + left * layout.advance,
            place.rect.y,
            (right - left) * layout.advance,
            place.rect.height,
        );
        self.fill_rect(rect, color);
        Some(rect)
    }

    /// The text of a row, each part in the color of its token, the rest in the color of text.
    fn line_text(&mut self, editor: &CodeEditor<'_>, layout: Layout, place: &RowPlace) {
        let line = editor.buffer.line(place.line);
        let piece = place.piece.clone();
        let Some(text) = line.get(piece.clone()).filter(|text| !text.is_empty()) else {
            return;
        };
        let theme = *self.theme();
        let plain = theme.colors.foreground;
        let spans = editor.colors.get(place.line).map_or(&[][..], Vec::as_slice);
        let mut parts: Vec<(Range<usize>, CodeKind)> = Vec::with_capacity(spans.len() * 2 + 1);
        let mut at = 0;
        for span in spans {
            if span.range.end > line.len() || span.range.end <= piece.start || span.range.start >= piece.end {
                continue;
            }
            // The part of the token on this row, from the start of the row; a token over one before it is left plain.
            let start = span.range.start.max(piece.start) - piece.start;
            let end = span.range.end.min(piece.end) - piece.start;
            if start < at {
                continue;
            }
            if start > at {
                parts.push((at..start, CodeKind::Plain));
            }
            parts.push((start..end, span.kind));
            at = end;
        }
        if at < text.len() {
            parts.push((at..text.len(), CodeKind::Plain));
        }
        let style = self.text_style(TextKind::Code);
        let mut columns = 0.0;
        for (range, kind) in parts {
            let part = text.get(range.clone()).unwrap_or_default();
            let drawn = shown(part, columns, editor.tab_width);
            let x = layout.text.x + columns * layout.advance;
            columns = columns_to(text, range.end, editor.tab_width);
            if x > place.rect.right() {
                break;
            }
            let run = TextRun {
                text: &drawn,
                style,
                color: theme.code.color(kind, plain),
                origin: Point::new(x, place.rect.y + 2.0),
                max_width: None,
            };
            let _ = self.text(&run);
        }
    }

    /// The caret, while it is in the visible half of its blink, and the frame where it turns asked for.
    fn draw_code_caret(
        &mut self,
        editor: &CodeEditor<'_>,
        (layout, rows): (Layout, &CodeRows),
        (state, top, first): (&CodeEditorState, f32, usize),
    ) {
        let (is_shown, next) = blink(state.changed_at, self.now());
        self.request_frame_at(next);
        let position = editor.buffer.position(editor.cursor.head);
        if position.line < first {
            return;
        }
        let text = editor.buffer.line(position.line);
        let pieces = rows.pieces(text);
        let piece_index = editor::row_of(&pieces, position.column);
        let piece_start = pieces.get(piece_index).map_or(0, |piece| piece.start);
        let row = rows.first_row(position.line) - rows.first_row(first) + piece_index;
        let piece = text.get(piece_start..).unwrap_or_default();
        let x = layout.text.x + columns_to(piece, position.column - piece_start, editor.tab_width) * layout.advance;
        let y = top + number::float_of_count(row) * layout.line_height;
        let caret = Rect::new(x, y, self.theme().sizes.border.max(1.0) * 2.0, layout.line_height);
        if !layout.text.contains(Point::new(x, y)) && y + layout.line_height < layout.text.y {
            return;
        }
        self.set_caret(caret);
        if is_shown && !editor.is_read_only {
            let color = self.theme().colors.foreground;
            self.clip(layout.text, |ui| ui.fill_rect(caret, color));
        }
    }
}
