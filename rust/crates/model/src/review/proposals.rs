//! What is proposed for each conflict of a file: the resolution of the rules, else the answer of the AI provider kept
//! for it, else nothing; and the choices of the user put in place of either.

use std::ops::Range;

use super::file::{FileLine, ReviewConflict, ReviewFile, RuleCase};
use super::trimmed;
use crate::ai::{Confidence, ConflictAnswer};

/// An answer of the AI provider kept for a conflict, by its fingerprint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiAnswer {
    /// The fingerprint of the conflict it answers (`review.fingerprint`).
    pub fingerprint: String,
    /// The model that answered, as the provider names it.
    pub model: String,
    /// What it answered.
    pub answer: ConflictAnswer,
}

/// The choice of a side, of both, or of lines written in the card (`review.choose`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChoiceKind {
    /// **Current only**.
    CurrentOnly,
    /// **Incoming only**.
    IncomingOnly,
    /// **Both, current first**.
    BothCurrentFirst,
    /// **Both, incoming first**.
    BothIncomingFirst,
    /// **Edit**: lines written in the card.
    Written,
}

/// Where a proposal comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalSource {
    /// The rules (`conflicts.resolve-by-rules`), with the case.
    Rules(RuleCase),
    /// The AI provider.
    Ai {
        /// The model that answered.
        model: String,
        /// Why, in its words.
        reason: String,
        /// How sure it said it is.
        confidence: Confidence,
    },
    /// The user's choice.
    Choice(ChoiceKind),
}

/// What is proposed for a conflict: the lines of the file it takes the place of, and the lines put there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    /// Where it comes from.
    pub source: ProposalSource,
    /// The lines of the file it replaces, from 0.
    pub replaced: Range<usize>,
    /// The lines put in their place, each with what ends it.
    pub lines: Vec<FileLine>,
    /// How many of its lines neither side nor the base has.
    pub new_lines: usize,
}

impl Proposal {
    /// The lines proposed, without what ends them.
    #[must_use]
    pub fn texts(&self) -> Vec<&str> {
        self.lines.iter().map(|line| line.text.as_str()).collect()
    }

    /// Whether the AI provider proposed it.
    #[must_use]
    pub fn is_from_ai(&self) -> bool {
        matches!(self.source, ProposalSource::Ai { .. })
    }
}

/// Who proposes: the rules with `conflicts.autoResolve`, the AI provider with `conflicts.ai`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Proposers {
    /// `conflicts.autoResolve`: the rules propose.
    pub rules: bool,
    /// `conflicts.ai`: the answers of the AI provider are proposed.
    pub ai: bool,
}

/// What the proposals of a file are made from.
#[derive(Clone, Copy, Debug)]
pub struct ProposalsInput<'a> {
    /// The file and its conflicts.
    pub file: &'a ReviewFile,
    /// Who proposes.
    pub proposers: Proposers,
    /// The answers of the AI provider kept for the conflicts of the file.
    pub answers: &'a [AiAnswer],
}

/// What is proposed for each conflict of a file, in order.
///
/// The rules come first: their proposal has the case as its reason and replaces the lines they replace. Otherwise an
/// answer of the AI provider kept for the conflict's fingerprint replaces exactly the lines from its first marker to
/// its last, in the line endings of the file. A proposal's new lines are those, trimmed and not empty, that neither
/// side nor the base has. Otherwise nothing is proposed.
///
/// Feature: review.proposals
#[must_use]
pub fn proposals(input: &ProposalsInput<'_>) -> Vec<Option<Proposal>> {
    input
        .file
        .conflicts
        .iter()
        .map(|conflict| {
            let rules = conflict
                .rule
                .as_ref()
                .filter(|_| input.proposers.rules)
                .map(|rule| Proposal {
                    source: ProposalSource::Rules(rule.case),
                    replaced: rule.replaced.clone(),
                    lines: rule.lines.clone(),
                    new_lines: 0,
                });
            rules.or_else(|| {
                let answer = input
                    .answers
                    .iter()
                    .filter(|_| input.proposers.ai)
                    .find(|answer| answer.fingerprint == conflict.fingerprint)?;
                let source = ProposalSource::Ai {
                    model: answer.model.clone(),
                    reason: answer.answer.reason.clone(),
                    confidence: answer.answer.confidence,
                };
                Some(in_place_of_markers(input.file, conflict, source, &answer.answer.lines))
            })
        })
        .collect()
}

/// A proposal of some lines in place of a conflict's lines from its first marker to its last, each ending as the line
/// of its first marker ends, the last one without an ending when the last marker was the last line of the file and had
/// none.
#[must_use]
pub fn in_place_of_markers(
    file: &ReviewFile,
    conflict: &ReviewConflict,
    source: ProposalSource,
    texts: &[String],
) -> Proposal {
    let ending = file.ending_of(conflict);
    let last = texts.len().saturating_sub(1);
    let bare = file.ends_without_ending(conflict);
    let lines = texts
        .iter()
        .enumerate()
        .map(|(index, text)| FileLine::new(text, if bare && index == last { "" } else { ending }))
        .collect();
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    Proposal {
        source,
        replaced: conflict.span.clone(),
        lines,
        new_lines: new_lines(conflict, &texts),
    }
}

/// How many lines, trimmed and not empty, neither side of a conflict nor its base has.
#[must_use]
pub fn new_lines(conflict: &ReviewConflict, lines: &[&str]) -> usize {
    let known: Vec<&str> = conflict
        .current
        .iter()
        .chain(&conflict.incoming)
        .chain(conflict.known_base().unwrap_or_default())
        .map(|line| trimmed(line))
        .collect();
    lines
        .iter()
        .map(|line| trimmed(line))
        .filter(|line| !line.is_empty() && !known.contains(line))
        .count()
}

/// The proposal of a choice: a side, both one after the other, or the lines written, in place of the conflict's lines
/// from its first marker to its last.
#[must_use]
pub fn chosen(file: &ReviewFile, conflict: &ReviewConflict, kind: ChoiceKind, written: &[String]) -> Proposal {
    let lines: Vec<String> = match kind {
        ChoiceKind::CurrentOnly => conflict.current.clone(),
        ChoiceKind::IncomingOnly => conflict.incoming.clone(),
        ChoiceKind::BothCurrentFirst => conflict.current.iter().chain(&conflict.incoming).cloned().collect(),
        ChoiceKind::BothIncomingFirst => conflict.incoming.iter().chain(&conflict.current).cloned().collect(),
        ChoiceKind::Written => written.to_vec(),
    };
    in_place_of_markers(file, conflict, ProposalSource::Choice(kind), &lines)
}

#[cfg(test)]
#[path = "proposals_tests.rs"]
mod tests;
