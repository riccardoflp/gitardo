//! A file in conflict as the review reads it: its text, each conflict git marked in it with its sides, what the rules
//! make of it and what each side changed of the base; and the text the file becomes with resolutions in place of
//! their lines.

use std::ops::Range;

use crate::conflicts::SideNames;
use crate::state::Operation;

/// A line of a file, with what ends it: `\n`, `\r\n`, `\r`, or nothing for a last line without one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileLine {
    /// Its text, without what ends it.
    pub text: String,
    /// What ends it.
    pub ending: String,
}

impl FileLine {
    /// A line with an ending.
    #[must_use]
    pub fn new(text: &str, ending: &str) -> FileLine {
        FileLine {
            text: text.to_owned(),
            ending: ending.to_owned(),
        }
    }
}

/// Why the rules could resolve a conflict, as the review names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuleCase {
    /// Spaces or line endings only.
    SpacesOrLineEndings,
    /// The same change on both sides.
    SameChange,
    /// A change on one side only.
    OneSide,
    /// Changes to lines next to each other.
    NextToEachOther,
}

impl RuleCase {
    /// The case in words, its first letter in upper case: the reason of a proposal of the rules.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            RuleCase::SpacesOrLineEndings => "Spaces or line endings only",
            RuleCase::SameChange => "The same change on both sides",
            RuleCase::OneSide => "A change on one side only",
            RuleCase::NextToEachOther => "Changes to lines next to each other",
        }
    }
}

/// What the rules resolve a conflict to: the lines of the file it replaces and those put in their place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleProposal {
    /// Why it could be resolved.
    pub case: RuleCase,
    /// The lines of the file replaced, from 0: the markers and the lines both sides share around them.
    pub replaced: Range<usize>,
    /// The lines put in their place.
    pub lines: Vec<FileLine>,
}

/// What the base and each side have from where the two sides are known to be at the same place before a conflict to
/// where they are again after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictStretch {
    /// Where it is in the file, from 0: from the line after the upper anchor to the line before the lower one.
    pub file_lines: Range<usize>,
    /// The lines of the base.
    pub base: Vec<String>,
    /// The lines of the current side.
    pub current: Vec<String>,
    /// The lines of the incoming side.
    pub incoming: Vec<String>,
}

/// A conflict git marked in a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewConflict {
    /// Its place among the conflicts of the file, from 0.
    pub index: usize,
    /// Its fingerprint (`review.fingerprint`).
    pub fingerprint: String,
    /// The line of its first marker, from 1.
    pub line: usize,
    /// Its lines in the file, from 0: from its first marker to the one after its last.
    pub span: Range<usize>,
    /// The current side's lines between the markers.
    pub current: Vec<String>,
    /// The base's lines git wrote between `|||||||` and `=======`, when it did.
    pub base: Option<Vec<String>>,
    /// The incoming side's lines between the markers.
    pub incoming: Vec<String>,
    /// What the rules resolve it to, when they do.
    pub rule: Option<RuleProposal>,
    /// What the base and each side have around it, when the rules found where the sides meet again.
    pub stretch: Option<ConflictStretch>,
}

impl ReviewConflict {
    /// The base, as far as it is known: the stretch's, else what git wrote between the markers.
    #[must_use]
    pub fn known_base(&self) -> Option<&[String]> {
        self.stretch
            .as_ref()
            .map(|stretch| stretch.base.as_slice())
            .or(self.base.as_deref())
    }
}

/// A file with conflicts marked in it, as read for the review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewFile {
    /// Its path from the top of the working tree.
    pub path: String,
    /// Its text as read: the resolutions are put in it, and it is checked again before they are written.
    pub text: String,
    /// Its conflicts, in order.
    pub conflicts: Vec<ReviewConflict>,
}

impl ReviewFile {
    /// Its lines, each with what ends it.
    #[must_use]
    pub fn lines(&self) -> Vec<(&str, &str)> {
        split_lines(&self.text)
    }

    /// The line ending of a conflict: CRLF when the line of its first marker ends with CRLF, LF otherwise.
    #[must_use]
    pub fn ending_of(&self, conflict: &ReviewConflict) -> &'static str {
        let lines = self.lines();
        match lines.get(conflict.span.start) {
            Some((_, "\r\n")) => "\r\n",
            _ => "\n",
        }
    }

    /// Whether the last marker of a conflict is the last line of the file and has no ending.
    #[must_use]
    pub fn ends_without_ending(&self, conflict: &ReviewConflict) -> bool {
        let lines = self.lines();
        conflict.span.end == lines.len() && lines.last().is_some_and(|(_, ending)| ending.is_empty())
    }
}

