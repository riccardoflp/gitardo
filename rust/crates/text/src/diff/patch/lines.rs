//! The patch of the lines selected of a Diff page (`diff.lines-patch`, D78): of the differences between the text of the
//! index and the other text, only the lines whose rows are selected, each run of them a hunk with no line of context,
//! for `git apply --cached --unidiff-zero` on its standard input.

use std::fmt::Write as _;
use std::ops::Range;

use super::{ExactLine, PatchDirection, count_of, exact_lines, keys, named, put};
use crate::diff::{Change, LineDiffInput, line_diff};

#[cfg(test)]
mod tests;

/// What the patch of the lines selected is made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinesPatchInput<'a> {
    /// The path of the file from the top of the repository.
    pub path: &'a str,
    /// The exact text the patch applies to: the index.
    pub index: &'a str,
    /// The text lines are taken from: the working tree to stage, `HEAD` to unstage.
    pub other: &'a str,
    /// Which way.
    pub direction: PatchDirection,
    /// The lines of the index whose rows are selected, counted from 0.
    pub index_lines: Range<usize>,
    /// The lines of the other text whose rows are selected, counted from 0.
    pub other_lines: Range<usize>,
    /// For a file the index does not have, the mode it is created with: `100644`, or `100755` for one git sees as
    /// executable.
    pub new_file: Option<&'a str>,
}

/// One line of a difference, in the order the patch has them: a line of the index, removed or kept, or a line of the
/// other text, added or left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// A line of the index, by its number from 0, and whether it is removed.
    Old(usize, bool),
    /// A line of the other text, by its number from 0, whether it is added, and how many lines of the index are before
    /// the place it goes.
    New(usize, bool, usize),
    /// The end of a difference: the lines that do not change after it end a run.
    End,
}

/// The patch that takes into the index only the lines selected of the differences between it and the other text: a line
/// the other text adds when its row is selected, a line it removes when its row is selected, the others left as they
/// are; each run of lines taken under a header with no line of context. A line taken after a last line with no line
/// break takes the change of that line with it. None when no line selected differs.
///
/// Feature: diff.lines-patch
#[must_use]
pub fn lines_patch(input: &LinesPatchInput<'_>) -> Option<String> {
    let index = exact_lines(input.index);
    let other = exact_lines(input.other);
    let steps = steps_taken(input, &index, &other)?;
    let hunks = hunks_of(&steps);
    if hunks.is_empty() {
        return None;
    }
    let mut patch = heading(input);
    let ending = if input.index.contains("\r\n") { "\r\n" } else { "\n" };
    let mut offset: isize = 0;
    for hunk in &hunks {
        let removed = hunk.iter().filter(|step| matches!(step, Step::Old(..))).count();
        let added = hunk.len() - removed;
        let old_start = match hunk.first() {
            Some(Step::Old(line, _)) => line + 1,
            Some(Step::New(_, _, place)) => *place,
            _ => 0,
        };
        let before = if removed > 0 { old_start - 1 } else { old_start };
        let new_before = usize::try_from(count_of(before) + offset).unwrap_or(0);
        let new_start = if added > 0 { new_before + 1 } else { new_before };
        offset += count_of(added) - count_of(removed);
        let _ = writeln!(patch, "@@ -{old_start},{removed} +{new_start},{added} @@");
        for step in hunk {
            match *step {
                Step::Old(line, _) => {
                    if let Some(line) = index.get(line) {
                        put(&mut patch, '-', line.raw, line.has_break);
                    }
                }
                Step::New(line, _, _) => {
                    if let Some(line) = other.get(line) {
                        added_line(&mut patch, line, (input.direction, ending));
                    }
                }
                Step::End => {}
            }
        }
    }
    Some(patch)
}

/// The index with only the lines selected of the differences taken from the other text, as `git apply` of the patch of
/// `lines_patch` would leave it: the lines of the index kept in their line endings, the lines taken in those of the
/// index. The window reverts lines with it, the working tree as the index and the left side as the other text. None
/// when no line selected differs.
#[must_use]
pub fn lines_taken(input: &LinesPatchInput<'_>) -> Option<String> {
    let index = exact_lines(input.index);
    let other = exact_lines(input.other);
    let steps = steps_taken(input, &index, &other)?;
    if !steps
        .iter()
        .any(|step| matches!(step, Step::Old(_, true) | Step::New(_, true, _)))
    {
        return None;
    }
    let ending = if input.index.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<(&str, bool)> = Vec::new();
    let mut next = 0;
    for step in &steps {
        match *step {
            Step::Old(line, taken) => {
                keep_until(&index, (&mut lines, &mut next), line);
                if let (false, Some(kept)) = (taken, index.get(line)) {
                    lines.push((kept.raw, kept.has_break));
                }
                next = line + 1;
            }
            Step::New(line, true, place) => {
                keep_until(&index, (&mut lines, &mut next), place);
                if let Some(added) = other.get(line) {
                    lines.push((added.content, added.has_break));
                }
            }
            Step::New(_, false, _) | Step::End => {}
        }
    }
    keep_until(&index, (&mut lines, &mut next), index.len());
    let count = lines.len();
    let mut text = String::new();
    for (place, (line, has_break)) in lines.into_iter().enumerate() {
        text.push_str(line);
        if has_break || place + 1 < count {
            // A line of the index keeps its `\r` in its text; a line taken gets the ending of the index.
            text.push_str(if line.ends_with('\r') { "\n" } else { ending });
        }
    }
    Some(text)
}

