//! How a file of JSON is laid out: its lines, their indentation, and the white space and comments around values, so
//! that an edit can write in the style the file already has.

use std::ops::Range;

use crate::parse::{Entry, Node, NodeKind};

/// The indentation used when a file has none to copy: four spaces, as the pretty text writes.
pub(crate) const DEFAULT_UNIT: &str = "    ";

/// The habits of a file that a new line written into it follows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Conventions {
    /// What ends its lines: a carriage return and a line feed when the first line ends so, a line feed otherwise.
    pub(crate) newline: &'static str,
    /// The indentation of one level: the spaces or tab a member is indented by more than its object.
    pub(crate) unit: String,
    /// Whether the whole value of the file is on one line.
    pub(crate) shape: Shape,
}

/// Whether something is written on one line or on several.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// On a single line.
    OneLine,
    /// Over several lines.
    SeveralLines,
}

impl Conventions {
    /// The habits of a text, whose value is `root`.
    pub(crate) fn of(text: &str, root: &Node) -> Conventions {
        Conventions {
            newline: newline_of(text),
            unit: unit_of(text, root).unwrap_or_else(|| DEFAULT_UNIT.to_string()),
            shape: shape_of(text, root.start..root.end),
        }
    }
}

/// What ends the lines of a text: a carriage return and a line feed when its first line ends so, a line feed
/// otherwise.
pub(crate) fn newline_of(text: &str) -> &'static str {
    match text.find('\n') {
        Some(offset) if offset > 0 && text.as_bytes().get(offset - 1) == Some(&b'\r') => "\r\n",
        _ => "\n",
    }
}

/// Whether some bytes of a text are on one line.
pub(crate) fn shape_of(text: &str, bytes: Range<usize>) -> Shape {
    let bytes = text.as_bytes().get(bytes).unwrap_or_default();
    if bytes.contains(&b'\n') {
        Shape::SeveralLines
    } else {
        Shape::OneLine
    }
}

/// The indentation of one level in a text: what the first entry that starts a line is indented by more than the line
/// its array or object opens on.
fn unit_of(text: &str, node: &Node) -> Option<String> {
    let entries = match &node.kind {
        NodeKind::Array(entries) | NodeKind::Object(entries) => entries,
        NodeKind::Scalar(_) => return None,
    };
    entries
        .iter()
        .find_map(|entry| unit_of_entry(text, node, entry).or_else(|| unit_of(text, &entry.value)))
}

/// What an entry is indented by more than the line its container opens on, when the entry starts a line.
fn unit_of_entry(text: &str, container: &Node, entry: &Entry) -> Option<String> {
    if !is_at_line_start(text, entry.start) {
        return None;
    }
    let outer = line_indent(text, container.start);
    let inner = line_indent(text, entry.start);
    inner
        .strip_prefix(outer)
        .filter(|unit| !unit.is_empty())
        .map(str::to_string)
}

/// The offset of the start of the line an offset is on.
pub(crate) fn line_start(text: &str, offset: usize) -> usize {
    let before = text.as_bytes().get(..offset).unwrap_or_default();
    before
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |newline| newline + 1)
}

/// The spaces and tabs at the start of the line an offset is on.
pub(crate) fn line_indent(text: &str, offset: usize) -> &str {
    let start = line_start(text, offset);
    let rest = text.get(start..).unwrap_or_default();
    let length = rest.bytes().take_while(|byte| is_space(*byte)).count();
    rest.get(..length).unwrap_or_default()
}

/// Whether only spaces and tabs come before an offset on its line.
pub(crate) fn is_at_line_start(text: &str, offset: usize) -> bool {
    let start = line_start(text, offset);
    text.as_bytes()
        .get(start..offset)
        .is_some_and(|before| before.iter().all(|byte| is_space(*byte)))
}

/// The length of the line ending at an offset: 2 for a carriage return and a line feed, 1 for a line feed, 0 when
/// no line ends there.
pub(crate) fn newline_length_at(text: &str, offset: usize) -> usize {
    let rest = text.get(offset..).unwrap_or_default();
    if rest.starts_with("\r\n") {
        2
    } else {
        usize::from(rest.starts_with('\n'))
    }
}

