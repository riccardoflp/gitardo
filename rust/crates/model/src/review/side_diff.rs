//! What one side of a conflict changed of the base, as a diff shows it, numbered as in the file.

use std::ops::Range;

use super::file::{ReviewConflict, ReviewFile, Which};
use crate::diff::LineChange;

/// How many lines of the file are shown before and after.
const AROUND: usize = 3;

/// The marks of the words that changed between two lines, as ranges of bytes of each (`diff.word-diff`).
pub type WordMarks<'a> = &'a dyn Fn(&str, &str) -> (Vec<Range<usize>>, Vec<Range<usize>>);

/// The changes from the lines of the base to those of a side (`diff.line-diff`).
pub type LineChanges<'a> = &'a dyn Fn(&[&str], &[&str]) -> Vec<LineChange>;

/// What the diff of a side is made from.
#[derive(Clone, Copy)]
pub struct SideDiffInput<'a> {
    /// The file.
    pub file: &'a ReviewFile,
    /// The conflict.
    pub conflict: &'a ReviewConflict,
    /// Which side.
    pub side: Which,
    /// The line diff.
    pub changes: LineChanges<'a>,
    /// The word diff.
    pub words: WordMarks<'a>,
}

/// A row of what a side changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SideRow {
    /// A line alike in the base and the side, numbered as the file would number it with that side in place of the
    /// conflict.
    Same {
        /// Its number.
        number: usize,
        /// Its text.
        text: String,
    },
    /// A line of the base the side took out.
    Removed {
        /// Its text.
        text: String,
        /// The words that changed.
        words: Vec<Range<usize>>,
    },
    /// A line the side wrote in their place.
    Added {
        /// Its text.
        text: String,
        /// The words that changed.
        words: Vec<Range<usize>>,
    },
}

/// What a side of a conflict changed of the base, between lines of the file around it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideDiff {
    /// The lines of the file before, with their numbers in the file.
    pub before: Vec<(usize, String)>,
    /// The rows of the side.
    pub rows: Vec<SideRow>,
    /// The lines of the file after, with their numbers in the file.
    pub after: Vec<(usize, String)>,
    /// Whether the side is compared with the base: not when the rules found no stretch, and its lines are shown as
    /// they are.
    pub is_compared: bool,
}

/// What a side of a conflict changed of the base: the lines of the stretch, the base's taken out with a minus over
/// the side's written with a plus, the words that changed marked, the lines alike as context; and 3 lines of the file
/// before and after the stretch, numbered as in the file. When the stretch is not known, the side's lines between the
/// markers as they are.
///
/// Feature: review.side-diff
#[must_use]
pub fn side_diff(input: &SideDiffInput<'_>) -> SideDiff {
    let lines = input.file.lines();
    let text = |index: usize| lines.get(index).map_or_default(|(line, _)| (*line).to_owned());
    let conflict = input.conflict;
    let (file_lines, rows, is_compared) = if let Some(stretch) = &conflict.stretch {
        let side = match input.side {
            Which::Current => &stretch.current,
            Which::Incoming => &stretch.incoming,
        };
        let rows = compared(input, &stretch.base, side, stretch.file_lines.start + 1);
        (stretch.file_lines.clone(), rows, true)
    } else {
        {
            let side = match input.side {
                Which::Current => &conflict.current,
                Which::Incoming => &conflict.incoming,
            };
            let rows = side
                .iter()
                .enumerate()
                .map(|(index, line)| SideRow::Same {
                    number: conflict.span.start + 1 + index,
                    text: line.clone(),
                })
                .collect();
            (conflict.span.clone(), rows, false)
        }
    };
    let before = (file_lines.start.saturating_sub(AROUND)..file_lines.start)
        .map(|index| (index + 1, text(index)))
        .collect();
    let after = (file_lines.end..(file_lines.end + AROUND).min(lines.len()))
        .map(|index| (index + 1, text(index)))
        .collect();
    SideDiff {
        before,
        rows,
        after,
        is_compared,
    }
}

/// The rows of a side compared with the base, its lines numbered from a number.
fn compared(input: &SideDiffInput<'_>, base: &[String], side: &[String], first: usize) -> Vec<SideRow> {
    let base: Vec<&str> = base.iter().map(String::as_str).collect();
    let side: Vec<&str> = side.iter().map(String::as_str).collect();
    let changes = (input.changes)(&base, &side);
    let mut rows = Vec::new();
    let (mut old, mut new) = (0, 0);
    let same_until = |rows: &mut Vec<SideRow>, (old, new): (&mut usize, &mut usize), until: usize| {
        while *new < until {
            if let Some(line) = side.get(*new) {
                rows.push(SideRow::Same {
                    number: first + *new,
                    text: (*line).to_owned(),
                });
            }
            *old += 1;
            *new += 1;
        }
    };
    for change in &changes {
        same_until(&mut rows, (&mut old, &mut new), change.right.start);
        changed(input, (&base, &side), change, &mut rows);
        old = change.left.end;
        new = change.right.end;
    }
    same_until(&mut rows, (&mut old, &mut new), side.len());
    rows
}

/// The rows of a change: the lines taken out over those written, the words of the lines paired one by one marked.
fn changed(input: &SideDiffInput<'_>, (base, side): (&[&str], &[&str]), change: &LineChange, rows: &mut Vec<SideRow>) {
    let removed = base.get(change.left.clone()).unwrap_or_default();
    let added = side.get(change.right.clone()).unwrap_or_default();
    let marks: Vec<_> = removed
        .iter()
        .zip(added)
        .map(|(left, right)| (input.words)(left, right))
        .collect();
    for (index, line) in removed.iter().enumerate() {
        rows.push(SideRow::Removed {
            text: (*line).to_owned(),
            words: marks.get(index).map_or_default(|(left, _)| left.clone()),
        });
    }
    for (index, line) in added.iter().enumerate() {
        rows.push(SideRow::Added {
            text: (*line).to_owned(),
            words: marks.get(index).map_or_default(|(_, right)| right.clone()),
        });
    }
}
