//! The reads of a comparison: the two sides compared, what each has that the other has not, the files that differ and
//! the lines that differ.

use git::read::compare::{CompareRequest, PatchForm};
use model::compare::{CommitSide, CompareMode, CompareView, ComparedCommit, MergeOutlook, Side};
use model::state::{ChangeKind, FileChange};
use serde::{Deserialize, Serialize};

use super::{commit_of, id};
use crate::feature::NotACommit;
use crate::state::{ChangeKindJson, FileChangeJson};

/// A side of a comparison: a commit, or the working tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ComparisonSideJson {
    /// A commit, by its full id.
    Commit {
        /// Its id.
        commit: String,
    },
    /// The files on disk, standing for `HEAD` and what is not committed.
    WorkingTree,
}

/// What a comparison shows of the two sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompareModeJson {
    /// What the second side changed since the two diverged: `<first>...<second>`.
    SinceDiverged,
    /// Every difference between the two: `<first> <second>`.
    Everything,
}

/// The parameters of `read/compare.read-sides`: the request of `compare.read-sides`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareSidesParams {
    /// The repository.
    pub repository: String,
    /// The first side.
    pub first: ComparisonSideJson,
    /// The second side.
    pub second: ComparisonSideJson,
    /// What is shown of them.
    pub mode: CompareModeJson,
    /// Only this file or folder, from the top; none for every file.
    #[serde(default)]
    pub path: Option<String>,
}

impl TryFrom<&ComparisonSideJson> for Side {
    type Error = NotACommit;

    fn try_from(side: &ComparisonSideJson) -> Result<Side, NotACommit> {
        Ok(match side {
            ComparisonSideJson::Commit { commit } => Side::Commit(commit_of(commit)?),
            ComparisonSideJson::WorkingTree => Side::WorkingTree,
        })
    }
}

impl From<CompareModeJson> for CompareMode {
    fn from(mode: CompareModeJson) -> CompareMode {
        match mode {
            CompareModeJson::SinceDiverged => CompareMode::SinceDiverged,
            CompareModeJson::Everything => CompareMode::Everything,
        }
    }
}

impl TryFrom<&CompareSidesParams> for CompareRequest {
    type Error = NotACommit;

    fn try_from(params: &CompareSidesParams) -> Result<CompareRequest, NotACommit> {
        Ok(CompareRequest {
            first: Side::try_from(&params.first)?,
            second: Side::try_from(&params.second)?,
            mode: CompareMode::from(params.mode),
            path: params.path.clone(),
        })
    }
}

/// What two sides of a comparison have that the other has not (`CompareView` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareViewJson {
    /// The commit the two diverged from; none when they have no history in common.
    pub merge_base: Option<String>,
    /// How many commits only the first side has.
    pub only_in_first: usize,
    /// How many commits only the second side has.
    pub only_in_second: usize,
    /// The files that differ, as the mode asks.
    pub files: Vec<FileChangeJson>,
    /// The commits of each side the other has not.
    pub commits: Vec<ComparedCommitJson>,
    /// What a merge of the second side into the first would do.
    pub merge: MergeOutlookJson,
}

/// A commit only one side of a comparison has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparedCommitJson {
    /// `first` or `second`: the side that has it.
    pub side: String,
    /// Its full id.
    pub id: String,
    /// The name of its author.
    pub author_name: String,
    /// When its author made it.
    pub author_date: i64,
    /// Its subject.
    pub subject: String,
    /// Whether the other side has the same change under another commit.
    pub is_also_in_other_side: bool,
}

/// What a merge of the second side into the first would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MergeOutlookJson {
    /// Not looked at: a side is the working tree, or the mode does not ask.
    NotAsked,
    /// It would merge without a conflict.
    Clean,
    /// It would leave these files in conflict.
    Conflicts {
        /// The files.
        files: Vec<String>,
    },
    /// Git could not tell.
    Unknown,
}

