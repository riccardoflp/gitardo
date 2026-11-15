//! The JSON of a change of a diff as a target (1.7): the band of a Diff page that **Stage change** and **Unstage
//! change** are run on, which the extension's Flow Diff gives for the band of its gutter.

use model::diff::{DiffChange, DiffSide};
use serde::{Deserialize, Serialize};

use super::NotACommit;

/// A change of a diff: its file, the two versions it is between, its lines on each side and their text as the page
/// showed them, so that the engine tells whether the file changed since (`DiffChange` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffChangeJson {
    /// The path of the file, from the top of the working tree, with `/`.
    pub path: String,
    /// The version on the left.
    pub left: DiffSideJson,
    /// The version on the right.
    pub right: DiffSideJson,
    /// Its lines on the left.
    pub left_lines: LinesJson,
    /// Its lines on the right.
    pub right_lines: LinesJson,
    /// The text of its lines on the left, without their line breaks.
    pub left_text: Vec<String>,
    /// The text of its lines on the right.
    pub right_text: Vec<String>,
}

/// A version of a file a diff shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DiffSideJson {
    /// What is checked out.
    Head,
    /// The index.
    Index,
    /// The working tree.
    WorkingTree,
    /// A commit, by its full id.
    Commit {
        /// Its id.
        commit: String,
    },
    /// No version.
    Empty,
}

/// Lines of a side, counted from 0, the end left out: an empty range is the place meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinesJson {
    /// The first line.
    pub start: usize,
    /// The line after the last.
    pub end: usize,
}

impl TryFrom<DiffSideJson> for DiffSide {
    type Error = NotACommit;

    fn try_from(side: DiffSideJson) -> Result<DiffSide, NotACommit> {
        Ok(match side {
            DiffSideJson::Head => DiffSide::Head,
            DiffSideJson::Index => DiffSide::Index,
            DiffSideJson::WorkingTree => DiffSide::WorkingTree,
            DiffSideJson::Commit { commit } => {
                DiffSide::Commit(model::CommitId::parse(&commit).ok_or(NotACommit(commit))?)
            }
            DiffSideJson::Empty => DiffSide::Empty,
        })
    }
}

impl TryFrom<DiffChangeJson> for DiffChange {
    type Error = NotACommit;

    fn try_from(change: DiffChangeJson) -> Result<DiffChange, NotACommit> {
        Ok(DiffChange {
            path: change.path,
            left: change.left.try_into()?,
            right: change.right.try_into()?,
            left_lines: change.left_lines.start..change.left_lines.end,
            right_lines: change.right_lines.start..change.right_lines.end,
            left_text: change.left_text,
            right_text: change.right_text,
        })
    }
}
