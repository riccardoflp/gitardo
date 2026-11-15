//! The reads of the conflicts: the versions of a file in conflict, the files in conflict with their stages, what the
//! two sides are, and what the rules make of each conflict.

use std::ops::Range;

use git::read::conflicts::{ConflictFile, StageEntry};
use git::read::merge_view::MergeSides;
use model::conflicts::{Side, SideNames, SideRole};
use serde::{Deserialize, Serialize};
use text::conflicts::{LeftAlone, RuleCase, RuleConflict, RuleResolutions};

use super::{BytesJson, id};
use crate::state::OperationJson;

/// The parameters of `read/merge-view.read-sides`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeSidesParams {
    /// The repository.
    pub repository: String,
    /// The file in conflict, from the top of the working tree.
    pub path: String,
    /// The text of its editor when it has edits not saved, which stands for the result.
    #[serde(default)]
    pub unsaved: Option<BytesJson>,
}

/// The versions of a file in conflict (`MergeSides` of the engine): each empty when that stage has no file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeSidesJson {
    /// The base, stage 1.
    pub base: BytesJson,
    /// The current side, stage 2.
    pub current: BytesJson,
    /// The incoming side, stage 3.
    pub incoming: BytesJson,
    /// The file as it is now, with its markers.
    pub result: BytesJson,
    /// The operation under way.
    pub operation: Option<OperationJson>,
    /// Whether both sides have the file.
    pub has_both_sides: bool,
}

impl From<&MergeSides> for MergeSidesJson {
    fn from(sides: &MergeSides) -> MergeSidesJson {
        MergeSidesJson {
            base: BytesJson::of(&sides.base),
            current: BytesJson::of(&sides.current),
            incoming: BytesJson::of(&sides.incoming),
            result: BytesJson::of(&sides.result),
            operation: sides.operation.map(OperationJson::from),
            has_both_sides: sides.has_both_sides,
        }
    }
}

/// A file in conflict, with the stages the index has of it (`ConflictFile` of the engine).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictFileJson {
    /// Its path.
    pub path: String,
    /// Stage 1, the base; none when the file is new on both sides.
    pub base: Option<StageEntryJson>,
    /// Stage 2, the current side; none when it deleted the file.
    pub current: Option<StageEntryJson>,
    /// Stage 3, the incoming side; none when it deleted the file.
    pub incoming: Option<StageEntryJson>,
}

/// A stage of a file in the index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageEntryJson {
    /// Its mode: `100644`.
    pub mode: String,
    /// The id of its blob.
    pub object: String,
}

impl From<&ConflictFile> for ConflictFileJson {
    fn from(file: &ConflictFile) -> ConflictFileJson {
        let stage = |entry: &StageEntry| StageEntryJson {
            mode: entry.mode.clone(),
            object: id(&entry.object),
        };
        ConflictFileJson {
            path: file.path.clone(),
            base: file.base.as_ref().map(stage),
            current: file.current.as_ref().map(stage),
            incoming: file.incoming.as_ref().map(stage),
        }
    }
}

/// What the two sides of the conflicts are (`SideNames` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SideNamesJson {
    /// The current side.
    pub current: SideJson,
    /// The incoming side.
    pub incoming: SideJson,
}

/// One side of the conflicts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SideJson {
    /// What it is in the operation under way.
    pub role: SideRoleJson,
    /// Its name: a branch, or a short commit; none when it has none.
    pub name: Option<String>,
    /// The subject of its commit, when it is one.
    pub subject: Option<String>,
}

/// What a side of the conflicts is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs, reason = "each is the role of model that it is named after")]
pub enum SideRoleJson {
    BranchCheckedOut,
    BranchBeingMerged,
    RebasedOnto,
    CommitBeingReplayed,
    CommitBeingPicked,
    StateBeforeReverted,
    PatchBeingApplied,
    YourFiles,
    ChangesBeingApplied,
}

impl From<&SideNames> for SideNamesJson {
    fn from(names: &SideNames) -> SideNamesJson {
        SideNamesJson {
            current: SideJson::from(&names.current),
            incoming: SideJson::from(&names.incoming),
        }
    }
}

impl From<&Side> for SideJson {
    fn from(side: &Side) -> SideJson {
        SideJson {
            role: match side.role {
                SideRole::BranchCheckedOut => SideRoleJson::BranchCheckedOut,
                SideRole::BranchBeingMerged => SideRoleJson::BranchBeingMerged,
                SideRole::RebasedOnto => SideRoleJson::RebasedOnto,
                SideRole::CommitBeingReplayed => SideRoleJson::CommitBeingReplayed,
                SideRole::CommitBeingPicked => SideRoleJson::CommitBeingPicked,
                SideRole::StateBeforeReverted => SideRoleJson::StateBeforeReverted,
                SideRole::PatchBeingApplied => SideRoleJson::PatchBeingApplied,
                SideRole::YourFiles => SideRoleJson::YourFiles,
                SideRole::ChangesBeingApplied => SideRoleJson::ChangesBeingApplied,
            },
            name: side.name.clone(),
            subject: side.subject.clone(),
        }
    }
}

