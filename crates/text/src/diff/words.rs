//! The words that changed within a line paired with another (`diff.word-diff`).

use std::ops::Range;

use super::{myers, numbered, runs};

/// More than this part of the characters of both lines changed, and the whole line is the change.
const WHOLE_LINE_ABOVE: f64 = 0.65;

/// Two lines paired by the alignment of a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordDiffInput<'a> {
    /// The line on the left.
    pub left: &'a str,
    /// The line on the right.
    pub right: &'a str,
}

/// The parts of two paired lines that changed, as ranges of bytes of each line, each on whole characters: the tokens
/// removed from the left and added on the right, marks that touch or overlap made one. A token is a run of letters,
/// digits and `_` of any script, a run of spaces and tabs, or any other character alone; the two lists of tokens are
/// compared by a minimal diff (Myers 1986, as `diff.line-diff`). Nothing is marked when a line is empty, or when more
/// than 65% of the characters of both lines changed: the whole line is the change.
///
/// Feature: diff.word-diff
#[must_use]
pub fn word_diff(input: &WordDiffInput<'_>) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    if input.left.is_empty() || input.right.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let left = tokens(input.left);
    let right = tokens(input.right);
    let (left_numbers, right_numbers) = numbered(&words(input.left, &left), &words(input.right, &right));
    let always = || true;
    let Ok(marks) = myers::marks(&left_numbers, &right_numbers, &always) else {
        return (Vec::new(), Vec::new());
    };
    let mut left_marks = Vec::new();
    let mut right_marks = Vec::new();
    for (removed, added) in runs(&marks.removed, &marks.added) {
        push_mark(&mut left_marks, span(&left, &removed));
        push_mark(&mut right_marks, span(&right, &added));
    }
    if changed_part(input.left, &left_marks) > WHOLE_LINE_ABOVE
        && changed_part(input.right, &right_marks) > WHOLE_LINE_ABOVE
    {
        return (Vec::new(), Vec::new());
    }
    (left_marks, right_marks)
}

/// The text of each token of a line.
fn words<'line>(line: &'line str, tokens: &[Range<usize>]) -> Vec<&'line str> {
    tokens
        .iter()
        .map(|token| line.get(token.clone()).unwrap_or_default())
        .collect()
}

/// The tokens of a line, as ranges of its bytes.
fn tokens(line: &str) -> Vec<Range<usize>> {
    let mut tokens: Vec<Range<usize>> = Vec::new();
    let mut last_class = None;
    for (start, character) in line.char_indices() {
        let class = class_of(character);
        let end = start + character.len_utf8();
        match tokens.last_mut() {
            Some(token) if class != Class::Other && last_class == Some(class) => token.end = end,
            _ => tokens.push(start..end),
        }
        last_class = Some(class);
    }
    tokens
}

/// What a character is, for cutting a line into tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// A letter, a digit or `_`.
    Word,
    /// A space or a tab.
    Space,
    /// Anything else, a token on its own.
    Other,
}

/// The class of a character.
fn class_of(character: char) -> Class {
    if character.is_alphanumeric() || character == '_' {
        Class::Word
    } else if character == ' ' || character == '\t' {
        Class::Space
    } else {
        Class::Other
    }
}

/// The bytes of a run of tokens; none for a run with no token.
fn span(tokens: &[Range<usize>], run: &Range<usize>) -> Option<Range<usize>> {
    let first = tokens.get(run.start)?;
    let last = tokens.get(run.end.checked_sub(1)?)?;
    Some(first.start..last.end)
}

/// Adds a mark to a list, made one with the last when they touch or overlap.
fn push_mark(marks: &mut Vec<Range<usize>>, mark: Option<Range<usize>>) {
    let Some(mark) = mark else {
        return;
    };
    match marks.last_mut() {
        Some(last) if mark.start <= last.end => last.end = last.end.max(mark.end),
        _ => marks.push(mark),
    }
}

/// The part of the characters of a line that its marks cover, from 0 to 1.
fn changed_part(line: &str, marks: &[Range<usize>]) -> f64 {
    let total = line.chars().count();
    let marked: usize = marks
        .iter()
        .map(|mark| line.get(mark.clone()).map_or(0, |part| part.chars().count()))
        .sum();
    let to_float = |count: usize| f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    if total == 0 {
        0.0
    } else {
        to_float(marked) / to_float(total)
    }
}
