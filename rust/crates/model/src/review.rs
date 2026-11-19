//! What the review of the conflicts reads and derives: the files in conflict with their conflicts (`review.page`),
//! what each side changed of the base (`review.side-diff`), what is proposed for each conflict (`review.proposals`),
//! where each line of a proposal comes from and what it does with the two sides (`review.proposal-lines`), and what
//! looks like a mistake in it (`review.proposal-warnings`).

mod file;
mod lines;
mod proposals;
mod resolutions;
mod side_diff;
mod warnings;

pub use file::{
    ConflictStretch, DeletedFile, FileLine, KeptChanges, KeptRow, Overlap, RecordedFile, ReviewConflict, ReviewFile,
    ReviewRead, RuleCase, RuleProposal, Which, replaced_text, split_lines,
};
pub use lines::{Origin, OriginCounts, ProposalLines, Shape, Sides, proposal_lines};
pub use proposals::{
    AiAnswer, ChoiceKind, Proposal, ProposalSource, ProposalsInput, Proposers, chosen, in_place_of_markers, new_lines,
    proposals,
};
pub use resolutions::{FileResolutions, Resolutions};
pub use side_diff::{LineChanges, SideDiff, SideDiffInput, SideRow, WordMarks, side_diff};
pub use warnings::{JsonFile, Piece, ProposalWarning, WarningInput, proposal_warnings};

/// A line as the review compares it: without the spaces at its ends.
fn trimmed(line: &str) -> &str {
    line.trim()
}
