//! What each card of the review shows, derived from what was read, the choices and what is accepted: the proposal
//! (`review.proposals`), where its lines come from (`review.proposal-lines`), what looks wrong in it
//! (`review.proposal-warnings`), the lines around it and the columns, numbered as the file would number them with the
//! proposal in place of the conflict.

use std::collections::BTreeMap;
use std::ops::Range;

use model::conflicts::{Side, SideNames, SideRole};
use model::review::{
    AiAnswer, ChoiceKind, JsonFile, Piece, Proposal, ProposalLines, ProposalSource, ProposalsInput, Proposers,
    ReviewConflict, ReviewFile, Shape, Sides, WarningInput, chosen, proposal_lines, proposal_warnings, proposals,
};
use views::review::{
    CardAi, CardProposal, CardShown, ConflictCard, ConflictKey, FileCards, NumberedLine, ProposedLine, SideColumn,
};

use super::{Fingerprinted, ReviewPage};

/// The names of the sides before the page is read.
pub static NO_SIDES: SideNames = SideNames {
    current: Side {
        role: SideRole::BranchCheckedOut,
        name: None,
        subject: None,
    },
    incoming: Side {
        role: SideRole::BranchBeingMerged,
        name: None,
        subject: None,
    },
};

/// How many lines of the file are shown before and after.
const AROUND: usize = 3;

/// A choice made on a conflict, with the lines written for it when it is **Edit**.
pub type Chosen = (Fingerprinted, ChoiceKind, Vec<String>);

/// What is known of the conflicts of the page besides what was read: the choices made, the questions to the AI
/// provider and their answers, and where the AI stands on each conflict.
pub struct Known<'a> {
    /// The choices made, by file and fingerprint.
    pub choices: &'a [Chosen],
    /// The questions to the AI provider and their answers.
    pub questions: &'a super::ai::Questions,
    /// Where the AI stands on each conflict it was asked about.
    pub ai_states: &'a BTreeMap<Fingerprinted, CardAi>,
}

/// The proposal of each conflict of a file: the choice made on it, else what the rules or the AI propose.
pub fn file_proposals(
    file: &ReviewFile,
    (choices, answers): (&[Chosen], &[AiAnswer]),
    (rules, ai): (bool, bool),
) -> Vec<Option<Proposal>> {
    let proposed = proposals(&ProposalsInput {
        file,
        proposers: Proposers { rules, ai },
        answers,
    });
    file.conflicts
        .iter()
        .zip(proposed)
        .map(|(conflict, proposal)| {
            let chosen_kind = choices
                .iter()
                .find(|((path, fingerprint), _, _)| *path == file.path && *fingerprint == conflict.fingerprint)
                .map(|(_, kind, lines)| (*kind, lines));
            chosen_kind.map_or(proposal, |(kind, lines)| Some(chosen(file, conflict, kind, lines)))
        })
        .collect()
}

/// What each card of the page shows.
pub fn cards(page: &ReviewPage, known: &Known<'_>, settings: (bool, bool)) -> Vec<FileCards> {
    let Some(readout) = page.readout.as_deref() else {
        return Vec::new();
    };
    readout
        .read
        .files
        .iter()
        .zip(&readout.side_diffs)
        .map(|(file, side_diffs)| {
            let lines: Vec<&str> = file.lines().into_iter().map(|(text, _)| text).collect();
            let pieces = json_pieces(file, &lines);
            let answers = super::ai::answers_of(known.questions, &page.folder, &file.path);
            let conflicts: Vec<ConflictCard> = file
                .conflicts
                .iter()
                .zip(file_proposals(file, (known.choices, &answers), settings))
                .zip(side_diffs)
                .map(|((conflict, proposal), diffs)| {
                    let key = ConflictKey {
                        path: file.path.clone(),
                        index: conflict.index,
                        fingerprint: conflict.fingerprint.clone(),
                    };
                    let shown = Shown {
                        file,
                        lines: &lines,
                        pieces: pieces.as_deref(),
                    };
                    let ai = known
                        .ai_states
                        .get(&(key.path.clone(), key.fingerprint.clone()))
                        .cloned()
                        .unwrap_or_default();
                    let mut card = card(page, (key, conflict, proposal.as_ref()), &shown, diffs.clone());
                    card.ai = ai;
                    card
                })
                .collect();
            let is_whole = conflicts.iter().all(|card| card.is_accepted);
            FileCards {
                is_folded: page.file_folds.get(&file.path).copied().unwrap_or(is_whole),
                path: file.path.clone(),
                conflicts,
            }
        })
        .collect()
}

