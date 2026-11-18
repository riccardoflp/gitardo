//! The JSON of what version 1.10 adds for milestone M22 (`repo.worktree-summary`, D70): what each repository of the
//! workspace, each of its worktrees and each of its submodules is on, for the rows of the extension's Repositories
//! view. `read/repo.worktree-summary` names the folders to follow and answers what each is on; `repository/summaries`
//! says the ones that changed since; `repository/folderChanged` is the extension saying what its watchers saw change
//! in one of them. The read is of many folders, so it takes no `repository`, and opens none.

use model::repo::{SummaryHead, WorktreeSummary};
use serde::{Deserialize, Serialize};

use crate::state::WorktreeJson;

/// The names of the methods of the summaries.
pub mod methods {
    /// `repo.worktree-summary`: the folders to follow, answered with what each is on (version 1.10).
    pub const WORKTREE_SUMMARY: &str = "read/repo.worktree-summary";
    /// The notification of the engine: what some of the folders followed are on, read again.
    pub const SUMMARIES: &str = "repository/summaries";
    /// The notification of the extension: its watchers saw files change in a folder followed.
    pub const FOLDER_CHANGED: &str = "repository/folderChanged";
}

/// The parameters of `read/repo.worktree-summary`: the repositories of the workspace, as `repository/discover` found
/// them. They take the place of the folders followed before.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummariesParams {
    /// The folders of the repositories, in the order the rows list them.
    pub folders: Vec<String>,
}

/// The answer of `read/repo.worktree-summary`, and the parameters of `repository/summaries`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummariesResult {
    /// What each folder is on: every folder followed in the answer, those read again in the notification.
    pub summaries: Vec<FolderSummaryJson>,
}

/// The parameters of `repository/folderChanged`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderChangedParams {
    /// The folder followed.
    pub folder: String,
    /// The paths the watchers saw change, from the top of the working tree, or in the git folder with `.git/`.
    #[serde(default)]
    pub paths: Vec<String>,
}

/// What a folder followed is on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSummaryJson {
    /// The folder of the repository, the worktree or the submodule.
    pub folder: String,
    /// What it is on, as read last; none when no read of it answered yet.
    pub summary: Option<SummaryJson>,
    /// Whether the last read of it failed, and so `summary` is the one before.
    pub is_stale: bool,
    /// The worktrees git lists for its repository, the main one first, when it is one of the folders asked; none for a
    /// worktree the engine follows because one of them lists it, and while git could not list them.
    pub worktrees: Option<Vec<WorktreeJson>>,
}

/// The branch checked out in a folder, how many files are changed there, and how far it is from its upstream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryJson {
    /// What is checked out.
    pub head: SummaryHeadJson,
    /// How many files are changed and not committed, untracked files included.
    pub changed: u32,
    /// The commits to push; none for a branch without an upstream, or detached.
    pub ahead: Option<u32>,
    /// The commits to pull; none for a branch without an upstream, or detached.
    pub behind: Option<u32>,
}

/// What a folder has checked out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SummaryHeadJson {
    /// A branch, by its short name.
    Branch {
        /// The name, `feature/login`.
        name: String,
    },
    /// A commit with no branch.
    Detached {
        /// Its short hash.
        commit: String,
    },
    /// A branch with no commit yet.
    NoCommits,
    /// The folder is gone.
    Missing,
}

impl From<&WorktreeSummary> for SummaryJson {
    fn from(summary: &WorktreeSummary) -> SummaryJson {
        let head = match &summary.head {
            SummaryHead::Branch(name) => SummaryHeadJson::Branch { name: name.clone() },
            SummaryHead::Detached(commit) => SummaryHeadJson::Detached { commit: commit.clone() },
            SummaryHead::NoCommits => SummaryHeadJson::NoCommits,
            SummaryHead::Missing => SummaryHeadJson::Missing,
        };
        SummaryJson {
            head,
            changed: summary.changed,
            ahead: summary.upstream.map(|(ahead, _)| ahead),
            behind: summary.upstream.map(|(_, behind)| behind),
        }
    }
}