/// Which side of a conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Which {
    /// The current side.
    Current,
    /// The incoming side.
    Incoming,
}

/// A file one side deleted and the other changed: what the side that kept it changed of the base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletedFile {
    /// Its path from the top of the working tree.
    pub path: String,
    /// The side that deleted it.
    pub deleted_by: Which,
    /// What the side that kept it changed.
    pub changes: KeptChanges,
}

/// What the side that kept a file deleted on the other changed of the base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptChanges {
    /// Its changes, as rows of a diff, with the lines between the changes left out.
    Rows(Vec<KeptRow>),
    /// It changed nothing of the lines of the file.
    Nothing,
    /// The changes are too many to show.
    TooMany,
}

/// A row of the changes of a file deleted on one side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptRow {
    /// A line alike in the base and the side, by its number in the side.
    Same(usize, String),
    /// A line of the base the side took out.
    Removed(String),
    /// A line the side wrote, by its number in the side.
    Added(usize, String),
    /// Lines left out here.
    LeftOut,
}

/// What the review reads of the files in conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewRead {
    /// The operation that stopped, none when the conflicts come from a stash applied.
    pub operation: Option<Operation>,
    /// What the two sides are.
    pub sides: SideNames,
    /// The files with conflicts marked in them.
    pub files: Vec<ReviewFile>,
    /// The files one side deleted and the other changed.
    pub deleted: Vec<DeletedFile>,
    /// The other files in conflict: binary, very large, added on both sides without markers.
    pub others: Vec<String>,
}

/// Why resolutions cannot be put in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overlap;

/// The lines of a text, each with what ends it.
#[must_use]
pub fn split_lines(text: &str) -> Vec<(&str, &str)> {
    let mut lines = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let end = rest.find(['\n', '\r']).unwrap_or(rest.len());
        let (line, after) = rest.split_at(end);
        let length = if after.starts_with("\r\n") {
            2
        } else {
            usize::from(!after.is_empty())
        };
        let (ending, after) = after.split_at(length);
        lines.push((line, ending));
        rest = after;
    }
    lines
}

/// The text of a file with lines put in place of others: each replacement is the lines of the file it takes, from 0,
/// and the lines put there. Replacements that take the same lines cannot be put together.
///
/// # Errors
///
/// [`Overlap`] when two replacements take a line in common.
pub fn replaced_text(text: &str, replacements: &[(Range<usize>, &[FileLine])]) -> Result<String, Overlap> {
    let mut sorted: Vec<&(Range<usize>, &[FileLine])> = replacements.iter().collect();
    sorted.sort_by_key(|(range, _)| (range.start, range.end));
    if sorted
        .windows(2)
        .any(|pair| matches!(pair, [first, second] if first.0.end > second.0.start))
    {
        return Err(Overlap);
    }
    let lines = split_lines(text);
    let mut written = String::with_capacity(text.len());
    let mut next = 0;
    for (range, put) in sorted {
        for (line, ending) in lines.get(next..range.start).unwrap_or_default() {
            written.push_str(line);
            written.push_str(ending);
        }
        for line in *put {
            written.push_str(&line.text);
            written.push_str(&line.ending);
        }
        next = next.max(range.end);
    }
    for (line, ending) in lines.get(next..).unwrap_or_default() {
        written.push_str(line);
        written.push_str(ending);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::{FileLine, Overlap, replaced_text, split_lines};

    #[test]
    fn splits_lines_keeping_what_ends_each() {
        assert_eq!(split_lines("a\r\nb\nc"), [("a", "\r\n"), ("b", "\n"), ("c", "")]);
        assert_eq!(split_lines(""), Vec::<(&str, &str)>::new());
    }

    #[test]
    fn puts_lines_in_place_of_others_and_refuses_two_on_the_same_lines() {
        let put = [FileLine::new("X", "\n")];
        let text = "a\nb\nc\nd\n";
        assert_eq!(replaced_text(text, &[(1..3, &put)]), Ok(String::from("a\nX\nd\n")));
        assert_eq!(
            replaced_text(text, &[(3..4, &[]), (0..1, &put)]),
            Ok(String::from("X\nb\nc\n"))
        );
        assert_eq!(replaced_text(text, &[(0..2, &put), (1..3, &put)]), Err(Overlap));
    }
}
