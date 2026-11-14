//! The lines of the two sides of a diff in rows, so that unchanged lines face each other and each change takes the same
//! height on both sides (`diff.align`).

use std::ops::Range;

/// A change between the two sides, by the lines it takes on each, counted from 0; one side may have none, and then
/// its empty range is the place meant: the line before it is the last line before the change.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineChange {
    /// Its lines on the left.
    pub left: Range<usize>,
    /// Its lines on the right.
    pub right: Range<usize>,
}

impl LineChange {
    /// What it does to the lines.
    #[must_use]
    pub fn kind(&self) -> LineChangeKind {
        match (self.left.is_empty(), self.right.is_empty()) {
            (true, false) => LineChangeKind::Added,
            (false, true) => LineChangeKind::Removed,
            _ => LineChangeKind::Changed,
        }
    }
}

/// What a change does to the lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineChangeKind {
    /// Lines of the right side only: green.
    Added,
    /// Lines of the left side only: red.
    Removed,
    /// Lines of the left replaced by lines of the right: blue.
    Changed,
}

/// The parts of a pair of lines that changed, as ranges of the bytes of the left line and of the right one.
pub type WordMarks = (Vec<Range<usize>>, Vec<Range<usize>>);

/// What the rows are made from: the lines of each side, the changes between them in order, and how the words that
/// changed in two paired lines are found (`diff.word-diff`).
pub struct AlignInput<'a> {
    /// The lines of the left side.
    pub left: &'a [&'a str],
    /// The lines of the right side.
    pub right: &'a [&'a str],
    /// The changes, in order, none touching the next.
    pub changes: &'a [LineChange],
    /// The words that changed in two paired lines.
    pub word_marks: &'a dyn Fn(&str, &str) -> WordMarks,
}

/// The two sides in rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AlignedDiff {
    /// The rows, from the top.
    pub rows: Vec<AlignedRow>,
    /// The rows of each change, in order.
    pub changes: Vec<ChangeRows>,
}

/// A row: a line of each side, or a line of one side facing a filler row on the other.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AlignedRow {
    /// The line of the left side; none for a filler row, which is blank and has no number.
    pub left: Option<RowLine>,
    /// The line of the right side; none for a filler row.
    pub right: Option<RowLine>,
    /// The change the row is of, by its place; none for an unchanged row.
    pub change: Option<usize>,
}

/// A line in a row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowLine {
    /// Which line of its side, from 0.
    pub line: usize,
    /// The parts of it that changed, when it is paired with a line of the other side.
    pub marks: Vec<Range<usize>>,
}

/// The rows of a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRows {
    /// What it does.
    pub kind: LineChangeKind,
    /// Its rows.
    pub rows: Range<usize>,
}

/// The lines of the two sides in rows: an unchanged line is one row with the same line on both sides; in a change of
/// `L` lines on the left and `R` on the right, the first `min(L, R)` lines face each other with their word marks, and
/// the lines left of the longer side face filler rows. Both sides have the same number of rows, and every line of each
/// side is in exactly one row, in order.
///
/// Feature: diff.align
#[must_use]
pub fn align(input: &AlignInput<'_>) -> AlignedDiff {
    let mut diff = AlignedDiff::default();
    let (mut left, mut right) = (0, 0);
    for (index, change) in input.changes.iter().enumerate() {
        unchanged(&mut diff, (&mut left, &mut right), change.left.start);
        let start = diff.rows.len();
        let paired = change.left.len().min(change.right.len());
        for offset in 0..change.left.len().max(change.right.len()) {
            let (left_line, right_line) = (change.left.start + offset, change.right.start + offset);
            let (left_marks, right_marks) = if offset < paired {
                (input.word_marks)(line_text(input.left, left_line), line_text(input.right, right_line))
            } else {
                (Vec::new(), Vec::new())
            };
            diff.rows.push(AlignedRow {
                left: (offset < change.left.len()).then_some(RowLine {
                    line: left_line,
                    marks: left_marks,
                }),
                right: (offset < change.right.len()).then_some(RowLine {
                    line: right_line,
                    marks: right_marks,
                }),
                change: Some(index),
            });
        }
        diff.changes.push(ChangeRows {
            kind: change.kind(),
            rows: start..diff.rows.len(),
        });
        left = change.left.end;
        right = change.right.end;
    }
    unchanged(&mut diff, (&mut left, &mut right), input.left.len());
    diff
}

/// The text of a line, empty past the end.
fn line_text<'a>(lines: &[&'a str], line: usize) -> &'a str {
    lines.get(line).copied().unwrap_or_default()
}

/// The unchanged rows from where each side is up to a line of the left side.
fn unchanged(diff: &mut AlignedDiff, (left, right): (&mut usize, &mut usize), until: usize) {
    while *left < until {
        diff.rows.push(AlignedRow {
            left: Some(RowLine {
                line: *left,
                marks: Vec::new(),
            }),
            right: Some(RowLine {
                line: *right,
                marks: Vec::new(),
            }),
            change: None,
        });
        *left += 1;
        *right += 1;
    }
}
