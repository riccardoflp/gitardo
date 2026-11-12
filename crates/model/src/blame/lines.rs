//! The lines of a file that shows its blame as it is scrolled and edited: the commit named on the first line in sight,
//! and the lines typed, which are not committed yet.

use super::BlameLine;

/// What [`pick_pinned_line`] looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinnedInput<'a> {
    /// The blame of each line; none for a line git said nothing of.
    pub lines: &'a [Option<BlameLine>],
    /// What is said beside each line (`blame.format-line`).
    pub texts: &'a [String],
    /// The first line in sight, from 0.
    pub first_in_sight: usize,
}

/// The line in sight to give the text of the first line of its run, when that first line is scrolled out of sight.
///
/// Feature: blame.pick-pinned-line
#[must_use]
pub fn pick_pinned_line(input: &PinnedInput<'_>) -> Option<(usize, String)> {
    let first = input.first_in_sight;
    if !input.texts.get(first)?.is_empty() {
        return None;
    }
    let Some(BlameLine::Committed(commit)) = input.lines.get(first)? else {
        return None;
    };
    let is_same = |index: usize| matches!(input.lines.get(index), Some(Some(BlameLine::Committed(other))) if other.id == commit.id);
    let mut start = first;
    while start > 0 && is_same(start - 1) {
        start -= 1;
    }
    let text = input.texts.get(start)?;
    (start < first && !text.is_empty()).then(|| (first, text.clone()))
}

/// An edit of the text: the range it replaced, from its start to its end, each a line and a column from 0, and the
/// text put in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEdit<'a> {
    /// Where the range starts.
    pub start: (usize, usize),
    /// Where it ends.
    pub end: (usize, usize),
    /// What was put in its place.
    pub text: &'a str,
}

/// The blame of each line once the edits are made, in their order: the lines they touch are not committed yet.
///
/// An edit from the start of a line to the start of a line, putting in nothing or whole lines, takes out the lines it
/// spans and puts in one line not committed for each line break; any other edit replaces the lines it touches with one
/// line not committed for each line of the text put in.
///
/// Feature: blame.mark-edited-lines
#[must_use]
pub fn mark_edited_lines(lines: &[Option<BlameLine>], edits: &[LineEdit<'_>]) -> Vec<Option<BlameLine>> {
    let mut lines = lines.to_vec();
    for edit in edits {
        let breaks = edit.text.matches('\n').count();
        let is_whole = edit.start.1 == 0 && edit.end.1 == 0 && (edit.text.is_empty() || edit.text.ends_with('\n'));
        let (from, to, put) = if is_whole {
            (edit.start.0, edit.end.0, breaks)
        } else {
            (edit.start.0, edit.end.0 + 1, breaks + 1)
        };
        let from = from.min(lines.len());
        let to = to.clamp(from, lines.len());
        let _replaced = lines.splice(from..to, std::iter::repeat_n(Some(BlameLine::NotCommitted), put));
    }
    lines
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::sync::Arc;

    use super::{LineEdit, PinnedInput, mark_edited_lines, pick_pinned_line};
    use crate::blame::{BlameCommit, BlameLine};
    use crate::{CommitId, Timestamp};

    fn commit(digit: char) -> BlameLine {
        BlameLine::Committed(Arc::new(BlameCommit {
            id: CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash")),
            author: String::from("Ann Lee"),
            author_time: Timestamp(0),
            summary: format!("Commit {digit}"),
            previous: None,
        }))
    }

    /// Lines 1 to 9 of commit `1`, 10 to 20 of commit `2`, 21 to 25 of commit `3`, grouped or not.
    fn file(group_lines: bool) -> (Vec<Option<BlameLine>>, Vec<String>) {
        let mut lines = Vec::new();
        let mut texts = Vec::new();
        for (digit, count) in [('1', 9), ('2', 11), ('3', 5)] {
            for index in 0..count {
                lines.push(Some(commit(digit)));
                let says = index == 0 || !group_lines;
                texts.push(if says { format!("Commit {digit}") } else { String::new() });
            }
        }
        (lines, texts)
    }

    fn pinned(group_lines: bool, line: usize) -> Option<(usize, String)> {
        let (lines, texts) = file(group_lines);
        pick_pinned_line(&PinnedInput {
            lines: &lines,
            texts: &texts,
            first_in_sight: line - 1,
        })
    }

    #[test]
    fn blame_pick_pinned_line__line_12_first_in_sight_shows_the_text_of_line_10() {
        assert_eq!(pinned(true, 12), Some((11, String::from("Commit 2"))));
    }

    #[test]
    fn blame_pick_pinned_line__line_10_first_in_sight_pins_nothing() {
        assert_eq!(pinned(true, 10), None);
    }

    #[test]
    fn blame_pick_pinned_line__with_grouping_off_nothing_is_pinned() {
        for line in [1, 12, 21, 25] {
            assert_eq!(pinned(false, line), None, "line {line}");
        }
    }

    /// Lines 1 to 5, of commits `1` to `5`.
    fn five() -> Vec<Option<BlameLine>> {
        ['1', '2', '3', '4', '5'].map(|digit| Some(commit(digit))).to_vec()
    }

    /// Each line as its commit's digit, or `-` when it is not committed.
    fn shown(lines: &[Option<BlameLine>]) -> String {
        lines
            .iter()
            .map(|line| match line {
                Some(BlameLine::Committed(commit)) => commit.summary.chars().last().unwrap_or('?'),
                _ => '-',
            })
            .collect()
    }

    #[test]
    fn blame_mark_edited_lines__the_examples_hold() {
        let typed = LineEdit {
            start: (2, 5),
            end: (2, 5),
            text: "x",
        };
        assert_eq!(shown(&mark_edited_lines(&five(), &[typed])), "12-45");
        let pasted = LineEdit {
            start: (1, 0),
            end: (1, 0),
            text: "first\nsecond\n",
        };
        assert_eq!(shown(&mark_edited_lines(&five(), &[pasted])), "1--2345");
        let deleted = LineEdit {
            start: (3, 0),
            end: (4, 0),
            text: "",
        };
        assert_eq!(shown(&mark_edited_lines(&five(), &[deleted])), "1235");
    }

    #[test]
    fn blame_mark_edited_lines__enter_in_the_middle_of_a_line_gives_two_lines_not_committed() {
        let enter = LineEdit {
            start: (2, 3),
            end: (2, 3),
            text: "\n",
        };
        assert_eq!(shown(&mark_edited_lines(&five(), &[enter])), "12--45");
    }
}
