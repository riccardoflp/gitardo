#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{AiAnswer, ProposalSource, ProposalsInput, Proposers, proposals};
use crate::ai::{Confidence, ConflictAnswer};
use crate::review::file::{FileLine, ReviewConflict, ReviewFile, RuleCase, RuleProposal};

/// A file `a / <<<<<<< HEAD / B / ======= / C / >>>>>>> f / d` in some line ending, with the rules' proposal or not.
fn file(ending: &str, rule: bool) -> ReviewFile {
    let text = ["a", "<<<<<<< HEAD", "B", "=======", "C", ">>>>>>> f", "d"]
        .iter()
        .map(|line| [line, ending].concat())
        .collect();
    ReviewFile {
        path: String::from("src/app.rs"),
        text,
        conflicts: vec![ReviewConflict {
            index: 0,
            fingerprint: String::from("0123456789abcdef"),
            line: 2,
            span: 1..6,
            current: vec![String::from("B")],
            base: None,
            incoming: vec![String::from("    call();")],
            rule: rule.then(|| RuleProposal {
                case: RuleCase::NextToEachOther,
                replaced: 1..6,
                lines: vec![FileLine::new("B", ending)],
            }),
            stretch: None,
        }],
    }
}

/// An answer of the AI provider for the conflict of [`file`].
fn answer(lines: &[&str]) -> AiAnswer {
    AiAnswer {
        fingerprint: String::from("0123456789abcdef"),
        model: String::from("claude"),
        answer: ConflictAnswer {
            lines: lines.iter().map(|line| (*line).to_owned()).collect(),
            reason: String::from("Both changes are kept."),
            confidence: Confidence::High,
        },
    }
}

const BOTH: Proposers = Proposers { rules: true, ai: true };

#[test]
fn review_proposals__with_the_rules_switched_off_a_conflict_they_resolve_has_no_rules_proposal() {
    let file = file("\n", true);
    let found = proposals(&ProposalsInput {
        file: &file,
        proposers: Proposers { rules: false, ai: true },
        answers: &[],
    });
    assert_eq!(found, [None]);
    let on = proposals(&ProposalsInput {
        file: &file,
        proposers: BOTH,
        answers: &[],
    });
    assert!(matches!(
        on.first(),
        Some(Some(proposal)) if proposal.source == ProposalSource::Rules(RuleCase::NextToEachOther)
    ));
}

#[test]
fn review_proposals__the_rules_come_before_an_answer_of_the_ai() {
    let file = file("\n", true);
    let answers = [answer(&["x();"])];
    let found = proposals(&ProposalsInput {
        file: &file,
        proposers: BOTH,
        answers: &answers,
    });
    let Some(Some(proposal)) = found.first() else {
        panic!("a proposal")
    };
    assert_eq!(proposal.source, ProposalSource::Rules(RuleCase::NextToEachOther));
    assert_eq!(proposal.new_lines, 0);
}

#[test]
fn review_proposals__an_answer_in_a_crlf_file_ends_its_lines_with_crlf() {
    let file = file("\r\n", false);
    let answers = [answer(&["x();"])];
    let found = proposals(&ProposalsInput {
        file: &file,
        proposers: BOTH,
        answers: &answers,
    });
    let Some(Some(proposal)) = found.first() else {
        panic!("a proposal")
    };
    assert_eq!(proposal.lines, [FileLine::new("x();", "\r\n")]);
    assert_eq!(proposal.replaced, 1..6);
    assert!(matches!(proposal.source, ProposalSource::Ai { .. }));
}

#[test]
fn review_proposals__a_line_no_side_has_is_new_and_one_indented_otherwise_is_not() {
    let file = file("\n", false);
    let count = |lines: &[&str]| {
        let answers = [answer(lines)];
        let found = proposals(&ProposalsInput {
            file: &file,
            proposers: BOTH,
            answers: &answers,
        });
        match found.first() {
            Some(Some(proposal)) => proposal.new_lines,
            _ => panic!("a proposal"),
        }
    };
    assert_eq!(count(&["log(value);"]), 1);
    assert_eq!(count(&["call();"]), 0);
    assert_eq!(count(&["  B", ""]), 0);
}
