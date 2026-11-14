//! What a choice puts in a region of the result, which choices need no thought, how many conflicts are left, and the
//! line of the result a line of a column stands for.

use std::ops::Range;

use super::{Region, RegionKind};

/// The four texts of a Merge page, as lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergeTexts<'a> {
    /// The base.
    pub base: &'a [&'a str],
    /// The current side.
    pub current: &'a [&'a str],
    /// The incoming side.
    pub incoming: &'a [&'a str],
    /// The result.
    pub result: &'a [&'a str],
}

/// What is put in place of a region of the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Choice {
    /// **Accept current** (→).
    Current,
    /// **Accept incoming** (←).
    Incoming,
    /// **Accept both, current first** (+ on the current side).
    CurrentFirst,
    /// **Accept both, incoming first** (+ on the incoming side).
    IncomingFirst,
}

/// A column of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Column {
    /// The base, what the two sides started from.
    Base,
    /// The current side.
    Current,
    /// The result.
    Result,
    /// The incoming side.
    Incoming,
}

impl Region {
    /// The lines of the current side in the region: for a conflict git still marks, the part between its markers.
    #[must_use]
    pub fn current_lines<'a>(&self, texts: &MergeTexts<'a>) -> Vec<&'a str> {
        match &self.marked {
            Some(block) if self.is_unresolved() => lines(texts.result, &block.current),
            _ => lines(texts.current, &self.current),
        }
    }

    /// The lines of the incoming side in the region: for a conflict git still marks, the part between its markers.
    #[must_use]
    pub fn incoming_lines<'a>(&self, texts: &MergeTexts<'a>) -> Vec<&'a str> {
        match &self.marked {
            Some(block) if self.is_unresolved() => lines(texts.result, &block.incoming),
            _ => lines(texts.incoming, &self.incoming),
        }
    }

    /// The lines a choice puts in place of the region in the result.
    #[must_use]
    pub fn chosen<'a>(&self, texts: &MergeTexts<'a>, choice: Choice) -> Vec<&'a str> {
        let (current, incoming) = (self.current_lines(texts), self.incoming_lines(texts));
        match choice {
            Choice::Current => current,
            Choice::Incoming => incoming,
            Choice::CurrentFirst => current.into_iter().chain(incoming).collect(),
            Choice::IncomingFirst => incoming.into_iter().chain(current).collect(),
        }
    }

    /// Whether a choice would change what the result has in the region.
    #[must_use]
    pub fn would_change(&self, texts: &MergeTexts<'_>, choice: Choice) -> bool {
        self.is_unresolved() || self.chosen(texts, choice) != lines(texts.result, &self.result)
    }
}

/// The lines of a text at a range, empty past its end.
fn lines<'a>(text: &[&'a str], range: &Range<usize>) -> Vec<&'a str> {
    text.get(range.clone()).unwrap_or_default().to_vec()
}

/// The choices that need no thought, for each region whose result does not have it already: the incoming side for a
/// region of the incoming side; the current side for a region of the current side or of both, and for a conflict whose
/// two sides have the same lines or whose incoming side has the base's lines; the incoming side for a conflict whose
/// current side has the base's lines.
#[must_use]
pub fn simple_choices(regions: &[Region], texts: &MergeTexts<'_>) -> Vec<(usize, Choice)> {
    regions
        .iter()
        .enumerate()
        .filter_map(|(place, region)| {
            let choice = match region.kind {
                RegionKind::Incoming => Some(Choice::Incoming),
                RegionKind::Current | RegionKind::Both => Some(Choice::Current),
                RegionKind::Conflict => {
                    let current = region.current_lines(texts);
                    let incoming = region.incoming_lines(texts);
                    let base = lines(texts.base, &region.base);
                    if current == incoming || incoming == base {
                        Some(Choice::Current)
                    } else if current == base {
                        Some(Choice::Incoming)
                    } else {
                        None
                    }
                }
            }?;
            region.would_change(texts, choice).then_some((place, choice))
        })
        .collect()
}

/// How many conflicts git still marks.
#[must_use]
pub fn conflicts_left(regions: &[Region]) -> usize {
    regions.iter().filter(|region| region.is_unresolved()).count()
}

/// Whether the file had conflicts: a region the two sides changed differently, marked still or not.
#[must_use]
pub fn has_conflicts(regions: &[Region]) -> bool {
    regions.iter().any(|region| region.kind == RegionKind::Conflict)
}

/// The line of the result a line of a column stands for: in the result, itself; in a side or the base, a line before a
/// region moves by as many lines as the regions before it grew or shrank in the result, and a line inside a region goes
/// to the first line of the region in the result (for a conflict git still marks, the first line of that side's part).
#[must_use]
pub fn result_line(regions: &[Region], (column, line): (Column, usize)) -> usize {
    let range = |region: &Region| match column {
        Column::Base => region.base.clone(),
        Column::Current => region.current.clone(),
        Column::Incoming => region.incoming.clone(),
        Column::Result => region.result.clone(),
    };
    if column == Column::Result {
        return line;
    }
    let mut found = line;
    for region in regions {
        let lines = range(region);
        if lines.contains(&line) {
            let part = region
                .marked
                .as_ref()
                .filter(|_| region.is_unresolved())
                .map(|block| match column {
                    Column::Incoming => block.incoming.start,
                    Column::Base | Column::Current | Column::Result => block.current.start,
                });
            return part.unwrap_or(region.result.start);
        }
        if line < lines.start {
            break;
        }
        found = region.result.end + (line - lines.end);
    }
    found
}
