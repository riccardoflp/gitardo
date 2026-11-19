//! What the operations on the lines selected of a Diff page share (D78): where the lines of a side are in the index,
//! through the lines that do not change between the two (Q1302), and how many rows a patch changes.

use std::ops::Range;

use text::diff::{Change, LineDiffInput, line_diff, lines};

/// The lines of the index that some lines of another text are at: a line both have is at its line of the index, a line
/// changed in the index at the line it faces there, the n-th old line facing the n-th new one.
pub(super) fn placed_in_index(other: &str, index: &str, selected: &Range<usize>) -> Range<usize> {
    let (other_lines, index_lines) = (lines(other), lines(index));
    let always = || true;
    let Ok(differences) = line_diff(&LineDiffInput {
        left: &other_lines,
        right: &index_lines,
        keep_going: &always,
    }) else {
        return selected.clone();
    };
    let start = place_line(&differences, selected.start);
    let end = place_line(&differences, selected.end).max(start);
    start..end
}

/// The place in the index of a place among the lines of the other text.
fn place_line(differences: &[Change], place: usize) -> usize {
    let mut shift = 0_isize;
    for difference in differences {
        let (old, new) = (difference.left.lines(), difference.right.lines());
        if place < old.start {
            break;
        }
        if place < old.end {
            return new.start + (place - old.start).min(new.len());
        }
        shift = signed(new.end) - signed(old.end);
    }
    usize::try_from(signed(place) + shift).unwrap_or(0)
}

/// A count as a signed number.
fn signed(count: usize) -> isize {
    isize::try_from(count).unwrap_or(isize::MAX)
}

/// The smallest run of lines that holds two.
pub(super) fn union(one: &Range<usize>, other: &Range<usize>) -> Range<usize> {
    one.start.min(other.start)..one.end.max(other.end)
}

/// How many rows the hunks of a patch change: for each hunk, the more of the lines it removes and those it adds, the
/// rows `diff.align` lays a change out on.
pub(super) fn rows_changed(patch: &str) -> usize {
    patch
        .lines()
        .filter_map(|line| line.strip_prefix("@@ -"))
        .map(|header| {
            let counts: Vec<usize> = header
                .split([' ', ','])
                .filter_map(|part| part.trim_start_matches('+').parse().ok())
                .collect();
            match counts.as_slice() {
                [_, removed, _, added, ..] => (*removed).max(*added),
                _ => 0,
            }
        })
        .sum()
}

/// `1 line` or `<n> lines`.
pub(super) fn counted(count: usize) -> String {
    if count == 1 {
        String::from("1 line")
    } else {
        format!("{count} lines")
    }
}
