//! The answers of the AI provider kept for the conflicts of a file in the review (`ai.resolve-conflicts`), proposed in
//! its Merge page for the conflicts git still marks, each found by its fingerprint (`review.fingerprint`), as the review
//! page proposes them (`review.proposals`).

use model::merge_view::{Proposal, ProposedBy, Region};
use model::review::{AiAnswer, ProposalSource, ProposalsInput, Proposers, proposals as review_proposals};
use text::conflicts::{RuleResolutions, parse_markers};

use crate::work::review::review_file;

/// What the answers are proposed from.
pub(super) struct AnswersInput<'a> {
    /// The result of the page.
    pub result: &'a str,
    /// Its regions.
    pub regions: &'a [Region],
    /// What the rules made of its conflicts, whose base the new lines are counted against.
    pub resolved: &'a RuleResolutions<'a>,
    /// The answers kept for the conflicts of the file.
    pub answers: &'a [AiAnswer],
}

/// The proposals of the answers kept, for the conflicts git still marks whose region has no proposal yet.
pub(super) fn proposals(input: &AnswersInput<'_>, taken: &[usize]) -> Vec<Proposal> {
    if input.answers.is_empty() {
        return Vec::new();
    }
    let marked = parse_markers(input.result).unwrap_or_default();
    let file = review_file("", input.result, &marked, input.resolved);
    let proposed = review_proposals(&ProposalsInput {
        file: &file,
        proposers: Proposers { rules: false, ai: true },
        answers: input.answers,
    });
    file.conflicts
        .iter()
        .zip(proposed)
        .filter_map(|(conflict, proposal)| {
            let proposal = proposal?;
            let region = input.regions.iter().position(|region| {
                region.is_unresolved() && region.marked.as_ref().is_some_and(|block| block.span == conflict.span)
            })?;
            let ProposalSource::Ai {
                model,
                reason,
                confidence,
            } = proposal.source
            else {
                return None;
            };
            (!taken.contains(&region)).then(|| Proposal {
                region,
                replaced: proposal.replaced,
                text: proposal
                    .lines
                    .iter()
                    .flat_map(|line| [line.text.as_str(), line.ending.as_str()])
                    .collect(),
                by: ProposedBy::Ai {
                    model,
                    reason,
                    confidence,
                    new_lines: proposal.new_lines,
                },
            })
        })
        .collect()
}
