//! The history of a file, of some lines or of a folder: which lines of the file as committed the lines as they are now
//! come from, and what the timeline is narrowed to.

mod history;
mod line_diff;

use crate::CommitId;

pub use history::{CommittedLinesQuery, HistoryOf, LinesOfFile, selected_lines};
use line_diff::changed_runs;

/// Some lines of a file, counted from 1, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    /// The first line.
    pub start: usize,
    /// The last line, at least the first.
    pub end: usize,
}

/// What [`map_lines`] maps: the two texts of a file and the lines of the one as it is now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapLinesInput<'a> {
    /// The file as committed.
    pub committed: &'a str,
    /// The file as it is now, saved or not.
    pub current: &'a str,
    /// The lines of the file as it is now.
    pub lines: LineRange,
}

/// Where some lines of a file are in the commit checked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommittedLines {
    /// No commit is checked out, or the one checked out has no such file.
    InNoCommit,
    /// The file is in the commit checked out.
    Committed {
        /// The commit checked out.
        commit: CommitId,
        /// The name the file has in it, from the top of the working tree.
        path: String,
        /// The lines of the commit the lines asked about come from; none when all of them were added since.
        lines: Option<LineRange>,
    },
}

/// The lines of the file as committed that some lines of the file as it is now come from: an unchanged line stands for
/// itself, a line put in where others were taken out stands for all of those, and a line only added stands for
/// nothing. The result goes from the first line any of them stands for to the last; none when all were added since.
///
/// Feature: file-history.map-lines
#[must_use]
pub fn map_lines(input: &MapLinesInput<'_>) -> Option<LineRange> {
    let committed = lines(input.committed);
    let current = lines(input.current);
    let selected = input.lines.start.min(input.lines.end).saturating_sub(1)..input.lines.end.max(input.lines.start);
    let mut found: Option<LineRange> = None;
    let mut take = |first: usize, last: usize| {
        let range = found.get_or_insert(LineRange {
            start: first + 1,
            end: last + 1,
        });
        range.start = range.start.min(first + 1);
        range.end = range.end.max(last + 1);
    };
    // The lines before each run of changes are unchanged, so each stands for the same line of the committed text.
    let (mut committed_line, mut current_line) = (0, 0);
    let runs = changed_runs(&committed, &current);
    let after_last = line_diff::ChangedRun {
        old: committed.len()..committed.len(),
        new: current.len()..current.len(),
    };
    for run in runs.iter().chain(std::iter::once(&after_last)) {
        for offset in 0..run.new.start.saturating_sub(current_line) {
            if selected.contains(&(current_line + offset)) {
                take(committed_line + offset, committed_line + offset);
            }
        }
        let is_edit = !run.old.is_empty();
        if is_edit && run.new.clone().any(|line| selected.contains(&line)) {
            take(run.old.start, run.old.end.saturating_sub(1));
        }
        (committed_line, current_line) = (run.old.end, run.new.end);
    }
    found
}

/// The lines of a text, split on `\n` or `\r\n`; the empty piece after a final line break is not a line.
fn lines(text: &str) -> Vec<&str> {
    let text = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() {
        return Vec::new();
    }
    text.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{LineRange, MapLinesInput, map_lines};

    /// The text of the letters, one per line.
    fn text(letters: &str, line_break: &str) -> String {
        letters.split(' ').flat_map(|letter| [letter, line_break]).collect()
    }

    fn mapped(committed: &str, current: &str, start: usize, end: usize) -> Option<(usize, usize)> {
        let input = MapLinesInput {
            committed,
            current,
            lines: LineRange { start, end },
        };
        map_lines(&input).map(|range| (range.start, range.end))
    }

    /// The examples of the card, with lines ending in a line break of a kind.
    fn check_examples(line_break: &str) {
        let committed = text("a b c d e", line_break);
        let current = text("a B c x d e", line_break);
        assert_eq!(mapped(&committed, &current, 2, 2), Some((2, 2)));
        assert_eq!(mapped(&committed, &current, 4, 4), None);
        assert_eq!(mapped(&committed, &current, 3, 5), Some((3, 4)));
        assert_eq!(mapped(&committed, &current, 1, 6), Some((1, 5)));
    }

    #[test]
    fn file_history_map_lines__holds_the_examples_of_the_card() {
        check_examples("\n");
    }

    #[test]
    fn file_history_map_lines__gives_the_same_with_crlf_line_endings() {
        check_examples("\r\n");
        let committed = text("a b c d e", "\n");
        let current = text("a B c x d e", "\r\n");
        assert_eq!(mapped(&committed, &current, 3, 5), Some((3, 4)));
    }

    #[test]
    fn file_history_map_lines__gives_the_same_range_back_for_identical_texts() {
        let committed = text("a b c d e f g h i j", "\n");
        assert_eq!(mapped(&committed, &committed, 4, 9), Some((4, 9)));
        assert_eq!(mapped("a\nb", "a\nb", 2, 2), Some((2, 2)));
    }

    #[test]
    fn file_history_map_lines__maps_lines_moved_down_by_a_line_inserted_above() {
        let committed = text("a b c d e f g h i j", "\n");
        let current = text("new a b c d e f g h i j", "\n");
        assert_eq!(mapped(&committed, &current, 5, 10), Some((4, 9)));
        assert_eq!(mapped(&committed, &current, 1, 1), None);
        assert_eq!(mapped("", "a\n", 1, 1), None);
    }
}
