//! The reads of commits: the commit a name stands for, the details of a commit and the lines it changed, a comparison
//! of two sides, and the commits of what is checked out.

use git::read::compare::{CompareRequest, PatchForm};
use model::commit::{BranchLog, Commit};
use model::commit_page::{DiffLine, FilePatch, Hunk, LineKind, PatchLines};
use model::compare::{CommitSide, CompareMode, ComparePatch, CompareView, ComparedCommit, MergeOutlook, Side};
use model::details::{CommitDetail, Signature};
use model::state::{ChangeKind, FileChange};
use serde::{Deserialize, Serialize};

use super::{BytesJson, commit_of, id, ids};
use crate::feature::NotACommit;
use crate::state::{ChangeKindJson, FileChangeJson};

/// The parameters of `read/search.read-revision` and `read/compare.resolve-revision`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionParams {
    /// The repository.
    pub repository: String,
    /// The name, as typed: a hash, a branch, a tag, `HEAD~2`.
    pub revision: String,
}

/// The answer of `read/search.read-revision` and `read/compare.resolve-revision`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionResult {
    /// The commit the name stands for; none for a name git does not know as a commit.
    pub commit: Option<String>,
}

/// Which commit `read/details.read-commit` reads: `{ commit }`, `{ stash }` or `{ uncommitted: true }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommitAskedJson {
    /// A commit, by its full id.
    Commit {
        /// Its id.
        commit: String,
    },
    /// A stash, by its place in the list: 0 for `stash@{0}`.
    Stash {
        /// Its place.
        stash: usize,
    },
    /// The uncommitted changes, as the row above the commits shows them.
    Uncommitted {
        /// Always true.
        uncommitted: bool,
    },
}

/// The parameters of `read/details.read-commit`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetailParams {
    /// The repository.
    pub repository: String,
    /// Which commit.
    #[serde(flatten)]
    pub asked: CommitAskedJson,
}

/// Who made a commit, or committed it, and when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureJson {
    /// The name.
    pub name: String,
    /// The address.
    pub email: String,
    /// When, in seconds since 1970.
    pub date: i64,
}

/// The details of a commit (`CommitDetail` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetailJson {
    /// Its full id; none for the uncommitted changes.
    pub id: Option<String>,
    /// Its parents, the first one first.
    pub parents: Vec<String>,
    /// Who made it; none for the uncommitted changes.
    pub author: Option<SignatureJson>,
    /// Who committed it; none for the uncommitted changes.
    pub committer: Option<SignatureJson>,
    /// Its message, without the line break at its end.
    pub message: String,
    /// The files it changed against its first parent.
    pub files: Vec<FileChangeJson>,
    /// For a stash made with its untracked files, those files.
    pub untracked_files: Vec<FileChangeJson>,
}

impl From<&Signature> for SignatureJson {
    fn from(signature: &Signature) -> SignatureJson {
        SignatureJson {
            name: signature.name.clone(),
            email: signature.email.clone(),
            date: signature.date.0,
        }
    }
}

impl From<&CommitDetail> for CommitDetailJson {
    fn from(detail: &CommitDetail) -> CommitDetailJson {
        CommitDetailJson {
            id: detail.id.as_ref().map(id),
            parents: ids(&detail.parents),
            author: detail.author.as_ref().map(SignatureJson::from),
            committer: detail.committer.as_ref().map(SignatureJson::from),
            message: detail.message.clone(),
            files: detail.files.iter().map(FileChangeJson::from).collect(),
            untracked_files: detail.untracked_files.iter().map(FileChangeJson::from).collect(),
        }
    }
}

/// The parameters of `read/commit-page.read-patch`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitPatchParams {
    /// The repository.
    pub repository: String,
    /// The commit.
    pub commit: String,
    /// Only this file of it, from the top of the working tree; none for every file.
    #[serde(default)]
    pub path: Option<String>,
}

/// The lines that differ: one patch per file, matched to the files listed; nothing when git's output did not match
/// them; or the text of the patch, for a comparison read as text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatchJson {
    /// The lines of each file.
    Files {
        /// One per file, in the order the files are listed.
        files: Vec<FilePatchJson>,
    },
    /// Git's output did not match the files listed, so no lines are shown rather than shown against the wrong files.
    Unmatched,
    /// The patch as git wrote it.
    Text {
        /// Its bytes.
        text: BytesJson,
    },
}

