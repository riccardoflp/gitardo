#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::ai::Confidence;
use model::review::{Origin, OriginCounts, ProposalSource, RuleCase, Shape, SideDiff};

use super::card::origin_words;
use super::decide::origin;
use super::{CardProposal, CardShown, ConflictCard, ConflictKey, SideColumn};
use crate::text::review::{heading, origin_counts};

/// The names of the sides of a merge of `feature/map-view` into `main`.
const NAMES: (&str, &str) = ("main", "feature/map-view");

/// Counts of lines from each place.
fn counted(current: usize, both: usize, incoming: usize, new: usize) -> OriginCounts {
    OriginCounts {
        current,
        incoming,
        both,
        base: 0,
        new,
    }
}

#[test]
fn review_card__the_heading_says_what_the_proposal_does_with_the_two_sides() {
    let plain = OriginCounts::default();
    let cases = [
        (Some(Shape::Current), plain, "Takes only the current side, main"),
        (
            Some(Shape::Incoming),
            plain,
            "Takes only the incoming side, feature/map-view",
        ),
        (
            Some(Shape::CurrentFirst),
            plain,
            "Keeps both sides: main first, then feature/map-view",
        ),
        (
            Some(Shape::IncomingFirst),
            plain,
            "Keeps both sides: feature/map-view first, then main",
        ),
        (Some(Shape::Mixed), counted(1, 0, 1, 0), "Takes lines of both sides"),
        (
            Some(Shape::Mixed),
            counted(1, 0, 1, 1),
            "Takes lines of both sides, and writes 1 line of its own",
        ),
        (
            Some(Shape::Mixed),
            counted(0, 0, 0, 2),
            "Writes 2 lines of its own in place of both sides",
        ),
        (Some(Shape::Nothing), plain, "Leaves out the lines of both sides"),
        (None, plain, "Nothing proposed yet"),
    ];
    for (shape, counts, said) in cases {
        assert_eq!(heading(shape, &counts, NAMES), said);
    }
}

#[test]
fn review_card__the_counts_leave_out_a_zero_but_the_new_lines_of_the_ai() {
    assert_eq!(
        origin_counts(&counted(4, 1, 4, 0), NAMES, true),
        "4 from main \u{b7} 1 on both sides \u{b7} 4 from feature/map-view \u{b7} 0 new"
    );
    assert_eq!(
        origin_counts(&counted(4, 1, 4, 0), NAMES, false),
        "4 from main \u{b7} 1 on both sides \u{b7} 4 from feature/map-view"
    );
    assert_eq!(
        origin_counts(&counted(0, 0, 2, 0), NAMES, false),
        "2 from feature/map-view"
    );
}

#[test]
fn review_card__the_pointer_on_a_line_says_where_it_comes_from() {
    let names = (String::from("main"), String::from("feature/map-view"));
    let said = |origin| origin_words(origin, &names);
    assert_eq!(said(Origin::Current).as_deref(), Some("From main"));
    assert_eq!(said(Origin::Incoming).as_deref(), Some("From feature/map-view"));
    assert_eq!(said(Origin::Both).as_deref(), Some("On both sides"));
    assert_eq!(
        said(Origin::Base).as_deref(),
        Some("As it was before either side changed it")
    );
    assert_eq!(said(Origin::New).as_deref(), Some("New: neither side has this line"));
    assert_eq!(said(Origin::Blank), None);
}

/// A card with a proposal from a source, with some new lines.
fn card(source: ProposalSource, new: usize) -> ConflictCard {
    let empty = SideDiff {
        before: Vec::new(),
        rows: Vec::new(),
        after: Vec::new(),
        is_compared: false,
    };
    ConflictCard {
        key: ConflictKey {
            path: String::from("src/app.rs"),
            index: 0,
            fingerprint: String::from("f"),
        },
        line: 11,
        ai: super::CardAi::Idle,
        proposal: Some(CardProposal {
            source,
            shape: Shape::Mixed,
            counts: counted(1, 0, 1, new),
            lines: Vec::new(),
            warnings: Vec::new(),
            from_current: Vec::new(),
            to_incoming: Vec::new(),
        }),
        before: Vec::new(),
        after: Vec::new(),
        side_diffs: Box::new((empty.clone(), empty)),
        base: None,
        current: SideColumn::default(),
        incoming: SideColumn::default(),
        choices: Vec::new(),
        is_accepted: false,
        shown: CardShown::default(),
    }
}

#[test]
fn review_card__a_rules_proposal_shows_its_case_and_an_ai_one_its_new_lines() {
    let rules = origin(&card(ProposalSource::Rules(RuleCase::NextToEachOther), 0));
    assert_eq!(rules, "Rules \u{b7} Changes to lines next to each other");
    let ai = ProposalSource::Ai {
        model: String::from("model-a"),
        reason: String::from("Both changes kept"),
        confidence: Confidence::High,
    };
    let said = origin(&card(ai, 2));
    assert!(
        said.starts_with("AI \u{b7} model-a \u{b7} high \u{b7} Both changes kept"),
        "{said}"
    );
    assert!(
        said.contains("2 new lines: Written by the language model: look at it before you accept it"),
        "{said}"
    );
}
