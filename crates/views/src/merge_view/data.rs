//! What a Merge page is made of: the four texts of the file, their regions, what each region offers and what is
//! proposed for its conflicts.

use std::ops::Range;

use model::merge_view::{Choice, MergeTexts, Proposal, Region};

/// The texts of a file in conflict and their regions, as a worker computed them for one text of the result.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MergeText {
    /// The base, as UTF-8 with what is not replaced; empty when both sides added the file.
    pub base: String,
    /// The current side.
    pub current: String,
    /// The incoming side.
    pub incoming: String,
    /// The result the regions were computed for.
    pub result: String,
    /// Where each line of the base is in its text, without what ends it.
    pub base_lines: Vec<Range<usize>>,
    /// Where each line of the current side is.
    pub current_lines: Vec<Range<usize>>,
    /// Where each line of the incoming side is.
    pub incoming_lines: Vec<Range<usize>>,
    /// Where each line of the result is.
    pub result_lines: Vec<Range<usize>>,
    /// The regions, in order (`merge-view.three-way`).
    pub regions: Vec<Region>,
    /// What the buttons of each region would do, in the order of the regions.
    pub offers: Vec<Offers>,
    /// The choices that need no thought (`merge-view.accept-simple`).
    pub simple: Vec<(usize, Choice)>,
    /// What the rules propose for the conflicts git still marks (`merge-view.accept-proposals`).
    pub proposals: Vec<Proposal>,
}

/// Which choices would change what the result has in a region: the buttons its bands show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[allow(clippy::struct_excessive_bools, reason = "each is a button of its own, shown or not")]
pub struct Offers {
    /// **→**.
    pub current: bool,
    /// **+** on the current side.
    pub current_first: bool,
    /// **←**.
    pub incoming: bool,
    /// **+** on the incoming side.
    pub incoming_first: bool,
}

impl MergeText {
    /// A line of the base.
    #[must_use]
    pub fn base_line(&self, line: usize) -> &str {
        text_of(&self.base, &self.base_lines, line)
    }

    /// A line of the current side.
    #[must_use]
    pub fn current_line(&self, line: usize) -> &str {
        text_of(&self.current, &self.current_lines, line)
    }

    /// A line of the incoming side.
    #[must_use]
    pub fn incoming_line(&self, line: usize) -> &str {
        text_of(&self.incoming, &self.incoming_lines, line)
    }

    /// A line of the result.
    #[must_use]
    pub fn result_line(&self, line: usize) -> &str {
        text_of(&self.result, &self.result_lines, line)
    }

    /// The lines of each text, for what is derived from all of them.
    #[must_use]
    pub fn lines(&self) -> TextLines<'_> {
        TextLines {
            base: of(&self.base, &self.base_lines),
            current: of(&self.current, &self.current_lines),
            incoming: of(&self.incoming, &self.incoming_lines),
            result: of(&self.result, &self.result_lines),
        }
    }

    /// How many conflicts git still marks.
    #[must_use]
    pub fn conflicts_left(&self) -> usize {
        model::merge_view::conflicts_left(&self.regions)
    }

    /// What is proposed for a region, when something is.
    #[must_use]
    pub fn proposal_of(&self, region: usize) -> Option<(usize, &Proposal)> {
        self.proposals
            .iter()
            .enumerate()
            .find(|(_, proposal)| proposal.region == region)
    }
}

/// The lines of the four texts.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TextLines<'a> {
    /// The base.
    pub base: Vec<&'a str>,
    /// The current side.
    pub current: Vec<&'a str>,
    /// The incoming side.
    pub incoming: Vec<&'a str>,
    /// The result.
    pub result: Vec<&'a str>,
}

impl<'a> TextLines<'a> {
    /// The four texts as the derivations take them.
    #[must_use]
    pub fn texts(&'a self) -> MergeTexts<'a> {
        MergeTexts {
            base: &self.base,
            current: &self.current,
            incoming: &self.incoming,
            result: &self.result,
        }
    }
}

/// The lines of a text, from where they are in it.
fn of<'a>(text: &'a str, ranges: &[Range<usize>]) -> Vec<&'a str> {
    ranges
        .iter()
        .map(|range| text.get(range.clone()).unwrap_or_default())
        .collect()
}

/// The text of a line, empty past the end.
fn text_of<'a>(text: &'a str, lines: &[Range<usize>], line: usize) -> &'a str {
    lines
        .get(line)
        .and_then(|range| text.get(range.clone()))
        .unwrap_or_default()
}