impl From<&CompareView> for CompareViewJson {
    fn from(view: &CompareView) -> CompareViewJson {
        CompareViewJson {
            merge_base: view.merge_base.as_ref().map(id),
            only_in_first: view.only_in_first,
            only_in_second: view.only_in_second,
            files: view.files.iter().map(FileChangeJson::from).collect(),
            commits: view.commits.iter().map(ComparedCommitJson::from).collect(),
            merge: MergeOutlookJson::from(&view.merge),
        }
    }
}

impl From<&ComparedCommit> for ComparedCommitJson {
    fn from(commit: &ComparedCommit) -> ComparedCommitJson {
        ComparedCommitJson {
            side: String::from(match commit.side {
                CommitSide::First => "first",
                CommitSide::Second => "second",
            }),
            id: id(&commit.id),
            author_name: commit.author_name.clone(),
            author_date: commit.author_date.0,
            subject: commit.subject.clone(),
            is_also_in_other_side: commit.is_also_in_other_side,
        }
    }
}

impl From<&MergeOutlook> for MergeOutlookJson {
    fn from(merge: &MergeOutlook) -> MergeOutlookJson {
        match merge {
            MergeOutlook::NotAsked => MergeOutlookJson::NotAsked,
            MergeOutlook::Clean => MergeOutlookJson::Clean,
            MergeOutlook::Conflicts(files) => MergeOutlookJson::Conflicts { files: files.clone() },
            MergeOutlook::Unknown => MergeOutlookJson::Unknown,
        }
    }
}

/// The parameters of `read/compare.read-files`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareFilesParams {
    /// The repository.
    pub repository: String,
    /// The commit compared from, by any name git understands.
    pub from: String,
    /// The commit compared to.
    pub to: String,
}

/// How `read/compare.read-patch` gives the lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatchFormJson {
    /// As lines matched to the files listed.
    Lines,
    /// As the text git wrote.
    Text,
}

/// The parameters of `read/compare.read-patch`: a comparison, its merge base and its files as `read/compare.read-sides`
/// gave them, and the form of the answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparePatchParams {
    /// The repository.
    pub repository: String,
    /// The first side.
    pub first: ComparisonSideJson,
    /// The second side.
    pub second: ComparisonSideJson,
    /// What is shown of them.
    pub mode: CompareModeJson,
    /// Only this file or folder; none for every file.
    #[serde(default)]
    pub path: Option<String>,
    /// The commit the two diverged from, as `read/compare.read-sides` gave it.
    #[serde(default)]
    pub merge_base: Option<String>,
    /// The files the lines are matched to.
    #[serde(default)]
    pub files: Vec<FileChangeJson>,
    /// Lines or text.
    pub form: PatchFormJson,
}

impl ComparePatchParams {
    /// The comparison it asks for.
    ///
    /// # Errors
    ///
    /// When a side or the merge base is not the full id of a commit.
    pub fn comparison(&self) -> Result<CompareRequest, NotACommit> {
        Ok(CompareRequest {
            first: Side::try_from(&self.first)?,
            second: Side::try_from(&self.second)?,
            mode: CompareMode::from(self.mode),
            path: self.path.clone(),
        })
    }

    /// The form of the answer, as the engine names it.
    #[must_use]
    pub fn patch_form(&self) -> PatchForm {
        match self.form {
            PatchFormJson::Lines => PatchForm::Lines,
            PatchFormJson::Text => PatchForm::Text,
        }
    }
}

impl From<&FileChangeJson> for FileChange {
    fn from(change: &FileChangeJson) -> FileChange {
        FileChange {
            path: change.path.clone(),
            original_path: change.original_path.clone(),
            kind: match change.kind {
                ChangeKindJson::Added => ChangeKind::Added,
                ChangeKindJson::Modified => ChangeKind::Modified,
                ChangeKindJson::Deleted => ChangeKind::Deleted,
                ChangeKindJson::Renamed => ChangeKind::Renamed,
                ChangeKindJson::Copied => ChangeKind::Copied,
                ChangeKindJson::TypeChanged => ChangeKind::TypeChanged,
            },
        }
    }
}
