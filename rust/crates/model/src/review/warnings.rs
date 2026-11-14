//! What looks like a mistake in a proposal: a name both sides gave a value to and that the proposal keeps twice, and a
//! JSON file that would no longer be JSON.

use super::lines::Origin;
use super::trimmed;

/// What looks wrong in a proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalWarning {
    /// A name given a value by a line of the current side and by a different line of the incoming side, both kept.
    SetTwice {
        /// The name: a key, a property, a variable, a setting.
        name: String,
        /// The lines of the proposal it is on.
        lines: Vec<usize>,
    },
    /// The file, JSON with either side, would not be JSON with the proposal.
    NotJson,
}

/// A piece of a file in conflict: lines as they are, or a conflict with the text of each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece<'a> {
    /// Lines outside the conflicts, as they are.
    Lines(&'a str),
    /// A conflict.
    Conflict {
        /// The current side's text.
        current: &'a str,
        /// The incoming side's text.
        incoming: &'a str,
    },
}

/// A JSON file whose proposal is checked: its pieces, which conflict the proposal is for, and what tells JSON.
#[derive(Clone, Copy)]
pub struct JsonFile<'a> {
    /// The file, in pieces.
    pub pieces: &'a [Piece<'a>],
    /// The conflict of the proposal, by its place among the conflicts of the file.
    pub conflict: usize,
    /// Whether a text is JSON.
    pub is_json: &'a dyn Fn(&str) -> bool,
}

/// What is looked at in a proposal.
#[derive(Clone, Copy)]
pub struct WarningInput<'a> {
    /// The lines proposed.
    pub proposed: &'a [&'a str],
    /// Where each of them comes from (`review.proposal-lines`).
    pub origins: &'a [Origin],
    /// The file, when its name ends in `.json`.
    pub json: Option<JsonFile<'a>>,
}

/// The words before a name that declare it.
const DECLARING: [&str; 13] = [
    "const",
    "let",
    "var",
    "export",
    "readonly",
    "pub",
    "static",
    "mut",
    "val",
    "final",
    "public",
    "private",
    "protected",
];

/// What looks like a mistake of keeping both sides in a proposal.
///
/// A name is what a line gives a value with `:` or `=` (not `==`, `=>` or `::`), after its indentation and the words
/// that declare it, in quotes or not. It is set twice when a line of the current side and a different line of the
/// incoming side give it a value at the same indentation. A JSON file is checked when it is JSON with every conflict
/// resolved for one side: with the proposal in place of its conflict, and the others resolved for that side, it must
/// be JSON too.
///
/// Feature: review.proposal-warnings
#[must_use]
pub fn proposal_warnings(input: WarningInput<'_>) -> Vec<ProposalWarning> {
    let mut warnings = set_twice(input.proposed, input.origins);
    if let Some(file) = input.json
        && breaks_json(file, input.proposed)
    {
        warnings.push(ProposalWarning::NotJson);
    }
    warnings
}

/// A line of a side kept in the proposal that gives a name a value.
struct Named<'a> {
    /// Its place in the proposal.
    index: usize,
    /// The side it comes from.
    origin: Origin,
    /// The line, without the spaces at its ends.
    line: &'a str,
    /// Its indentation.
    indent: &'a str,
    /// The name.
    name: &'a str,
}