/// The lines of the index from the next one up to a line, kept as they are.
fn keep_until<'a>(index: &[ExactLine<'a>], (lines, next): (&mut Vec<(&'a str, bool)>, &mut usize), end: usize) {
    for line in index.get(*next..end.max(*next)).unwrap_or_default() {
        lines.push((line.raw, line.has_break));
    }
    *next = end.max(*next);
}

/// The steps of every difference between the index and the other text, each line marked taken when its row is
/// selected; none when the differences cannot be found.
fn steps_taken(input: &LinesPatchInput<'_>, index: &[ExactLine<'_>], other: &[ExactLine<'_>]) -> Option<Vec<Step>> {
    let (index_keys, other_keys) = (keys(index), keys(other));
    let (index_keys, other_keys): (Vec<&str>, Vec<&str>) = (
        index_keys.iter().map(String::as_str).collect(),
        other_keys.iter().map(String::as_str).collect(),
    );
    let always = || true;
    let differences = line_diff(&LineDiffInput {
        left: &index_keys,
        right: &other_keys,
        keep_going: &always,
    })
    .ok()?;
    Some(
        differences
            .iter()
            .flat_map(|difference| {
                let mut steps = steps_of(difference, input, index);
                steps.push(Step::End);
                steps
            })
            .collect(),
    )
}

/// The names of the file at the top of a patch, and for a file the index does not have, its mode and `/dev/null`.
fn heading(input: &LinesPatchInput<'_>) -> String {
    let (old, new) = (named("a/", input.path), named("b/", input.path));
    match input.new_file {
        Some(mode) => format!("diff --git {old} {new}\nnew file mode {mode}\n--- /dev/null\n+++ {new}\n"),
        None => format!("diff --git {old} {new}\n--- {old}\n+++ {new}\n"),
    }
}

/// A line added: when staging, with the line endings of the index, as `git add` would store it.
fn added_line(patch: &mut String, line: &ExactLine<'_>, (direction, ending): (PatchDirection, &str)) {
    match direction {
        PatchDirection::Stage if line.has_break => {
            patch.push('+');
            patch.push_str(line.content);
            patch.push_str(ending);
        }
        PatchDirection::Stage => put(patch, '+', line.content, false),
        PatchDirection::Unstage => put(patch, '+', line.raw, line.has_break),
    }
}

/// The lines of a difference in the rows `diff.align` lays them out on, the n-th old line facing the n-th new one, each
/// marked taken when its row is selected; a line added after a last line with no line break takes that line's change.
fn steps_of(difference: &Change, input: &LinesPatchInput<'_>, index: &[ExactLine<'_>]) -> Vec<Step> {
    let (old, new) = (difference.left.lines(), difference.right.lines());
    let selected = |range: &Range<usize>, line: usize| range.contains(&line);
    let mut old_taken: Vec<bool> = old.clone().map(|line| selected(&input.index_lines, line)).collect();
    let mut new_taken: Vec<bool> = new.clone().map(|line| selected(&input.other_lines, line)).collect();
    // The old line with no line break after it is the last of the index: the lines put after it need one on it.
    let unbroken = old
        .clone()
        .position(|line| index.get(line).is_some_and(|line| !line.has_break));
    if let Some(at) = unbroken
        && new_taken.iter().skip(at + 1).any(|taken| *taken)
    {
        if let Some(taken) = old_taken.get_mut(at) {
            *taken = true;
        }
        if let Some(taken) = new_taken.get_mut(at) {
            *taken = true;
        }
    }
    let rows = old.len().max(new.len());
    let mut steps = Vec::with_capacity(old.len() + new.len());
    for row in 0..rows {
        if let Some(taken) = old_taken.get(row) {
            steps.push(Step::Old(old.start + row, *taken));
        }
        if let Some(taken) = new_taken.get(row) {
            // It goes after the old line of its row, or after the last old line when its row has none.
            let place = old.start + (row + 1).min(old.len());
            steps.push(Step::New(new.start + row, *taken, place));
        }
    }
    steps
}

/// The runs of lines taken: a line of the index kept, or the end of a difference, ends one; a line of the other text
/// left out does not.
fn hunks_of(steps: &[Step]) -> Vec<Vec<Step>> {
    let mut hunks = Vec::new();
    let mut current: Vec<Step> = Vec::new();
    for step in steps {
        match *step {
            Step::Old(_, true) | Step::New(_, true, _) => current.push(*step),
            Step::Old(_, false) | Step::End => {
                if !current.is_empty() {
                    hunks.push(std::mem::take(&mut current));
                }
            }
            Step::New(_, false, _) => {}
        }
    }
    hunks
}
