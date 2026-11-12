//! What the review of the conflicts derives from a proposal: where each of its lines comes from and what it does with
//! the two sides (`review.proposal-lines`), and what looks like a mistake in it (`review.proposal-warnings`).

mod lines;
mod warnings;

pub use lines::{Origin, OriginCounts, ProposalLines, Shape, Sides, proposal_lines};
pub use warnings::{JsonFile, Piece, ProposalWarning, WarningInput, proposal_warnings};

/// A line as the review compares it: without the spaces at its ends.
fn trimmed(line: &str) -> &str {
    line.trim()
}
