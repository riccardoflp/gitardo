//! The JSON of `RepositoryState` (`docs/transition/02-protocol.md`, Types): the fields of `model`, in camelCase, a
//! commit as its full id, a time in seconds, a path as a string.

mod convert;

use serde::{Deserialize, Serialize};

/// What one read of a repository found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateJson {
    /// The top folder of the working tree that was read.
    pub folder: String,
    /// The folder of git's files for this worktree.
    pub git_folder: String,
    /// What is checked out.
    pub head: HeadJson,
    /// The local branches, by name.
    pub branches: Vec<BranchJson>,
    /// The remote branches, by name.
    pub remote_branches: Vec<RemoteBranchJson>,
    /// The tags, by name.
    pub tags: Vec<TagJson>,
    /// The stashes, newest first.
    pub stashes: Vec<StashJson>,
    /// The worktrees, the main one first.
    pub worktrees: Vec<WorktreeJson>,
    /// The names of the remotes.
    pub remotes: Vec<String>,
    /// Who commits are made as.
    pub identity: IdentityJson,
    /// Git's `branch.sort`, when it is set.
    pub branch_sort: Option<String>,
    /// The subjects of the last entries of the reflog of `HEAD`, newest first.
    pub reflog_subjects: Vec<String>,
    /// What is not committed.
    pub working_tree: WorkingTreeJson,
    /// The operation that stopped half way, if one did.
    pub operation: Option<OperationJson>,
    /// Where an interactive rebase stopped to edit or to split a commit.
    pub rebase_stop: Option<RebaseStopJson>,
    /// The bisect under way, if one is.
    pub bisect: Option<BisectJson>,
    /// The submodules, each followed by its own.
    pub submodules: Vec<SubmoduleJson>,
    /// The commits whose parents were not fetched, where the history of a shallow clone ends; empty when the
    /// repository is not shallow (since 1.11).
    #[serde(default)]
    pub shallow: Vec<String>,
}

/// What is checked out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadJson {
    /// The commit; none before the first commit.
    pub commit: Option<String>,
    /// The full name of the branch, `refs/heads/main`; none when `HEAD` is detached.
    pub branch: Option<String>,
}

/// A local branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchJson {
    /// Its short name.
    pub name: String,
    /// The commit it points at.
    pub commit: String,
    /// The branch it follows, when it follows one.
    pub upstream: Option<UpstreamJson>,
    /// Whether it is checked out in the worktree read.
    pub is_checked_out: bool,
    /// The date of its commit.
    pub date: i64,
}

/// The branch a local branch follows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamJson {
    /// Its full name, `refs/remotes/origin/main`.
    pub reference: String,
    /// The remote.
    pub remote: String,
    /// The name of the branch on the remote.
    pub remote_branch: String,
    /// How many commits the local branch has that it does not.
    pub ahead: u32,
    /// How many commits it has that the local branch does not.
    pub behind: u32,
    /// Whether it is gone from the remote.
    pub is_gone: bool,
}

/// A remote branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBranchJson {
    /// Its name with its remote, `origin/main`.
    pub name: String,
    /// The remote.
    pub remote: String,
    /// The commit it points at.
    pub commit: String,
    /// The date of its commit.
    pub date: i64,
}

/// A tag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagJson {
    /// Its name.
    pub name: String,
    /// The commit it points at.
    pub commit: String,
    /// The date of its commit.
    pub date: i64,
}

/// A stash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StashJson {
    /// Its commit.
    pub commit: String,
    /// Its place, `stash@{index}`.
    pub index: usize,
    /// Its message.
    pub message: String,
    /// When it was made.
    pub date: i64,
}

/// A worktree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeJson {
    /// Its folder.
    pub folder: String,
    /// The commit checked out in it; none before the first commit.
    pub commit: Option<String>,
    /// The branch checked out in it; none when it is detached.
    pub branch: Option<String>,
    /// Whether it is the main worktree.
    pub is_main: bool,
    /// Whether it is the one read.
    pub is_current: bool,
    /// Whether its folder is gone.
    pub is_missing: bool,
    /// How many files are not committed in it, when that was read.
    pub uncommitted: Option<usize>,
}

/// Who commits are made as.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityJson {
    /// `user.name`.
    pub name: Option<String>,
    /// `user.email`.
    pub email: Option<String>,
}

/// What is not committed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingTreeJson {
    /// The changes in the index.
    pub staged: Vec<FileChangeJson>,
    /// The changes not in the index.
    pub unstaged: Vec<FileChangeJson>,
    /// The files git does not track.
    pub untracked: Vec<String>,
    /// The files in conflict.
    pub conflicted: Vec<ConflictedFileJson>,
}

/// A file changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeJson {
    /// Its path, from the top of the working tree.
    pub path: String,
    /// The path it had, for a rename or a copy.
    pub original_path: Option<String>,
    /// How it changed.
    pub kind: ChangeKindJson,
}

/// How a file changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs, reason = "each is the kind of change of model that it is named after")]
pub enum ChangeKindJson {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
}

/// A file in conflict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictedFileJson {
    /// Its path.
    pub path: String,
    /// What each side did to it.
    pub kind: ConflictKindJson,
}

/// What each side did to a file in conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(
    missing_docs,
    reason = "each is the kind of conflict of model that it is named after"
)]
pub enum ConflictKindJson {
    BothDeleted,
    AddedByUs,
    DeletedByThem,
    AddedByThem,
    DeletedByUs,
    BothAdded,
    BothModified,
}

/// The operation that stopped half way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs, reason = "each is the operation of model that it is named after")]
pub enum OperationJson {
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Patch,
}

/// Where an interactive rebase stopped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebaseStopJson {
    /// `edit` or `split`.
    pub kind: StopKindJson,
    /// The commit it stopped at.
    pub commit: String,
    /// Its subject.
    pub subject: String,
}

/// Why an interactive rebase stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs, reason = "each is the kind of stop of model that it is named after")]
pub enum StopKindJson {
    Edit,
    Split,
}

/// A bisect under way.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BisectJson {
    /// The commit marked bad.
    pub bad: Option<String>,
    /// The commits marked good.
    pub good: Vec<String>,
    /// The commits skipped.
    pub skipped: Vec<String>,
    /// How many commits are left to test, when git says.
    pub remaining: Option<u32>,
}

/// A submodule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmoduleJson {
    /// Its name in `.gitmodules`.
    pub name: String,
    /// Its folder, from the top of the repository.
    pub folder: String,
    /// The folder of the submodule it is in, for a submodule of a submodule.
    pub parent: Option<String>,
    /// Its URL.
    pub url: Option<String>,
    /// The branch it follows.
    pub branch: Option<String>,
    /// The commit the repository records for it.
    pub recorded: Option<String>,
    /// The commit staged for it.
    pub staged: Option<String>,
    /// What is checked out in it, when it is initialized.
    pub checkout: Option<CheckoutJson>,
}

/// What is checked out in a submodule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutJson {
    /// The commit.
    pub commit: Option<String>,
    /// The branch.
    pub branch: Option<String>,
    /// How many files are not committed in it.
    pub uncommitted: usize,
    /// How far it is from the commit recorded.
    pub distance: Option<DistanceJson>,
}

/// How far two commits are from each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistanceJson {
    /// The commits one has that the other has not.
    pub ahead: u32,
    /// The commits the other has that the one has not.
    pub behind: u32,
}
