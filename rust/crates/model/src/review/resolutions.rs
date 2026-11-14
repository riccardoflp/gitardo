//! The resolutions accepted on the review of the conflicts, file by file, as `review.write` writes them and
//! `review.show-result` shows them.

use super::file::{Overlap, replaced_text};
use super::proposals::{Proposal, ProposalSource};

/// The resolutions accepted in a file, and the text they were made from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileResolutions {
    /// Its path from the top of the working tree.
    pub path: String,
    /// Its text when the proposals were made: the file is written only while it still is.
    pub text: String,
    /// How many conflicts it has.
    pub conflicts: usize,
    /// The proposals accepted.
    pub accepted: Vec<Proposal>,
}

impl FileResolutions {
    /// The text of the file with the accepted proposals in place of their lines.
    ///
    /// # Errors
    ///
    /// [`Overlap`] when two of them take the place of the same lines.
    pub fn resolved(&self) -> Result<String, Overlap> {
        let replacements: Vec<_> = self
            .accepted
            .iter()
            .map(|proposal| (proposal.replaced.clone(), proposal.lines.as_slice()))
            .collect();
        replaced_text(&self.text, &replacements)
    }

    /// Whether every conflict of it is accepted, so that none is left marked once they are written.
    #[must_use]
    pub fn is_whole(&self) -> bool {
        self.accepted.len() >= self.conflicts
    }

    /// How many of the accepted come from the rules, from the AI and from the user.
    #[must_use]
    pub fn sources(&self) -> (usize, usize, usize) {
        self.accepted
            .iter()
            .fold((0, 0, 0), |(rules, ai, chosen), proposal| match proposal.source {
                ProposalSource::Rules(_) => (rules + 1, ai, chosen),
                ProposalSource::Ai { .. } => (rules, ai + 1, chosen),
                ProposalSource::Choice(_) => (rules, ai, chosen + 1),
            })
    }
}

/// The resolutions accepted on the review, for every file of it, with or without any accepted.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Resolutions {
    /// The files, in the order of the review.
    pub files: Vec<FileResolutions>,
}

impl Resolutions {
    /// How many resolutions are accepted.
    #[must_use]
    pub fn accepted(&self) -> usize {
        self.files.iter().map(|file| file.accepted.len()).sum()
    }

    /// How many of them come from the AI.
    #[must_use]
    pub fn from_ai(&self) -> usize {
        self.files.iter().map(|file| file.sources().1).sum()
    }

    /// How many conflicts are left once they are written.
    #[must_use]
    pub fn left(&self) -> usize {
        self.files
            .iter()
            .map(|file| file.conflicts.saturating_sub(file.accepted.len()))
            .sum()
    }
}