/// The file a card is in.
struct Shown<'a> {
    /// The file.
    file: &'a ReviewFile,
    /// Its lines, without their endings.
    lines: &'a [&'a str],
    /// Its pieces, when it is a JSON file.
    pieces: Option<&'a [OwnedPiece]>,
}

/// A piece of a JSON file, owned.
enum OwnedPiece {
    /// Lines outside the conflicts.
    Lines(String),
    /// A conflict, with the text of each side.
    Conflict(String, String),
}

/// The pieces of a JSON file, for `review.proposal-warnings`; none for a file whose name does not end in `.json`.
fn json_pieces(file: &ReviewFile, lines: &[&str]) -> Option<Vec<OwnedPiece>> {
    let is_json = std::path::Path::new(&file.path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if !is_json {
        return None;
    }
    let text = |lines: &[String]| {
        lines
            .iter()
            .flat_map(|line| {
                [
                    line.as_str(),
                    "
",
                ]
            })
            .collect::<String>()
    };
    let between = |range: Range<usize>| {
        lines
            .get(range)
            .unwrap_or_default()
            .iter()
            .flat_map(|line| {
                [
                    *line, "
",
                ]
            })
            .collect::<String>()
    };
    let mut pieces = Vec::new();
    let mut at = 0;
    for conflict in &file.conflicts {
        pieces.push(OwnedPiece::Lines(between(at..conflict.span.start)));
        pieces.push(OwnedPiece::Conflict(text(&conflict.current), text(&conflict.incoming)));
        at = conflict.span.end;
    }
    pieces.push(OwnedPiece::Lines(between(at..lines.len())));
    Some(pieces)
}

/// Numbered lines of the file.
fn numbered(lines: &[&str], range: Range<usize>, first: usize) -> Vec<NumberedLine> {
    lines
        .get(range)
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, text)| NumberedLine {
            number: first + index,
            text: (*text).to_owned(),
        })
        .collect()
}

/// The lines a side has where the proposal goes: the lines the proposal replaces outside the markers, around the
/// side's lines between them.
fn side_lines(lines: &[&str], replaced: &Range<usize>, conflict: &ReviewConflict, side: &[String]) -> Vec<String> {
    let outside = |range: Range<usize>| {
        lines
            .get(range)
            .unwrap_or_default()
            .iter()
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>()
    };
    let mut all = outside(replaced.start.min(conflict.span.start)..conflict.span.start);
    all.extend(side.iter().cloned());
    all.extend(outside(conflict.span.end..replaced.end.max(conflict.span.end)));
    all
}

/// Lines as slices.
fn as_strs(lines: &[String]) -> Vec<&str> {
    lines.iter().map(String::as_str).collect()
}

