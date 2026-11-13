//! The patch of one change of a Diff page, which `git apply --cached --unidiff-zero` takes on its standard input to
//! move that change into the index, or back out of it (`diff.change-patch`).

use std::fmt::Write as _;
use std::ops::{Range, RangeInclusive};

use super::{Change, LineDiffInput, Span, line_diff};

#[cfg(test)]
mod tests;

/// Which way a patch moves the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatchDirection {
    /// Towards the working tree: the lines added take the line endings of the index, as `git add` would store them.
    Stage,
    /// Towards `HEAD`: the lines put back keep their own line endings.
    Unstage,
}

/// Where the change is, by its lines counted from 0; with none, the empty range at the place meant, as
/// `model::diff::LineChange` has it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeLines {
    /// In the lines of the index: a change of the diff of the index against `HEAD`.
    Index(Range<usize>),
    /// In the lines of the other text: the working tree when staging, `HEAD` when unstaging a change of the diff of the
    /// working tree against `HEAD`.
    Other(Range<usize>),
}

/// What the patch of one change is made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePatchInput<'a> {
    /// The path of the file from the top of the repository.
    pub path: &'a str,
    /// The exact text of the index.
    pub index: &'a str,
    /// The text the index moves towards: the working tree to stage, `HEAD` to unstage.
    pub other: &'a str,
    /// Which way.
    pub direction: PatchDirection,
    /// Where the change is.
    pub change: ChangeLines,
}

/// The patch that moves one change into the index, or back out of it: the differences between the exact text of the
/// index and the other text that touch the lines of the change, each under a header with no line of context, its old
/// lines with `-` and its new lines with `+`, going forward from the index as it is. None when no difference touches
/// the change: it is staged already, or nothing of it is staged.
///
/// Feature: diff.change-patch
#[must_use]
pub fn change_patch(input: &ChangePatchInput<'_>) -> Option<String> {
    let index = exact_lines(input.index);
    let other = exact_lines(input.other);
    let (index_keys, other_keys) = (keys(&index), keys(&other));
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
    let here = match &input.change {
        ChangeLines::Index(lines) | ChangeLines::Other(lines) => place(lines),
    };
    let touching: Vec<&Change> = differences
        .iter()
        .filter(|difference| {
            let side = match input.change {
                ChangeLines::Index(_) => difference.left,
                ChangeLines::Other(_) => difference.right,
            };
            let there = place(&side.lines());
            there.start() <= here.end() && here.start() <= there.end()
        })
        .collect();
    if touching.is_empty() {
        return None;
    }
    let ending = if input.index.contains("\r\n") { "\r\n" } else { "\n" };
    let (old, new) = (named("a/", input.path), named("b/", input.path));
    let mut patch = format!("diff --git {old} {new}\n--- {old}\n+++ {new}\n");
    let mut offset: isize = 0;
    for difference in touching {
        let header = Header::of(difference, offset);
        offset += count_of(difference.right.count) - count_of(difference.left.count);
        let _ = writeln!(
            patch,
            "@@ -{},{} +{},{} @@",
            header.old.start, header.old.count, header.new.start, header.new.count
        );
        for line in index.get(difference.left.lines()).unwrap_or_default() {
            put(&mut patch, '-', line.raw, line.has_break);
        }
        for line in other.get(difference.right.lines()).unwrap_or_default() {
            match input.direction {
                PatchDirection::Stage if line.has_break => {
                    patch.push('+');
                    patch.push_str(line.content);
                    patch.push_str(ending);
                }
                PatchDirection::Stage => put(&mut patch, '+', line.content, false),
                PatchDirection::Unstage => put(&mut patch, '+', line.raw, line.has_break),
            }
        }
    }
    Some(patch)
}

/// The header of a difference in a patch: where it is in the index, and where it lands once the differences before it
/// in the patch are applied.
struct Header {
    /// Its lines in the index.
    old: Span,
    /// Its lines once applied.
    new: Span,
}

impl Header {
    /// The header of a difference after differences that added `offset` lines more than they removed.
    fn of(difference: &Change, offset: isize) -> Header {
        let old = difference.left;
        let before = if old.count > 0 { old.start - 1 } else { old.start };
        let new_before = usize::try_from(count_of(before) + offset).unwrap_or(0);
        let count = difference.right.count;
        Header {
            old,
            new: Span {
                start: if count > 0 { new_before + 1 } else { new_before },
                count,
            },
        }
    }
}

/// A count as a signed number, to add and take away.
fn count_of(count: usize) -> isize {
    isize::try_from(count).unwrap_or(isize::MAX)
}

/// A line of a patch: its sign and its text, then its line break, or `\ No newline at end of file` when it has none.
fn put(patch: &mut String, sign: char, text: &str, has_break: bool) {
    patch.push(sign);
    patch.push_str(text);
    patch.push('\n');
    if !has_break {
        patch.push_str("\\ No newline at end of file\n");
    }
}

/// A line of a text exactly as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExactLine<'a> {
    /// The line without its line break, and without the `\r` of a `\r\n`.
    content: &'a str,
    /// The line without its `\n`: with the `\r` of a `\r\n`.
    raw: &'a str,
    /// Whether a line break follows it; the last line of a text may have none.
    has_break: bool,
}

/// The lines of a text exactly as they are.
fn exact_lines(text: &str) -> Vec<ExactLine<'_>> {
    let mut lines = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let (raw, has_break, after) = match rest.find('\n') {
            Some(end) => (
                rest.get(..end).unwrap_or_default(),
                true,
                rest.get(end + 1..).unwrap_or_default(),
            ),
            None => (rest, false, ""),
        };
        let content = raw.strip_suffix('\r').unwrap_or(raw);
        lines.push(ExactLine {
            content,
            raw,
            has_break,
        });
        rest = after;
    }
    lines
}

/// What the lines are compared by: their text, a `\r\n` read as `\n`, and the last line with no line break told apart
/// from the same line with one, so that adding a line break is a difference. A NUL byte cannot be in a text the Diff
/// page shows, so it marks the line.
fn keys(lines: &[ExactLine<'_>]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            if line.has_break {
                line.content.to_owned()
            } else {
                format!("{}\0", line.content)
            }
        })
        .collect()
}

/// Where some lines are, in halves of a line: line `n` (from 1) is `2n`, and the place between lines `n` and `n + 1` is
/// `2n + 1`. Lines cover from the place before the first to the last; no line covers the place it sits at. As
/// `diff.staged-changes` places them.
fn place(lines: &Range<usize>) -> RangeInclusive<usize> {
    let before = 2 * lines.start + 1;
    if lines.is_empty() {
        before..=before
    } else {
        before..=2 * lines.end
    }
}

/// A path as a patch names it, after `a/` or `b/`: with `/`, and in double quotes with its special characters escaped
/// when it has any, as git writes it.
fn named(prefix: &str, path: &str) -> String {
    let path = format!("{prefix}{}", path.replace('\\', "/"));
    if !path.contains(|character: char| character == '"' || character.is_control()) {
        return path;
    }
    let mut quoted = String::from("\"");
    for character in path.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\t' => quoted.push_str("\\t"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            other if other.is_control() => {
                let _ = write!(quoted, "\\{:03o}", u32::from(other));
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}