/// The offset after the spaces and tabs that start at an offset.
pub(crate) fn skip_spaces(text: &str, offset: usize) -> usize {
    let rest = text.as_bytes().get(offset..).unwrap_or_default();
    offset + rest.iter().take_while(|byte| is_space(**byte)).count()
}

/// The offset before the spaces and tabs at the end of some bytes of a text, going back no further than their start.
pub(crate) fn skip_spaces_back(text: &str, bytes: Range<usize>) -> usize {
    let end = bytes.end;
    let before = text.as_bytes().get(bytes).unwrap_or_default();
    end - before.iter().rev().take_while(|byte| is_space(**byte)).count()
}

/// The offset after the spaces, tabs and comments that start at an offset and stay on its line: where its line ends,
/// or where something else starts on it. A block comment that starts on the line counts whole, even over several
/// lines.
pub(crate) fn same_line_trivia_end(text: &str, offset: usize) -> usize {
    let mut position = skip_spaces(text, offset);
    loop {
        let rest = text.get(position..).unwrap_or_default();
        if rest.starts_with("//") {
            return position + rest.find(['\r', '\n']).unwrap_or(rest.len());
        }
        if !rest.starts_with("/*") {
            return position;
        }
        let length = rest
            .get(2..)
            .and_then(|inside| inside.find("*/"))
            .map_or(rest.len(), |end| end + 4);
        position = skip_spaces(text, position + length);
    }
}

/// Whether some bytes of a text are only white space.
pub(crate) fn is_white_space(text: &str, bytes: Range<usize>) -> bool {
    text.get(bytes)
        .is_some_and(|between| between.bytes().all(crate::parse::is_white_space))
}

fn is_space(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{Dialect, parse_document};

    #[test]
    fn the_indentation_unit_is_what_the_first_entry_on_its_own_line_adds() {
        let cases = [
            ("{\n  \"a\": 1\n}", "  "),
            ("{\n\t\"a\": 1\n}", "\t"),
            ("{ \"a\": {\n      \"b\": 1 } }", "      "),
            ("[1, 2, 3]", DEFAULT_UNIT),
            ("{\n}", DEFAULT_UNIT),
            ("// head\n{\n   \"a\": [\n      1\n   ]\n}", "   "),
            ("{\"a\": 1,\n\"b\": 2}", DEFAULT_UNIT),
        ];
        for (text, unit) in cases {
            let root = parse_document(text, Dialect::WithComments).unwrap();
            assert_eq!(Conventions::of(text, &root).unit, unit, "{text:?}");
        }
    }

    #[test]
    fn the_newline_is_the_one_of_the_first_line() {
        let cases = [("{}", "\n"), ("{\r\n}", "\r\n"), ("{\n}\r\n", "\n"), ("\r\n{}", "\r\n")];
        for (text, newline) in cases {
            assert_eq!(newline_of(text), newline, "{text:?}");
        }
    }

    #[test]
    fn trivia_on_the_same_line_ends_at_the_line_or_at_what_follows() {
        let cases = [
            ("1, // note\n2", 2, 10),
            ("1 /* a */ /* b */ }", 1, 18),
            ("1 /* a\n b */ \r\n", 1, 13),
            ("1   ", 1, 4),
            ("1  2", 1, 3),
        ];
        for (text, offset, end) in cases {
            assert_eq!(same_line_trivia_end(text, offset), end, "{text:?}");
        }
    }

    #[test]
    fn lines_and_indentation_are_found_from_any_offset() {
        let text = "{\n\t  \"a\": 1,\r\n  \"b\": 2 }";
        assert_eq!(line_start(text, 6), 2);
        assert_eq!(line_indent(text, 6), "\t  ");
        assert!(is_at_line_start(text, 5));
        assert!(!is_at_line_start(text, 6));
        assert_eq!(newline_length_at(text, 12), 2);
        assert_eq!(newline_length_at(text, 0), 0);
        assert_eq!(skip_spaces_back(text, 2..5), 2);
        assert_eq!(shape_of(text, 0..2), Shape::SeveralLines);
        assert_eq!(shape_of(text, 0..1), Shape::OneLine);
    }
}
