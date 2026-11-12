//! The differences between two texts: the lines that changed (`diff.line-diff`), used by every diff of the application,
//! and the words that changed within a line paired with another (`diff.word-diff`).

mod myers;
mod words;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::ops::Range;

pub use words::{WordDiffInput, word_diff};

/// The lines of a text, each without its line break: `\r\n` is read as `\n`, so that a file checked out with other line
/// endings has the same lines. A text that ends with a line break has no empty line after it.
#[must_use]
pub fn lines(text: &str) -> Vec<&str> {
    line_ranges(text)
        .into_iter()
        .map(|range| text.get(range).unwrap_or_default())
        .collect()
}

/// Where each line of a text is, as [`lines`] gives them: the bytes of each, without its line break.
#[must_use]
pub fn line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            ranges.push(start..without_return(text, start, index));
            start = index + 1;
        }
    }
    if start < text.len() {
        ranges.push(start..without_return(text, start, text.len()));
    }
    ranges
}

/// The end of a line before the `\r` of a `\r\n`.
fn without_return(text: &str, start: usize, end: usize) -> usize {
    if end > start && text.as_bytes().get(end - 1) == Some(&b'\r') {
        end - 1
    } else {
        end
    }
}

/// What a change does to the lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    /// Lines of the second text only.
    Added,
    /// Lines of the first text only.
    Removed,
    /// Lines of the first text replaced by lines of the second.
    Changed,
}

/// Where a change is on one side, as a hunk header says it: its first line counted from 1 and how many lines; with no
/// line, the line before the place meant, 0 at the start of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Its first line from 1, or the line before it when it has none.
    pub start: usize,
    /// How many lines.
    pub count: usize,
}

impl Span {
    /// Its lines, counted from 0; with none, the empty range at the place meant.
    #[must_use]
    pub fn lines(&self) -> Range<usize> {
        if self.count == 0 {
            self.start..self.start
        } else {
            let first = self.start.saturating_sub(1);
            first..first + self.count
        }
    }

    /// The span of some lines counted from 0.
    fn of(lines: &Range<usize>) -> Span {
        Span {
            start: if lines.is_empty() { lines.start } else { lines.start + 1 },
            count: lines.len(),
        }
    }
}

/// A change between two texts: lines taken out of the first, put in the second, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Change {
    /// What it does.
    pub kind: ChangeKind,
    /// Where it is in the first text.
    pub left: Span,
    /// Where it is in the second text.
    pub right: Span,
}

/// The two texts of a line diff, as lines, and what stops it.
pub struct LineDiffInput<'a> {
    /// The lines of the first text, as [`lines`] gives them.
    pub left: &'a [&'a str],
    /// The lines of the second text.
    pub right: &'a [&'a str],
    /// Asked between steps of the search: false stops it, as when its time limit has passed. The caller reads the
    /// clock, so that this crate does not.
    pub keep_going: &'a dyn Fn() -> bool,
}

/// The diff stopped before it ended: it has too many differences to compute in its time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TooComplex;

/// The changes between two texts, line by line, in order: a minimal diff, which no other way from the first text to
/// the second beats with fewer lines removed and added. Lines are compared as they are, spaces included. A run of
/// removed lines and a run of added lines with no line between them are one change, of kind *changed*.
///
/// Myers, "An O(ND) Difference Algorithm and Its Variations", Algorithmica 1, 1986 (see `myers`).
///
/// Feature: diff.line-diff
///
/// # Errors
///
/// [`TooComplex`] when `keep_going` says to stop before the diff is done.
pub fn line_diff(input: &LineDiffInput<'_>) -> Result<Vec<Change>, TooComplex> {
    let (left, right) = numbered(input.left, input.right);
    let marks = myers::marks(&left, &right, input.keep_going)?;
    Ok(runs(&marks.removed, &marks.added)
        .into_iter()
        .map(|(left, right)| Change {
            kind: match (left.is_empty(), right.is_empty()) {
                (false, true) => ChangeKind::Removed,
                (true, false) => ChangeKind::Added,
                _ => ChangeKind::Changed,
            },
            left: Span::of(&left),
            right: Span::of(&right),
        })
        .collect())
}

/// Each element of two lists as a number, the same number for the same text, so that the search compares numbers.
pub(crate) fn numbered<'a>(left: &[&'a str], right: &[&'a str]) -> (Vec<u32>, Vec<u32>) {
    let mut numbers: HashMap<&'a str, u32> = HashMap::new();
    let mut number = |text: &'a str| {
        let next = u32::try_from(numbers.len()).unwrap_or(u32::MAX);
        *numbers.entry(text).or_insert(next)
    };
    let left = left.iter().map(|text| number(text)).collect();
    let right = right.iter().map(|text| number(text)).collect();
    (left, right)
}

/// The runs of elements taken out and put in with no kept element between them, as places in each list, in order.
pub(crate) fn runs(removed: &[bool], added: &[bool]) -> Vec<(Range<usize>, Range<usize>)> {
    let mut runs = Vec::new();
    let (mut old, mut new) = (0, 0);
    while old < removed.len() || new < added.len() {
        let start = (old, new);
        loop {
            let before = (old, new);
            while removed.get(old).copied().unwrap_or(false) {
                old += 1;
            }
            while added.get(new).copied().unwrap_or(false) {
                new += 1;
            }
            if (old, new) == before {
                break;
            }
        }
        if (old, new) == start {
            old += 1;
            new += 1;
        } else {
            runs.push((start.0..old, start.1..new));
        }
    }
    runs
}
