//! The reads of files: which paths git ignores, the two sides of a file of a Diff page, a file as a commit has it, who
//! last changed its lines, the names it had, and where some of its lines are in the commit checked out.

use git::read::diff::{FileVersion, VersionRequest};
use model::blame::{BlameCommit, BlameLine};
use model::diff::DiffSides;
use model::file_history::{CommittedLines, LineRange};
use serde::{Deserialize, Serialize};

use super::{BytesJson, commit_of, id};

/// The parameters of `read/changes.read-ignored`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredParams {
    /// The repository.
    pub repository: String,
    /// The paths, from the top of the working tree, with `/`.
    pub paths: Vec<String>,
}

/// The answer of `read/changes.read-ignored`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredResult {
    /// Those of the paths that git ignores; a tracked file is never ignored.
    pub ignored: Vec<String>,
}

/// A version of a file to read for a side of a Diff page (`FileVersion` of the engine).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FileVersionJson {
    /// No file: the side of a file added or deleted.
    Absent,
    /// The file in the index.
    Index,
    /// The file in a commit.
    Commit {
        /// The commit, by its full id.
        commit: String,
    },
    /// The base of a file in conflict, stage 1 of the index.
    Base,
    /// The file on disk, or the text of its editor when it has edits not saved.
    WorkingTree {
        /// The text not saved; none to read the file on disk.
        #[serde(default)]
        unsaved: Option<BytesJson>,
    },
}

/// One side of a Diff page: a version of a file, by its path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    /// Which version.
    pub version: FileVersionJson,
    /// The file, from the top of the working tree, with `/`, by the name it has in that version.
    pub path: String,
}

/// The parameters of `read/diff.read-sides`: the request of `diff.read-sides`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSidesParams {
    /// The repository.
    pub repository: String,
    /// The left side.
    pub left: VersionJson,
    /// The right side.
    pub right: VersionJson,
}

/// The answer of `read/diff.read-sides`: the bytes of each side; none for a side with no file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSidesJson {
    /// The left side.
    pub left: Option<BytesJson>,
    /// The right side.
    pub right: Option<BytesJson>,
}

impl TryFrom<&VersionJson> for VersionRequest {
    type Error = String;

    fn try_from(side: &VersionJson) -> Result<VersionRequest, String> {
        let version = match &side.version {
            FileVersionJson::Absent => FileVersion::Absent,
            FileVersionJson::Index => FileVersion::Index,
            FileVersionJson::Commit { commit } => {
                FileVersion::Commit(commit_of(commit).map_err(|not| not.to_string())?)
            }
            FileVersionJson::Base => FileVersion::Base,
            FileVersionJson::WorkingTree { unsaved } => FileVersion::WorkingTree {
                unsaved: unsaved
                    .as_ref()
                    .map(BytesJson::bytes)
                    .transpose()
                    .map_err(|not| not.to_string())?,
            },
        };
        Ok(VersionRequest {
            version,
            path: side.path.clone(),
        })
    }
}

impl From<&DiffSides> for DiffSidesJson {
    fn from(sides: &DiffSides) -> DiffSidesJson {
        DiffSidesJson {
            left: sides.left.as_deref().map(BytesJson::of),
            right: sides.right.as_deref().map(BytesJson::of),
        }
    }
}

/// The parameters of `read/file-view.read-revision`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRevisionParams {
    /// The repository.
    pub repository: String,
    /// The file, from the top of the working tree, with `/`.
    pub path: String,
    /// The commit, by any name git understands.
    pub revision: String,
}

/// The parameters of `read/blame.read-line`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameLineParams {
    /// The repository.
    pub repository: String,
    /// The file, from the top of the working tree.
    pub path: String,
    /// The line, from 1.
    pub line: usize,
    /// The text of its editor when it has edits not saved; none to blame the file on disk.
    #[serde(default)]
    pub contents: Option<BytesJson>,
}

