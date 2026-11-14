//! The conflicts git marked in a file, in any of its styles (`merge`, `diff3`, `zdiff3`), whatever names git wrote
//! after the markers.

use std::ops::Range;

/// A line of a file: its text, and what ends it (`\n`, `\r\n`, `\r`, or nothing for a last line without one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line<'a> {
    /// The text, without what ends it.
    pub text: &'a str,
    /// What ends it.
    pub ending: &'a str,
}

/// A conflict as git marked it, by the places of its lines among the lines of the file (from 0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkedConflict {
    /// The line of its first marker, from 1.
    pub line: usize,
    /// Its lines, from its first marker to the one after its last.
    pub span: Range<usize>,
    /// The current side's lines.
    pub current: Range<usize>,
    /// The base's lines, when git wrote them (the `diff3` and `zdiff3` styles).
    pub base: Option<Range<usize>>,
    /// The incoming side's lines.
    pub incoming: Range<usize>,
    /// Whether it has markers of its own inside it: such a conflict is never resolved.
    pub is_broken: bool,
}

/// The lines of a text, each with what ends it.
#[must_use]
pub fn split_lines(text: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let end = rest.find(['\n', '\r']).unwrap_or(rest.len());
        let (line, after) = rest.split_at(end);
        let ending_length = if after.starts_with("\r\n") {
            2
        } else {
            usize::from(!after.is_empty())
        };
        let (ending, after) = after.split_at(ending_length);
        lines.push(Line { text: line, ending });
        rest = after;
    }
    lines
}

/// What a line is to the parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Marker {
    /// `<<<<<<<`.
    Open,
    /// `|||||||`.
    Base,
    /// `=======`, alone on its line.
    Separator,
    /// `>>>>>>>`.
    Close,
}

/// The marker a line is: 7 signs at its start followed by its end or a space; the separator exactly.
fn marker(line: &str) -> Option<Marker> {
    if line == "=======" {
        return Some(Marker::Separator);
    }
    [('<', Marker::Open), ('|', Marker::Base), ('>', Marker::Close)]
        .into_iter()
        .find_map(|(sign, marker)| {
            let after = line.strip_prefix(&sign.to_string().repeat(7))?;
            (after.is_empty() || after.starts_with(' ')).then_some(marker)
        })
}

/// Where the parser is inside a conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    /// Between `<<<<<<<` and `|||||||` or `=======`.
    Current,
    /// Between `|||||||` and `=======`.
    Base,
    /// Between `=======` and `>>>>>>>`.
    Incoming,
}

/// A conflict being read.
struct Open {
    /// The place of its first marker.
    start: usize,
    /// The part being read.
    part: Part,
    /// The place of `|||||||`, when there is one.
    base_marker: Option<usize>,
    /// The place of `=======`, once read.
    separator: Option<usize>,
    /// How many conflicts opened inside it are not closed yet.
    depth: usize,
    /// Whether it has markers of its own inside it.
    is_broken: bool,
}

/// The conflicts git marked in a text; none when the file ends inside a conflict, and the markers do not pair up.
///
/// `<<<<<<<` opens a conflict, `|||||||` while in the current side starts the base, `=======` starts the incoming
/// side, and `>>>>>>>` while in it closes the conflict. A conflict with another `<<<<<<<` before it closes, or with a
/// second `=======`, is broken; it closes at the `>>>>>>>` that pairs with its own `<<<<<<<`.
///
/// Feature: conflicts.parse-markers
#[must_use]
pub fn parse_markers(text: &str) -> Option<Vec<MarkedConflict>> {
    let mut conflicts = Vec::new();
    let mut open: Option<Open> = None;
    for (index, line) in split_lines(text).iter().enumerate() {
        let found = marker(line.text);
        match (&mut open, found) {
            (None, Some(Marker::Open)) => {
                open = Some(Open {
                    start: index,
                    part: Part::Current,
                    base_marker: None,
                    separator: None,
                    depth: 0,
                    is_broken: false,
                });
            }
            (None, _) | (Some(_), None) => {}
            (Some(reading), Some(found)) => {
                if read_marker(reading, index, found) {
                    conflicts.extend(open.take().map(|closed| closed_conflict(&closed, index)));
                }
            }
        }
    }
    open.is_none().then_some(conflicts)
}

/// Reads a marker inside a conflict; whether it closes the conflict.
fn read_marker(reading: &mut Open, index: usize, found: Marker) -> bool {
    match (found, reading.part) {
        (Marker::Open, _) => {
            reading.is_broken = true;
            reading.depth += 1;
        }
        (Marker::Close, _) if reading.depth > 0 => reading.depth -= 1,
        _ if reading.depth > 0 => {}
        (Marker::Base, Part::Current) => {
            reading.part = Part::Base;
            reading.base_marker = Some(index);
        }
        (Marker::Separator, Part::Current | Part::Base) => {
            reading.part = Part::Incoming;
            reading.separator = Some(index);
        }
        (Marker::Close, Part::Incoming) => return true,
        (Marker::Close, Part::Current | Part::Base) => {
            reading.is_broken = true;
            return true;
        }
        (Marker::Base | Marker::Separator, _) => reading.is_broken = true,
    }
    false
}

