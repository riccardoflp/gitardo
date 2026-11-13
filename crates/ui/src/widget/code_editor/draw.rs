//! Drawing an editor of code: the numbers of the lines in sight, the selection and the marks behind their text, the
//! text in the colors of its tokens, and the caret.

use std::ops::Range;

use render::{Border, CodeKind, Color, Point, Rect, TextRun};

use super::{CodeEditor, Layout, blink};
use crate::frame::{TextKind, Ui};
use crate::interaction::Response;
use crate::memory::CodeEditorState;
use crate::number;

/// How many columns a character takes after so many columns: a tab goes to the next stop.
pub(crate) fn columns_of(character: char, before: f32, tab_width: usize) -> usize {
    if character == '\t' {
        let width = tab_width.max(1);
        let reached = number::index_at(f64::from(before), 1.0);
        width - reached % width
    } else {
        1
    }
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

impl Ui<'_> {
    /// Draws the lines of an editor in sight, and gives them.
    pub(crate) fn draw_code(
        &mut self,
        editor: &CodeEditor<'_>,
        layout: Layout,
        state: &CodeEditorState,
        response: &Response,
    ) -> Range<usize> {
        let height = f64::from(layout.line_height);
        let offset = state.scroll.offset();
        let count = editor.buffer.line_count();
        let first = number::index_at(offset, height).min(count);
        let shown_count = number::index_at(f64::from(layout.text.height), height) + 2;
        let lines = first..first.saturating_add(shown_count).min(count);
        let first_top = number::narrow(number::wide_of_count(first) * height - offset);
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
                let row = Rect::new(layout.text.x, top, layout.text.width, layout.line_height);
                ui.line_number(layout, (line, top), line == caret_line);
                ui.backgrounds(editor, layout, (line, row));
                ui.line_text(editor, layout, (line, row));
                top += layout.line_height;
            }
        });
        if response.has_focus && self.is_window_focused() {
            self.draw_code_caret(editor, layout, (state, first_top + layout.text.y, first));
        }
        lines
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

    /// The parts of a line behind its text: what is selected, and what a search found, the current find outlined.
    fn backgrounds(&mut self, editor: &CodeEditor<'_>, layout: Layout, (line, row): (usize, Rect)) {
        let colors = self.theme().colors;
        let range = editor.buffer.line_range(line);
        let selection = editor.cursor.range();
        if !selection.is_empty() {
            let _selected = self.span_background(editor, layout, (line, row), (selection, colors.text_selection));
        }
        let first = editor.marks.partition_point(|mark| mark.end < range.start);
        for (index, mark) in editor.marks.iter().enumerate().skip(first) {
            if mark.start > range.end {
                break;
            }
            let rect = self.span_background(editor, layout, (line, row), (mark.clone(), colors.find_match));
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

    /// Fills the part of a line a range of the text covers, its line break as a column when it covers it too.
    fn span_background(
        &mut self,
        editor: &CodeEditor<'_>,
        layout: Layout,
        (line, row): (usize, Rect),
        (span, color): (Range<usize>, Color),
    ) -> Option<Rect> {
        let range = editor.buffer.line_range(line);
        let start = span.start.max(range.start);
        let end = span.end.min(range.end);
        let takes_break = span.end > range.end && span.start <= range.end && line + 1 < editor.buffer.line_count();
        if start > end || (start == end && !takes_break) {
            return None;
        }
        let text = editor.buffer.line(line);
        let left = columns_to(text, start - range.start, editor.tab_width);
        let right = columns_to(text, end - range.start, editor.tab_width) + if takes_break { 1.0 } else { 0.0 };
        let rect = Rect::new(
            layout.text.x + left * layout.advance,
            row.y,
            (right - left) * layout.advance,
            row.height,
        );
        self.fill_rect(rect, color);
        Some(rect)
    }

    /// The text of a line, each part in the color of its token, the rest in the color of text.
    fn line_text(&mut self, editor: &CodeEditor<'_>, layout: Layout, (line, row): (usize, Rect)) {
        let text = editor.buffer.line(line);
        if text.is_empty() {
            return;
        }
        let theme = *self.theme();
        let plain = theme.colors.foreground;
        let spans = editor.colors.get(line).map_or(&[][..], Vec::as_slice);
        let mut parts: Vec<(Range<usize>, CodeKind)> = Vec::with_capacity(spans.len() * 2 + 1);
        let mut at = 0;
        for span in spans {
            if span.range.end > text.len() || span.range.start < at {
                continue;
            }
            if span.range.start > at {
                parts.push((at..span.range.start, CodeKind::Plain));
            }
            parts.push((span.range.clone(), span.kind));
            at = span.range.end;
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
            if x > row.right() {
                break;
            }
            let run = TextRun {
                text: &drawn,
                style,
                color: theme.code.color(kind, plain),
                origin: Point::new(x, row.y + 2.0),
                max_width: None,
            };
            let _ = self.text(&run);
        }
    }

    /// The caret, while it is in the visible half of its blink, and the frame where it turns asked for.
    fn draw_code_caret(
        &mut self,
        editor: &CodeEditor<'_>,
        layout: Layout,
        (state, top, first): (&CodeEditorState, f32, usize),
    ) {
        let (is_shown, next) = blink(state.changed_at, self.now());
        self.request_frame_at(next);
        let position = editor.buffer.position(editor.cursor.head);
        let Some(row) = position.line.checked_sub(first) else {
            return;
        };
        let text = editor.buffer.line(position.line);
        let x = layout.text.x + columns_to(text, position.column, editor.tab_width) * layout.advance;
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