/// One card.
fn card(
    page: &ReviewPage,
    (key, conflict, proposal): (ConflictKey, &ReviewConflict, Option<&Proposal>),
    shown: &Shown<'_>,
    side_diffs: (model::review::SideDiff, model::review::SideDiff),
) -> ConflictCard {
    let lines = shown.lines;
    let replaced = proposal.map_or_else(|| conflict.span.clone(), |proposal| proposal.replaced.clone());
    let current = side_lines(lines, &replaced, conflict, &conflict.current);
    let incoming = side_lines(lines, &replaced, conflict, &conflict.incoming);
    let base = conflict
        .known_base()
        .map(|base| side_lines(lines, &replaced, conflict, base));
    let before = numbered(
        lines,
        replaced.start.saturating_sub(AROUND)..replaced.start,
        replaced.start.saturating_sub(AROUND) + 1,
    );
    let after_range = replaced.end..(replaced.end + AROUND).min(lines.len());
    let column = |side: &[String]| SideColumn {
        lines: side
            .iter()
            .enumerate()
            .map(|(index, text)| NumberedLine {
                number: replaced.start + 1 + index,
                text: text.clone(),
            })
            .collect(),
        after: numbered(lines, after_range.clone(), replaced.start + side.len() + 1),
    };
    let card_proposal = proposal.map(|proposal| {
        let texts = proposal.texts();
        let (current, incoming) = (as_strs(&current), as_strs(&incoming));
        let base = base.as_deref().map(as_strs);
        let derived = proposal_lines(
            Sides {
                current: &current,
                incoming: &incoming,
                base: base.as_deref(),
            },
            &texts,
        );
        card_proposal(proposal, (&texts, derived), shown, (conflict.index, replaced.start + 1))
    });
    let after_first = replaced.start + card_proposal.as_ref().map_or(0, |proposal| proposal.lines.len()) + 1;
    let fingerprinted = (key.path.clone(), key.fingerprint.clone());
    let is_accepted = proposal.is_some() && page.accepted.contains(&fingerprinted);
    ConflictCard {
        line: conflict.line,
        choices: choices_offered(card_proposal.as_ref()),
        before,
        after: numbered(lines, after_range.clone(), after_first),
        side_diffs: Box::new(side_diffs),
        base: conflict.known_base().map(<[String]>::to_vec),
        current: column(&current),
        incoming: column(&incoming),
        is_accepted,
        shown: CardShown {
            is_folded: page.folds.get(&fingerprinted).copied().unwrap_or(is_accepted),
            is_sides_shown: page.sides_shown.contains(&fingerprinted),
            is_base_shown: page.base_shown.contains(&fingerprinted),
        },
        proposal: card_proposal,
        ai: CardAi::default(),
        key,
    }
}

/// What the card shows of a proposal: its lines numbered from the first line it replaces, where each comes from, and
/// its warnings.
fn card_proposal(
    proposal: &Proposal,
    (texts, derived): (&[&str], ProposalLines),
    shown: &Shown<'_>,
    (index, first): (usize, usize),
) -> CardProposal {
    let is_json = |text: &str| json::parse(text).is_ok();
    let pieces: Option<Vec<Piece<'_>>> = shown.pieces.map(|pieces| {
        pieces
            .iter()
            .map(|piece| match piece {
                OwnedPiece::Lines(text) => Piece::Lines(text),
                OwnedPiece::Conflict(current, incoming) => Piece::Conflict { current, incoming },
            })
            .collect()
    });
    let warnings = proposal_warnings(WarningInput {
        proposed: texts,
        origins: &derived.origins,
        json: pieces.as_deref().map(|pieces| JsonFile {
            pieces,
            conflict: index,
            is_json: &is_json,
        }),
    });
    let warned: Vec<usize> = warnings
        .iter()
        .flat_map(|warning| match warning {
            model::review::ProposalWarning::SetTwice { lines, .. } => lines.clone(),
            model::review::ProposalWarning::NotJson => Vec::new(),
        })
        .collect();
    let _ = shown.file;
    CardProposal {
        source: proposal.source.clone(),
        shape: derived.shape,
        counts: derived.counts,
        lines: texts
            .iter()
            .zip(&derived.origins)
            .enumerate()
            .map(|(place, (text, origin))| ProposedLine {
                line: NumberedLine {
                    number: first + place,
                    text: (*text).to_owned(),
                },
                origin: *origin,
                is_warned: warned.contains(&place),
            })
            .collect(),
        warnings,
        from_current: derived.from_current,
        to_incoming: derived.to_incoming,
    }
}

/// The choices offered on a card: the sides and both of them in each order, but the shape the proposal has, and
/// **Edit**.
fn choices_offered(proposal: Option<&CardProposal>) -> Vec<ChoiceKind> {
    let shape = proposal.map(|proposal| proposal.shape);
    let chosen = proposal.and_then(|proposal| match proposal.source {
        ProposalSource::Choice(kind) => Some(kind),
        ProposalSource::Rules(_) | ProposalSource::Ai { .. } => None,
    });
    [
        (ChoiceKind::CurrentOnly, Shape::Current),
        (ChoiceKind::IncomingOnly, Shape::Incoming),
        (ChoiceKind::BothCurrentFirst, Shape::CurrentFirst),
        (ChoiceKind::BothIncomingFirst, Shape::IncomingFirst),
    ]
    .into_iter()
    .filter(|(kind, kind_shape)| shape != Some(*kind_shape) && chosen != Some(*kind))
    .map(|(kind, _)| kind)
    .chain([ChoiceKind::Written])
    .collect()
}