/// The lines of one file of a patch (`FilePatch` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePatchJson {
    /// Its path.
    pub path: String,
    /// The path it had, for a rename or a copy.
    pub original_path: Option<String>,
    /// How it changed.
    pub kind: ChangeKindJson,
    /// Its hunks.
    pub hunks: Vec<HunkJson>,
    /// How many lines were added.
    pub added: usize,
    /// How many lines were removed.
    pub removed: usize,
    /// Whether git found it binary.
    pub is_binary: bool,
    /// Whether its lines are past the cap of a page, and not shown.
    pub is_past_limit: bool,
}

/// A hunk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HunkJson {
    /// Its `@@ … @@` line.
    pub header: String,
    /// Its lines.
    pub lines: Vec<DiffLineJson>,
}

/// A line of a hunk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLineJson {
    /// What it is.
    pub kind: LineKindJson,
    /// Its text, without its first column.
    pub text: String,
    /// Its number in the file before, from 1.
    pub before: Option<usize>,
    /// Its number in the file after, from 1.
    pub after: Option<usize>,
}

/// What a line of a hunk is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs, reason = "each is the kind of line of model that it is named after")]
pub enum LineKindJson {
    Context,
    Added,
    Removed,
    NoNewline,
}

impl From<&PatchLines> for PatchJson {
    fn from(lines: &PatchLines) -> PatchJson {
        match lines {
            PatchLines::Files(files) => PatchJson::Files {
                files: files.iter().map(FilePatchJson::from).collect(),
            },
            PatchLines::Unmatched => PatchJson::Unmatched,
        }
    }
}

impl From<&ComparePatch> for PatchJson {
    fn from(patch: &ComparePatch) -> PatchJson {
        match patch {
            ComparePatch::Lines(lines) => PatchJson::from(lines),
            ComparePatch::Text(text) => PatchJson::Text {
                text: BytesJson::of(text),
            },
        }
    }
}

impl From<&FilePatch> for FilePatchJson {
    fn from(patch: &FilePatch) -> FilePatchJson {
        FilePatchJson {
            path: patch.path.clone(),
            original_path: patch.original_path.clone(),
            kind: ChangeKindJson::from(patch.kind),
            hunks: patch.hunks.iter().map(HunkJson::from).collect(),
            added: patch.added,
            removed: patch.removed,
            is_binary: patch.is_binary,
            is_past_limit: patch.is_past_limit,
        }
    }
}

impl From<&Hunk> for HunkJson {
    fn from(hunk: &Hunk) -> HunkJson {
        HunkJson {
            header: hunk.header.clone(),
            lines: hunk.lines.iter().map(DiffLineJson::from).collect(),
        }
    }
}

impl From<&DiffLine> for DiffLineJson {
    fn from(line: &DiffLine) -> DiffLineJson {
        DiffLineJson {
            kind: match line.kind {
                LineKind::Context => LineKindJson::Context,
                LineKind::Added => LineKindJson::Added,
                LineKind::Removed => LineKindJson::Removed,
                LineKind::NoNewline => LineKindJson::NoNewline,
            },
            text: line.text.clone(),
            before: line.before,
            after: line.after,
        }
    }
}

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

/// The parameters of `read/branch.read-log`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchLogParams {
    /// The repository.
    pub repository: String,
    /// The most commits listed.
    pub count: usize,
}

/// A commit as a list shows it (`Commit` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitJson {
    /// Its full id.
    pub id: String,
    /// Its parents, the first one first.
    pub parents: Vec<String>,
    /// The name of its author.
    pub author_name: String,
    /// The address of its author.
    pub author_email: String,
    /// When its author made it.
    pub author_date: i64,
    /// The first line of its message.
    pub subject: String,
}

impl From<&Commit> for CommitJson {
    fn from(commit: &Commit) -> CommitJson {
        CommitJson {
            id: id(&commit.id),
            parents: ids(&commit.parents),
            author_name: commit.author_name.clone(),
            author_email: commit.author_email.clone(),
            author_date: commit.author_date.0,
            subject: commit.subject.clone(),
        }
    }
}

/// The answer of `read/branch.read-log` (`BranchLog` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchLogJson {
    /// The commits of `HEAD`, newest first.
    pub commits: Vec<CommitJson>,
    /// Whether `HEAD` has more.
    pub has_more: bool,
    /// Those of them not pushed; none when there is no remote to push to.
    pub unpushed: Option<Vec<String>>,
    /// The upstream they are not in, by its short name; none when it is every remote branch.
    pub upstream: Option<String>,
}

impl From<&BranchLog> for BranchLogJson {
    fn from(log: &BranchLog) -> BranchLogJson {
        BranchLogJson {
            commits: log.commits.iter().map(CommitJson::from).collect(),
            has_more: log.has_more,
            unpushed: log.unpushed.as_deref().map(ids),
            upstream: log.upstream.clone(),
        }
    }
}