/// The parameters of `read/blame.read-file`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameFileParams {
    /// The repository.
    pub repository: String,
    /// The file, from the top of the working tree, by the name it has in the revision when one is given.
    pub path: String,
    /// The commit to blame the file as of; none for the file as it is.
    #[serde(default)]
    pub revision: Option<String>,
    /// The text of its editor when it has edits not saved.
    #[serde(default)]
    pub contents: Option<BytesJson>,
}

/// Who last changed a line: a commit, or a line not committed yet (`BlameLine` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BlameLineJson {
    /// A line not committed yet.
    NotCommitted,
    /// A line of a commit.
    Committed(BlameCommitJson),
}

/// The commit that last changed a line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameCommitJson {
    /// Its full id.
    pub commit: String,
    /// Its author.
    pub author: String,
    /// When its author made it.
    pub author_time: i64,
    /// Its subject.
    pub summary: String,
    /// The commit and the name the file had before it changed the line, for the blame before it.
    pub previous: Option<PreviousVersionJson>,
}

/// The version of a file before the commit that changed a line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviousVersionJson {
    /// The commit before.
    pub commit: String,
    /// The name the file had in it.
    pub path: String,
}

impl From<&BlameLine> for BlameLineJson {
    fn from(line: &BlameLine) -> BlameLineJson {
        match line {
            BlameLine::NotCommitted => BlameLineJson::NotCommitted,
            BlameLine::Committed(commit) => BlameLineJson::Committed(BlameCommitJson::from(commit.as_ref())),
        }
    }
}

impl From<&BlameCommit> for BlameCommitJson {
    fn from(commit: &BlameCommit) -> BlameCommitJson {
        BlameCommitJson {
            commit: id(&commit.id),
            author: commit.author.clone(),
            author_time: commit.author_time.0,
            summary: commit.summary.clone(),
            previous: commit.previous.as_ref().map(|previous| PreviousVersionJson {
                commit: id(&previous.commit),
                path: previous.path.clone(),
            }),
        }
    }
}

/// The parameters of `read/file-history.read-names`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamesParams {
    /// The repository.
    pub repository: String,
    /// The file, by its name now, from the top of the working tree.
    pub path: String,
}

/// The answer of `read/file-history.read-names`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamesResult {
    /// Every name the file had, the name given first.
    pub names: Vec<String>,
}

/// Some lines, from 1, the last one in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineRangeJson {
    /// The first line.
    pub start: usize,
    /// The last line.
    pub end: usize,
}

impl From<LineRange> for LineRangeJson {
    fn from(range: LineRange) -> LineRangeJson {
        LineRangeJson {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<LineRangeJson> for LineRange {
    fn from(range: LineRangeJson) -> LineRange {
        LineRange {
            start: range.start,
            end: range.end,
        }
    }
}

/// The parameters of `read/file-history.read-committed-lines` (`CommittedLinesQuery` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedLinesParams {
    /// The repository.
    pub repository: String,
    /// The file, from the top of the working tree.
    pub path: String,
    /// The name the file had before, when the index has it as a rename.
    #[serde(default)]
    pub renamed_from: Option<String>,
    /// The text of the file as its editor has it, saved or not.
    pub text: String,
    /// The lines of that text.
    pub lines: LineRangeJson,
}

/// Where some lines are in the commit checked out (`CommittedLines` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CommittedLinesJson {
    /// No commit has the file.
    InNoCommit,
    /// The commit checked out has the file.
    Committed {
        /// The commit.
        commit: String,
        /// The name of the file in it.
        path: String,
        /// The lines they come from; none when all were added since.
        lines: Option<LineRangeJson>,
    },
}

impl From<&CommittedLines> for CommittedLinesJson {
    fn from(lines: &CommittedLines) -> CommittedLinesJson {
        match lines {
            CommittedLines::InNoCommit => CommittedLinesJson::InNoCommit,
            CommittedLines::Committed { commit, path, lines } => CommittedLinesJson::Committed {
                commit: id(commit),
                path: path.clone(),
                lines: lines.map(LineRangeJson::from),
            },
        }
    }
}
