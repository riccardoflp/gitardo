//! The JSON of the targets of the conflicts, from version 1.3: the resolutions accepted on the review
//! ([`FileResolutionsJson`], for `review.write`) and the file of a Merge page ([`MergeResultJson`], for
//! `merge-view.mark-resolved`).

use model::ai::Confidence;
use model::merge_view::MergeResolution;
use model::review::{ChoiceKind, FileLine, FileResolutions, Proposal, ProposalSource, RuleCase, split_lines};
use serde::{Deserialize, Serialize};

/// The resolutions accepted in a file of the review (`FileResolutions` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResolutionsJson {
    /// Its path from the top of the working tree.
    pub path: String,
    /// Its text when the proposals were made: the file is written only while it still is.
    pub text: String,
    /// How many conflicts it has.
    pub conflicts: usize,
    /// The proposals accepted.
    pub accepted: Vec<ProposalJson>,
}

/// A proposal accepted for a conflict: the lines of the file it takes the place of, from 0, `from` included and `to`
/// not, and the text put there, each line with what ends it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalJson {
    /// Who proposed it.
    pub source: ProposalSourceJson,
    /// The first line it replaces.
    pub from: usize,
    /// The line after the last one it replaces.
    pub to: usize,
    /// The lines put in their place.
    pub text: String,
    /// How many of them neither side nor the base has.
    pub new_lines: usize,
}

/// Who proposed a resolution (`ProposalSource` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ProposalSourceJson {
    /// The rules, with the case.
    Rules {
        /// The case.
        case: RuleCaseJson,
    },
    /// The AI provider.
    Ai {
        /// The model that answered.
        model: String,
        /// Why, in its words.
        reason: String,
        /// How sure it said it is.
        confidence: ConfidenceJson,
    },
    /// The user's choice.
    Choice {
        /// What was chosen.
        choice: ResolutionChoiceJson,
    },
}

/// The case of the rules (`RuleCase` of `model`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleCaseJson {
    /// Spaces or line endings only.
    SpacesOrLineEndings,
    /// The same change on both sides.
    SameChange,
    /// A change on one side only.
    OneSide,
    /// Changes to lines next to each other.
    NextToEachOther,
}

/// How sure the AI provider said it is (`Confidence` of `model`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidenceJson {
    /// High.
    High,
    /// Medium.
    Medium,
    /// Low.
    Low,
}

/// What the user chose in the review (`ChoiceKind` of `model`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionChoiceJson {
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

/// The file of a Merge page, with its result when it has changes that are not saved (`MergeResolution` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResultJson {
    /// Its path from the top of the working tree.
    pub path: String,
    /// The result not saved; none when the file on disk is the result.
    pub unsaved: Option<String>,
    /// How many conflicts git still marks in it.
    pub marked: usize,
}

impl From<RuleCaseJson> for RuleCase {
    fn from(case: RuleCaseJson) -> RuleCase {
        match case {
            RuleCaseJson::SpacesOrLineEndings => RuleCase::SpacesOrLineEndings,
            RuleCaseJson::SameChange => RuleCase::SameChange,
            RuleCaseJson::OneSide => RuleCase::OneSide,
            RuleCaseJson::NextToEachOther => RuleCase::NextToEachOther,
        }
    }
}

impl From<ConfidenceJson> for Confidence {
    fn from(confidence: ConfidenceJson) -> Confidence {
        match confidence {
            ConfidenceJson::High => Confidence::High,
            ConfidenceJson::Medium => Confidence::Medium,
            ConfidenceJson::Low => Confidence::Low,
        }
    }
}

impl From<ResolutionChoiceJson> for ChoiceKind {
    fn from(choice: ResolutionChoiceJson) -> ChoiceKind {
        match choice {
            ResolutionChoiceJson::CurrentOnly => ChoiceKind::CurrentOnly,
            ResolutionChoiceJson::IncomingOnly => ChoiceKind::IncomingOnly,
            ResolutionChoiceJson::BothCurrentFirst => ChoiceKind::BothCurrentFirst,
            ResolutionChoiceJson::BothIncomingFirst => ChoiceKind::BothIncomingFirst,
            ResolutionChoiceJson::Written => ChoiceKind::Written,
        }
    }
}

impl From<ProposalSourceJson> for ProposalSource {
    fn from(source: ProposalSourceJson) -> ProposalSource {
        match source {
            ProposalSourceJson::Rules { case } => ProposalSource::Rules(case.into()),
            ProposalSourceJson::Ai {
                model,
                reason,
                confidence,
            } => ProposalSource::Ai {
                model,
                reason,
                confidence: confidence.into(),
            },
            ProposalSourceJson::Choice { choice } => ProposalSource::Choice(choice.into()),
        }
    }
}

impl From<ProposalJson> for Proposal {
    fn from(proposal: ProposalJson) -> Proposal {
        Proposal {
            source: proposal.source.into(),
            replaced: proposal.from..proposal.to,
            lines: split_lines(&proposal.text)
                .into_iter()
                .map(|(text, ending)| FileLine::new(text, ending))
                .collect(),
            new_lines: proposal.new_lines,
        }
    }
}

impl From<FileResolutionsJson> for FileResolutions {
    fn from(file: FileResolutionsJson) -> FileResolutions {
        FileResolutions {
            path: file.path,
            text: file.text,
            conflicts: file.conflicts,
            accepted: file.accepted.into_iter().map(Proposal::from).collect(),
        }
    }
}

impl From<MergeResultJson> for MergeResolution {
    fn from(result: MergeResultJson) -> MergeResolution {
        MergeResolution {
            path: result.path,
            unsaved: result.unsaved.map(String::into_bytes),
            marked: result.marked,
        }
    }
}