/// The parameters of `read/conflicts.resolve-by-rules`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesParams {
    /// The repository.
    pub repository: String,
    /// Only these files in conflict; none for every one.
    #[serde(default)]
    pub paths: Option<Vec<String>>,
}

/// What the rules make of the conflicts of one file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictProposalsJson {
    /// The file.
    pub path: String,
    /// Why the rules did not look into it; none when they did.
    pub left_alone: Option<LeftAloneJson>,
    /// Each conflict marked in it, in its order, with what the rules propose for it.
    pub conflicts: Vec<RuleConflictJson>,
}

/// Why the rules did not look into a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs, reason = "each is the reason of text that it is named after")]
pub enum LeftAloneJson {
    TooLarge,
    NoMarkers,
    Binary,
    NotUtf8,
    TooManyLines,
    MarkersDoNotPair,
    TooComplex,
}

/// A conflict marked in a file, with what the rules propose for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleConflictJson {
    /// The line of its `<<<<<<<`, from 1.
    pub line: usize,
    /// Its lines in the file, from its `<<<<<<<` to its `>>>>>>>`, from 0, the end left out.
    pub span: SpanJson,
    /// What the rules propose; none when no rule takes it.
    pub proposal: Option<ProposalJson>,
}

/// Lines of a file, from 0, the end left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanJson {
    /// The first line.
    pub start: usize,
    /// The line after the last.
    pub end: usize,
}

/// What a rule proposes for a conflict: lines in place of some lines of the file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalJson {
    /// The rule that took it.
    pub case: RuleCaseJson,
    /// The lines of the file replaced.
    pub replaced: SpanJson,
    /// The lines in their place.
    pub lines: Vec<ProposalLineJson>,
}

/// A line proposed, with what ends it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalLineJson {
    /// Its text.
    pub text: String,
    /// `\n`, `\r\n`, or nothing for the last line of a file without one.
    pub ending: String,
}

/// The rule that took a conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleCaseJson {
    /// The sides differ only in spaces or line endings.
    SpacesOrLineEndings,
    /// Both sides made the same change.
    SameChange,
    /// Only one side changed the base.
    OneSide,
    /// The sides changed lines next to each other.
    NextToEachOther,
}

/// A range of lines as the JSON has it.
fn span(range: &Range<usize>) -> SpanJson {
    SpanJson {
        start: range.start,
        end: range.end,
    }
}

/// What the rules made of a file, as the JSON has it.
#[must_use]
pub fn proposals_of(path: &str, resolutions: &RuleResolutions<'_>) -> ConflictProposalsJson {
    match resolutions {
        RuleResolutions::LeftAlone(reason) => ConflictProposalsJson {
            path: path.to_owned(),
            left_alone: Some(LeftAloneJson::from(*reason)),
            conflicts: Vec::new(),
        },
        RuleResolutions::LookedInto(conflicts) => ConflictProposalsJson {
            path: path.to_owned(),
            left_alone: None,
            conflicts: conflicts.iter().map(RuleConflictJson::from).collect(),
        },
    }
}

impl From<LeftAlone> for LeftAloneJson {
    fn from(reason: LeftAlone) -> LeftAloneJson {
        match reason {
            LeftAlone::TooLarge => LeftAloneJson::TooLarge,
            LeftAlone::NoMarkers => LeftAloneJson::NoMarkers,
            LeftAlone::Binary => LeftAloneJson::Binary,
            LeftAlone::NotUtf8 => LeftAloneJson::NotUtf8,
            LeftAlone::TooManyLines => LeftAloneJson::TooManyLines,
            LeftAlone::MarkersDoNotPair => LeftAloneJson::MarkersDoNotPair,
            LeftAlone::TooComplex => LeftAloneJson::TooComplex,
        }
    }
}

impl From<&RuleConflict<'_>> for RuleConflictJson {
    fn from(conflict: &RuleConflict<'_>) -> RuleConflictJson {
        RuleConflictJson {
            line: conflict.conflict.line,
            span: span(&conflict.conflict.span),
            proposal: conflict.resolution.as_ref().map(|resolution| ProposalJson {
                case: match resolution.case {
                    RuleCase::SpacesOrLineEndings => RuleCaseJson::SpacesOrLineEndings,
                    RuleCase::SameChange => RuleCaseJson::SameChange,
                    RuleCase::OneSide => RuleCaseJson::OneSide,
                    RuleCase::NextToEachOther => RuleCaseJson::NextToEachOther,
                },
                replaced: span(&resolution.replaced),
                lines: resolution
                    .lines
                    .iter()
                    .map(|line| ProposalLineJson {
                        text: line.text.to_owned(),
                        ending: line.ending.to_owned(),
                    })
                    .collect(),
            }),
        }
    }
}
