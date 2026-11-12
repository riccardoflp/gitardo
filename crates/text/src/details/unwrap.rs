//! The lines of a message that were cut to fit 72 columns, joined again so that the text wraps at the width of the place
//! that shows it.

/// The widest a message is taken to be wrapped at.
const WRAP_WIDTH: usize = 72;
/// A message whose longest line of prose is shorter than this was not wrapped.
const UNWRAPPED_BELOW: usize = 50;
/// A line that is one word of this many characters or more is never joined.
const LONG_WORD: usize = 30;

/// What a line of a message is, for joining.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineKind {
    /// An empty line.
    Empty,
    /// A fence of a code block, or a line inside one.
    Fenced,
    /// A trailer of the last paragraph: `Signed-off-by: …`.
    Trailer,
    /// A quotation, a heading, a single address, or one long word: laid out as written.
    Kept,
    /// The first line of an item of a list.
    Item,
    /// An indented line.
    Indented,
    /// A line of prose.
    Prose,
}

/// The body of a message with the lines that were cut to fit joined again by a space: a line is joined to the line
/// before when the line before is prose that could go on and the first word of this line would have taken it past the width the
/// message was wrapped at. Empty lines, fenced blocks, trailers, quotations, headings, addresses, long words, items of
/// lists and indented lines are never joined to the line before, but for the lines that go on after an item of a list;
/// a message whose longest line of prose is shorter than 50 characters is returned as it is. The indentation of a line
/// joined becomes part of the one space that replaces its line break.
///
/// Feature: details.message-unwrap
#[must_use]
pub fn unwrap_message(input: &str) -> String {
    let lines: Vec<&str> = input.split('\n').collect();
    let kinds = kinds(&lines);
    let width = lines
        .iter()
        .zip(&kinds)
        .filter(|(line, kind)| matches!(kind, LineKind::Prose | LineKind::Item | LineKind::Indented) && is_prose(line))
        .map(|(line, _)| length(line))
        .max()
        .unwrap_or(0)
        .min(WRAP_WIDTH);
    if width < UNWRAPPED_BELOW {
        return input.to_owned();
    }
    let mut unwrapped = String::with_capacity(input.len());
    let mut is_in_item = false;
    let mut previous: Option<(&str, LineKind)> = None;
    for (line, kind) in lines.iter().zip(&kinds) {
        let joins = previous.is_some_and(|before| joins(before, (line, *kind), is_in_item, width));
        is_in_item = match kind {
            LineKind::Item => true,
            LineKind::Indented | LineKind::Prose => is_in_item && (joins || *kind == LineKind::Indented),
            _ => false,
        };
        if joins {
            unwrapped.push(' ');
            unwrapped.push_str(line.trim_start());
        } else {
            if previous.is_some() {
                unwrapped.push('\n');
            }
            unwrapped.push_str(line);
        }
        previous = Some((line, *kind));
    }
    unwrapped
}

/// Whether a line is joined to the line before it.
fn joins(
    (before, before_kind): (&str, LineKind),
    (line, kind): (&str, LineKind),
    is_in_item: bool,
    width: usize,
) -> bool {
    let before_takes_more = matches!(before_kind, LineKind::Prose | LineKind::Item | LineKind::Indented);
    let goes_on = match kind {
        LineKind::Prose => true,
        LineKind::Indented => is_in_item,
        _ => false,
    };
    if !before_takes_more || !goes_on || (before_kind == LineKind::Indented && !is_in_item) {
        return false;
    }
    let first_word = line.split_whitespace().next().map_or(0, length);
    let taken = length(before);
    taken + 1 + first_word > width
}

/// What each line is: the fences and what is between them, the trailers of the last paragraph, and the rest.
fn kinds(lines: &[&str]) -> Vec<LineKind> {
    let mut is_fenced = false;
    let mut kinds: Vec<LineKind> = lines
        .iter()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                is_fenced = !is_fenced;
                return LineKind::Fenced;
            }
            if is_fenced {
                return LineKind::Fenced;
            }
            kind(line)
        })
        .collect();
    let last_paragraph = kinds
        .iter()
        .rposition(|kind| *kind == LineKind::Empty)
        .map_or(0, |empty| empty + 1);
    let is_trailers = lines
        .get(last_paragraph..)
        .is_some_and(|paragraph| !paragraph.is_empty() && paragraph.iter().all(|line| is_trailer(line)))
        && kinds
            .get(last_paragraph..)
            .is_some_and(|paragraph| !paragraph.contains(&LineKind::Fenced));
    if is_trailers && last_paragraph > 0 {
        for kind in kinds.iter_mut().skip(last_paragraph) {
            *kind = LineKind::Trailer;
        }
    }
    kinds
}

/// What a line outside a fenced block is.
fn kind(line: &str) -> LineKind {
    let trimmed = line.trim_start();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    match words.as_slice() {
        [] => LineKind::Empty,
        _ if trimmed.starts_with('>') || trimmed.starts_with('#') => LineKind::Kept,
        [word] if word.contains("://") || length(word) >= LONG_WORD => LineKind::Kept,
        _ if is_item(trimmed) => LineKind::Item,
        _ if trimmed.len() < line.len() => LineKind::Indented,
        _ => LineKind::Prose,
    }
}

/// Whether a line, without its indentation, starts an item of a list: `-`, `*`, `+`, `•`, or a number followed by `.`
/// or `)`, then a space.
fn is_item(trimmed: &str) -> bool {
    let mut characters = trimmed.chars();
    match characters.next() {
        Some('-' | '*' | '+' | '•') => characters.next().is_some_and(char::is_whitespace),
        Some(digit) if digit.is_ascii_digit() => {
            let rest = trimmed.trim_start_matches(|character: char| character.is_ascii_digit());
            let mut after = rest.chars();
            matches!(after.next(), Some('.' | ')')) && after.next().is_some_and(char::is_whitespace)
        }
        _ => false,
    }
}

/// Whether a line is a trailer: `Key: value`, with a key of letters, digits and dashes.
fn is_trailer(line: &str) -> bool {
    line.split_once(": ").is_some_and(|(key, value)| {
        !key.is_empty()
            && !value.trim().is_empty()
            && key
                .chars()
                .all(|character| character.is_alphanumeric() || character == '-')
    })
}

/// Whether a line is prose: at least two words.
fn is_prose(line: &str) -> bool {
    line.split_whitespace().nth(1).is_some()
}

/// The length of a line, in characters.
fn length(line: &str) -> usize {
    line.chars().count()
}