/// The names a line of each side gives a value to, kept both.
fn set_twice(proposed: &[&str], origins: &[Origin]) -> Vec<ProposalWarning> {
    let named: Vec<Named<'_>> = proposed
        .iter()
        .zip(origins)
        .enumerate()
        .filter(|(_, (_, origin))| matches!(origin, Origin::Current | Origin::Incoming))
        .filter_map(|(index, (line, origin))| {
            name_of(line).map(|(indent, name)| Named {
                index,
                origin: *origin,
                line: trimmed(line),
                indent,
                name,
            })
        })
        .collect();
    let mut warnings: Vec<ProposalWarning> = Vec::new();
    for current in named.iter().filter(|named| named.origin == Origin::Current) {
        let twins = named.iter().filter(|other| {
            other.origin == Origin::Incoming
                && other.indent == current.indent
                && other.name == current.name
                && other.line != current.line
        });
        let mut lines: Vec<usize> = twins.map(|other| other.index).collect();
        if lines.is_empty() {
            continue;
        }
        lines.push(current.index);
        let known = warnings.iter_mut().find_map(|warning| match warning {
            ProposalWarning::SetTwice { name, lines } if name == current.name => Some(lines),
            _ => None,
        });
        match known {
            Some(known) => known.append(&mut lines),
            None => warnings.push(ProposalWarning::SetTwice {
                name: current.name.to_owned(),
                lines,
            }),
        }
    }
    for warning in &mut warnings {
        if let ProposalWarning::SetTwice { lines, .. } = warning {
            lines.sort_unstable();
            lines.dedup();
        }
    }
    warnings
}

/// The indentation of a line and the name it gives a value to, when it gives one.
fn name_of(line: &str) -> Option<(&str, &str)> {
    let mut rest = line.trim_start();
    let indent = line.strip_suffix(rest).unwrap_or_default();
    while let Some(after) = DECLARING.iter().find_map(|word| {
        rest.strip_prefix(*word)
            .filter(|after| after.starts_with(char::is_whitespace))
    }) {
        rest = after.trim_start();
    }
    let quoted = ['"', '\''].into_iter().find_map(|quote| {
        let inside = rest.strip_prefix(quote)?;
        inside.split_once(quote)
    });
    let (name, after) = quoted.unwrap_or_else(|| {
        let end = rest
            .find(|character: char| !(character.is_alphanumeric() || "_$.-".contains(character)))
            .unwrap_or(rest.len());
        rest.split_at(end)
    });
    let after = after.trim_start();
    let gives = after.starts_with([':', '=']) && !["::", "==", "=>"].iter().any(|operator| after.starts_with(operator));
    (gives && !name.is_empty()).then_some((indent, name))
}

/// Whether the file is JSON with every conflict resolved for a side, and no longer with the proposal in its place.
fn breaks_json(file: JsonFile<'_>, proposed: &[&str]) -> bool {
    let mut proposal = proposed.join("\n");
    proposal.push('\n');
    [true, false].into_iter().any(|is_current| {
        let whole = joined(file.pieces, is_current, None);
        (file.is_json)(&whole) && !(file.is_json)(&joined(file.pieces, is_current, Some((file.conflict, &proposal))))
    })
}

