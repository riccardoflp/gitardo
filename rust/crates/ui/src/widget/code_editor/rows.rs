//! The rows the lines of an editor of code take on screen: one each while long lines are cut at the right edge, as many
//! as a line wraps to while they wrap at the width of the editor.

use std::ops::Range;

use editor::{Buffer, WrapWidth, row_count, wrap_line};

/// What an editor does with a line longer than it is wide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineWrap {
    /// It is cut at the right edge.
    #[default]
    Cut,
    /// It wraps into rows as wide as the text, broken between words where it can be: the number of the line, and what
    /// a view keeps before the text, stay on its first row, and the other rows start in the column of the text.
    Words,
}

/// The rows the lines of an editor take, from the top of its text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeRows {
    /// The room of each row while lines wrap; none while they are cut.
    width: Option<WrapWidth>,
    /// While lines wrap, the first row of each line, then how many rows there are; empty while they are cut.
    starts: Vec<usize>,
    /// How many lines there are.
    line_count: usize,
}

impl CodeRows {
    /// The rows of the lines of a text, wrapped in a width or cut.
    pub(crate) fn new(buffer: &Buffer, width: Option<WrapWidth>) -> CodeRows {
        let line_count = buffer.line_count();
        let Some(width) = width else {
            return CodeRows {
                width,
                starts: Vec::new(),
                line_count,
            };
        };
        let mut starts = Vec::with_capacity(line_count + 1);
        let mut row = 0;
        for line in 0..line_count {
            starts.push(row);
            row += row_count(buffer.line(line), width);
        }
        starts.push(row);
        CodeRows {
            width: Some(width),
            starts,
            line_count,
        }
    }

    /// How many rows the lines take.
    #[must_use]
    pub fn count(&self) -> usize {
        self.starts.last().copied().unwrap_or(self.line_count)
    }

    /// The row a line starts on, from the first row of the text.
    #[must_use]
    pub fn first_row(&self, line: usize) -> usize {
        if self.width.is_none() {
            return line.min(self.line_count);
        }
        self.starts.get(line).copied().unwrap_or_else(|| self.count())
    }

    /// How many rows a line takes.
    #[must_use]
    pub fn rows_in(&self, line: usize) -> usize {
        self.first_row(line + 1).saturating_sub(self.first_row(line)).max(1)
    }

    /// The line a row is a row of, and which of its rows it is; the last row for a row past the end.
    #[must_use]
    pub fn line_at(&self, row: usize) -> (usize, usize) {
        let last = self.line_count.saturating_sub(1);
        if self.width.is_none() {
            return (row.min(last), 0);
        }
        let row = row.min(self.count().saturating_sub(1));
        let line = self
            .starts
            .partition_point(|start| *start <= row)
            .saturating_sub(1)
            .min(last);
        (line, row - self.first_row(line))
    }

    /// The rows of the text of one of the lines, as ranges of its bytes: the whole line while lines are cut.
    #[must_use]
    pub fn pieces(&self, text: &str) -> Vec<Range<usize>> {
        match self.width {
            Some(width) => wrap_line(text, width),
            None => std::iter::once(0..text.len()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str, columns: Option<usize>) -> CodeRows {
        let buffer = Buffer::from_bytes(text.as_bytes()).unwrap_or_default();
        CodeRows::new(&buffer, columns.map(|columns| WrapWidth { columns, tab_width: 4 }))
    }

    #[test]
    fn wrapped_lines_take_a_row_for_each_piece_and_a_row_finds_its_line() {
        let wrapped = rows("short\nthe quick brown fox\nend", Some(10));
        assert_eq!(wrapped.count(), 4);
        assert_eq!(
            (0..3)
                .map(|line| (wrapped.first_row(line), wrapped.rows_in(line)))
                .collect::<Vec<_>>(),
            [(0, 1), (1, 2), (3, 1)]
        );
        assert_eq!(
            (0..5).map(|row| wrapped.line_at(row)).collect::<Vec<_>>(),
            [(0, 0), (1, 0), (1, 1), (2, 0), (2, 0)]
        );
        assert_eq!(wrapped.pieces("the quick brown fox"), [0..10, 10..19]);
    }

    #[test]
    fn cut_lines_take_a_row_each() {
        let cut = rows("short\nthe quick brown fox\nend", None);
        assert_eq!(cut.count(), 3);
        assert_eq!((cut.first_row(1), cut.rows_in(1), cut.line_at(1)), (1, 1, (1, 0)));
        assert_eq!(
            cut.pieces("the quick brown fox"),
            std::iter::once(0..19).collect::<Vec<_>>()
        );
    }
}
