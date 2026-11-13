//! `diff.revert-change`: puts back in the working tree what the left side of a Diff page has in place of one change,
//! from the **⟲** button of its band in the gutter.

use std::ops::Range;

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Text, Undo};
use model::diff::{DiffChange, DiffSide};

use super::change::{CHANGED, is_where_shown, read_versions, side_text, target_change, versions};

/// Said when the right side of the page is not the working tree.
const NOT_OF_THE_WORKING_TREE: &str = "Only a change of the working tree can be reverted.";
/// Why Undo does not take it back.
const NOT_UNDONE: &str = "The file is written as the left side has it; Undo does not take it back.";

/// Reads the working tree and the left side again, checks that the change is where the page showed it, replaces its
/// lines in the working tree by those the left side has there, in the line endings of the file, and writes the file:
/// `write <file>`.
///
/// Feature: diff.revert-change
#[must_use]
pub fn revert_change(context: &Context, answers: &Answers) -> Outcome {
    let change = match target_change(context) {
        Ok(change) => change,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    if change.right != DiffSide::WorkingTree {
        return Outcome::Refuse(Refusal::saying(NOT_OF_THE_WORKING_TREE));
    }
    let commit = match &change.left {
        DiffSide::Head => context.repository.head.commit.as_ref(),
        DiffSide::Commit(commit) => Some(commit),
        DiffSide::Index | DiffSide::WorkingTree | DiffSide::Empty => None,
    };
    let Some(versions) = versions(answers) else {
        return read_versions(change, commit);
    };
    let Some(working_tree) = versions.working_tree.as_deref() else {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    };
    if !is_where_shown(change, versions) {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    }
    let left = side_text(&change.left, versions).unwrap_or_default();
    let contents = reverted(working_tree, left, change);
    Outcome::Run(Plan {
        title: Text::from(format!("Revert change in {}", change.path)),
        steps: vec![Step::Write {
            path: context.repository.folder.join(&change.path),
            contents,
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_UNDONE)),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// A line of a file as bytes: its text, and the line break after it, empty for a last line with none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Line<'a> {
    /// The text, without the `\r` of a `\r\n`.
    text: &'a [u8],
    /// `\n`, `\r\n`, or nothing.
    ending: &'a [u8],
}

/// The lines of a file as bytes.
fn lines_of(bytes: &[u8]) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(rest.len(), |end| end + 1);
        let (line, after) = rest.split_at(end);
        let text_end = if line.ends_with(b"\r\n") {
            line.len() - 2
        } else if line.ends_with(b"\n") {
            line.len() - 1
        } else {
            line.len()
        };
        let (text, ending) = line.split_at(text_end);
        lines.push(Line { text, ending });
        rest = after;
    }
    lines
}

/// The working tree with the lines of a change replaced by those of the left side, which take the line break of the
/// file (`\r\n` when it has one, `\n` otherwise). A change that ends both sides ends the file as the left side does:
/// with a line break or without.
fn reverted(working_tree: &[u8], left: &[u8], change: &DiffChange) -> Vec<u8> {
    let right_lines = lines_of(working_tree);
    let left_lines = lines_of(left);
    let ending: &[u8] = if working_tree.windows(2).any(|pair| pair == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    let mut result: Vec<Line<'_>> = Vec::new();
    result.extend(slice(&right_lines, 0..change.right_lines.start));
    result.extend(slice(&left_lines, change.left_lines.clone()).iter().map(|line| Line {
        text: line.text,
        ending,
    }));
    result.extend(slice(&right_lines, change.right_lines.end..right_lines.len()));
    let ends_both = change.right_lines.end >= right_lines.len() && change.left_lines.end >= left_lines.len();
    let left_has_break = left_lines.last().is_none_or(|line| !line.ending.is_empty());
    let count = result.len();
    for line in result.iter_mut().take(count.saturating_sub(1)) {
        if line.ending.is_empty() {
            line.ending = ending;
        }
    }
    if let (Some(last), true) = (result.last_mut(), ends_both) {
        if !left_has_break {
            last.ending = b"";
        } else if last.ending.is_empty() {
            last.ending = ending;
        }
    }
    result
        .iter()
        .flat_map(|line| [line.text, line.ending])
        .flatten()
        .copied()
        .collect()
}

/// Some lines, none past the end.
fn slice<'a, 'line>(lines: &'a [Line<'line>], range: Range<usize>) -> &'a [Line<'line>] {
    let end = range.end.min(lines.len());
    lines.get(range.start.min(end)..end).unwrap_or_default()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