/// The conflict read, closed at a line.
fn closed_conflict(reading: &Open, close: usize) -> MarkedConflict {
    let separator = reading.separator.unwrap_or(close);
    let current_end = reading.base_marker.unwrap_or(separator);
    MarkedConflict {
        line: reading.start + 1,
        span: reading.start..close + 1,
        current: reading.start + 1..current_end,
        base: reading.base_marker.map(|base| base + 1..separator),
        incoming: (separator + 1).min(close)..close,
        is_broken: reading.is_broken,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Line, MarkedConflict, parse_markers, split_lines};

    /// The texts of some lines of a text.
    fn texts(text: &str, lines: &std::ops::Range<usize>) -> Vec<String> {
        let all = split_lines(text);
        all.get(lines.clone())
            .unwrap_or_default()
            .iter()
            .map(|line| line.text.to_owned())
            .collect()
    }

    #[test]
    fn conflicts_parse_markers__one_conflict_with_its_two_sides() {
        let text = "a\n<<<<<<< HEAD\nB\n=======\nC\n>>>>>>> f\n";
        let found = parse_markers(text).unwrap_or_default();
        assert_eq!(
            found,
            [MarkedConflict {
                line: 2,
                span: 1..6,
                current: 2..3,
                base: None,
                incoming: 4..5,
                is_broken: false,
            }]
        );
        assert_eq!(texts(text, &found[0].current), ["B"]);
        assert_eq!(texts(text, &found[0].incoming), ["C"]);
    }

    #[test]
    fn conflicts_parse_markers__the_base_of_the_diff3_style() {
        let text = "<<<<<<< HEAD\nB\n||||||| 234cee0\nb\n=======\nC\n>>>>>>> f\n";
        let found = parse_markers(text).unwrap_or_default();
        let base = found
            .first()
            .and_then(|conflict| conflict.base.clone())
            .unwrap_or_default();
        assert_eq!(texts(text, &base), ["b"]);
        assert_eq!(texts(text, &found[0].current), ["B"]);
    }

    #[test]
    fn conflicts_parse_markers__eight_signs_or_a_letter_after_seven_is_not_a_marker() {
        for line in ["<<<<<<<<", "<<<<<<<x"] {
            let text = format!("{line}\nB\n=======\nC\n>>>>>>>\n");
            assert_eq!(parse_markers(&text), Some(Vec::new()), "{line}");
        }
        assert_eq!(
            parse_markers("<<<<<<<\nB\n=======\nC\n>>>>>>>\n").map(|found| found.len()),
            Some(1)
        );
    }

    #[test]
    fn conflicts_parse_markers__a_conflict_with_two_separators_is_broken() {
        let text = "<<<<<<< HEAD\nB\n=======\nC\n=======\nD\n>>>>>>> f\nafter\n";
        let found = parse_markers(text).unwrap_or_default();
        assert_eq!(found.len(), 1);
        assert!(found[0].is_broken);
        assert_eq!(found[0].span, 0..7);
        let nested = "<<<<<<< a\n<<<<<<< b\nx\n=======\ny\n>>>>>>> b\n=======\nz\n>>>>>>> a\n";
        let found = parse_markers(nested).unwrap_or_default();
        assert_eq!(
            (found.len(), found[0].is_broken, found[0].span.clone()),
            (1, true, 0..9)
        );
    }

    #[test]
    fn conflicts_parse_markers__a_file_that_ends_inside_a_conflict_gives_none() {
        assert_eq!(parse_markers("a\n<<<<<<< HEAD\nB\n=======\nC\n"), None);
    }

    #[test]
    fn conflicts_parse_markers__crlf_is_parsed_as_lf() {
        let lf = "a\n<<<<<<< HEAD\nB\n=======\nC\n>>>>>>> f\nz";
        assert_eq!(parse_markers(&lf.replace('\n', "\r\n")), parse_markers(lf));
        assert_eq!(parse_markers(&lf.replace('\n', "\r")), parse_markers(lf));
    }

    #[test]
    fn each_line_keeps_what_ends_it() {
        assert_eq!(
            split_lines("a\r\nb\rc\nd"),
            [
                Line {
                    text: "a",
                    ending: "\r\n"
                },
                Line {
                    text: "b",
                    ending: "\r"
                },
                Line {
                    text: "c",
                    ending: "\n"
                },
                Line { text: "d", ending: "" },
            ]
        );
    }
}