/// The file with every conflict resolved for one side, but one, which has the proposal.
fn joined(pieces: &[Piece<'_>], is_current: bool, proposal: Option<(usize, &str)>) -> String {
    let mut text = String::new();
    let mut conflict = 0;
    for piece in pieces {
        match piece {
            Piece::Lines(lines) => text.push_str(lines),
            Piece::Conflict { current, incoming } => {
                let side = if is_current { current } else { incoming };
                match proposal {
                    Some((index, proposed)) if index == conflict => text.push_str(proposed),
                    _ => text.push_str(side),
                }
                conflict += 1;
            }
        }
    }
    text
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{JsonFile, Piece, ProposalWarning, WarningInput, name_of, proposal_warnings};
    use crate::review::{Sides, proposal_lines};

    /// Whether a text of one object, a member on each line, separates its members with commas: as much of JSON as
    /// these tests need, since the parser of JSON is not among what `model` may use.
    fn members_separated(text: &str) -> bool {
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
        lines.windows(2).all(|pair| match pair {
            [line, next] => *line == "{" || *next == "}" || line.ends_with(','),
            _ => true,
        }) && lines.last().is_none_or(|line| !line.ends_with(','))
            || lines.len() < 2
    }

    fn warnings(
        current: &[&str],
        incoming: &[&str],
        proposed: &[&str],
        json: Option<JsonFile<'_>>,
    ) -> Vec<ProposalWarning> {
        let origins = proposal_lines(
            Sides {
                current,
                incoming,
                base: None,
            },
            proposed,
        )
        .origins;
        proposal_warnings(WarningInput {
            proposed,
            origins: &origins,
            json,
        })
    }

    #[test]
    fn review_proposal_warnings__a_key_of_json_from_both_sides_is_set_twice_and_breaks_the_file() {
        let current = ["  \"version\": \"1.2.1\","];
        let incoming = ["  \"shipping\": true,", "  \"version\": \"1.3.0\","];
        let proposed = [current[0], "  \"shipping\": true", incoming[1]];
        let pieces = [
            Piece::Lines("{\n"),
            Piece::Conflict {
                current: "  \"version\": \"1.2.1\",\n",
                incoming: "  \"shipping\": true,\n  \"version\": \"1.3.0\",\n",
            },
            Piece::Lines("  \"name\": \"shop\"\n}\n"),
        ];
        let file = JsonFile {
            pieces: &pieces,
            conflict: 0,
            is_json: &members_separated,
        };
        let found = warnings(&current, &incoming, &proposed, Some(file));
        assert_eq!(
            found,
            [
                ProposalWarning::SetTwice {
                    name: String::from("version"),
                    lines: vec![0, 2],
                },
                ProposalWarning::NotJson,
            ]
        );
    }

    #[test]
    fn review_proposal_warnings__a_variable_declared_on_both_sides_is_set_twice_and_the_same_line_is_not() {
        let current = ["const count = items.length;", "check(count == 1);"];
        let incoming = ["const count = items.reduce(add, 0);", "check(count == 1);"];
        let proposed = [current[0], incoming[0], current[1]];
        assert_eq!(
            warnings(&current, &incoming, &proposed, None),
            [ProposalWarning::SetTwice {
                name: String::from("count"),
                lines: vec![0, 1],
            }]
        );
        let same = ["const count = 1;"];
        assert_eq!(warnings(&same, &same, &[same[0], same[0]], None), []);
    }

    #[test]
    fn review_proposal_warnings__a_json_file_neither_side_makes_json_is_not_checked() {
        let pieces = [
            Piece::Lines("{\n  // the shop\n"),
            Piece::Conflict {
                current: "  \"a\": 1\n",
                incoming: "  \"b\": 2\n",
            },
            Piece::Lines("}\n"),
        ];
        let never = |_: &str| false;
        let file = JsonFile {
            pieces: &pieces,
            conflict: 0,
            is_json: &never,
        };
        let found = warnings(
            &["  \"a\": 1"],
            &["  \"b\": 2"],
            &["  \"a\": 1", "  \"b\": 2"],
            Some(file),
        );
        assert_eq!(found, []);
    }

    #[test]
    fn the_name_a_line_gives_a_value_to() {
        assert_eq!(name_of("  \"version\": \"1.2.1\","), Some(("  ", "version")));
        assert_eq!(name_of("export const limit = 3;"), Some(("", "limit")));
        assert_eq!(name_of("pub static mut COUNT: usize = 0;"), Some(("", "COUNT")));
        assert_eq!(name_of("    timeout: 30"), Some(("    ", "timeout")));
        assert_eq!(name_of("if count == 1 {"), None);
        assert_eq!(name_of("use std::io;"), None);
        assert_eq!(name_of("items.map(item => item)"), None);
        assert_eq!(name_of("value => 1"), None);
        assert_eq!(name_of("model::review"), None);
    }

    #[test]
    fn names_at_other_indentations_are_not_the_same_setting() {
        let current = ["  \"name\": \"a\","];
        let incoming = ["    \"name\": \"b\","];
        assert_eq!(warnings(&current, &incoming, &[current[0], incoming[0]], None), []);
    }
}
