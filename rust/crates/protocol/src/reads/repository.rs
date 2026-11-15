//! The reads of what a repository holds besides its commits: the reflog, the remotes, the identity, the worktrees and
//! what new ones get, the pull requests, the common git folder and the message git prepared.

use std::collections::BTreeMap;
use std::path::PathBuf;

use git::read::identity::IdentityState;
use git::read::remote::Remote;
use model::github::{Checks, PullRequest, PullRequests};
use model::reflog::ReflogPosition;
use model::worktree_setup::{
    CommandProposal, Ignored, IgnoredEntry, IgnoredKind, PathStatus, PatternTake, SetupPatterns, SetupPreview,
    TakeRequest, Tracked,
};
use serde::{Deserialize, Serialize};

use super::id;
use crate::state::IdentityJson;

/// The parameters of `read/reflog.read`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflogParams {
    /// The repository.
    pub repository: String,
    /// The reference, in full, or `HEAD`.
    pub reference: String,
    /// How many positions, at most.
    pub count: usize,
}

/// A position of a reference in its reflog (`ReflogPosition` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflogPositionJson {
    /// The commit it was at.
    pub commit: String,
    /// What moved it there, as git wrote it.
    pub subject: String,
    /// Its selector: `HEAD@{2}`.
    pub selector: String,
    /// When it moved there.
    pub time: i64,
}

impl From<&ReflogPosition> for ReflogPositionJson {
    fn from(position: &ReflogPosition) -> ReflogPositionJson {
        ReflogPositionJson {
            commit: id(&position.commit),
            subject: position.subject.clone(),
            selector: position.selector.clone(),
            time: position.time.0,
        }
    }
}

/// A remote with the URL it is fetched from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteJson {
    /// Its name.
    pub name: String,
    /// Its URL, as git uses it.
    pub url: String,
}

impl From<&Remote> for RemoteJson {
    fn from(remote: &Remote) -> RemoteJson {
        RemoteJson {
            name: remote.name.clone(),
            url: remote.url.clone(),
        }
    }
}

/// Who commits are made as, and the `user.*` keys the repository sets for itself (`IdentityKeys` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityKeysJson {
    /// The name and the address git will use.
    pub identity: IdentityJson,
    /// The `user.*` keys of the repository's own configuration.
    pub local_keys: Vec<String>,
}

impl From<&IdentityState> for IdentityKeysJson {
    fn from(state: &IdentityState) -> IdentityKeysJson {
        IdentityKeysJson {
            identity: IdentityJson {
                name: state.identity.name.clone(),
                email: state.identity.email.clone(),
            },
            local_keys: state.local_keys.clone(),
        }
    }
}

/// The parameters of `read/worktree.read-folder-links` and `read/worktree-setup.read-ignored`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeParams {
    /// The repository.
    pub repository: String,
    /// The folder of the worktree read.
    pub worktree: String,
}

/// An answer that is a list of paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathsResult {
    /// The paths.
    pub paths: Vec<String>,
}

/// What git ignores at the top of a checkout (`IgnoredEntry` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredEntryJson {
    /// Its path from the top, with `/`.
    pub path: String,
    /// What it is, with its size.
    pub kind: IgnoredKindJson,
}

/// What an ignored path is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum IgnoredKindJson {
    /// A file.
    File {
        /// Its size.
        bytes: u64,
    },
    /// A folder.
    Folder {
        /// How many files were counted in it.
        files: usize,
        /// Their size.
        bytes: u64,
        /// Whether the count stopped before the end.
        has_more: bool,
    },
    /// A link.
    Link,
}

impl From<&IgnoredEntry> for IgnoredEntryJson {
    fn from(entry: &IgnoredEntry) -> IgnoredEntryJson {
        IgnoredEntryJson {
            path: entry.path.clone(),
            kind: match &entry.kind {
                IgnoredKind::File { bytes } => IgnoredKindJson::File { bytes: *bytes },
                IgnoredKind::Folder(size) => IgnoredKindJson::Folder {
                    files: size.files,
                    bytes: size.bytes,
                    has_more: size.has_more,
                },
                IgnoredKind::Link => IgnoredKindJson::Link,
            },
        }
    }
}

/// What link and copy patterns are asked of a checkout (`TakeRequest` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeRequestJson {
    /// The top folder of the checkout.
    pub folder: String,
    /// The patterns of the folders to link.
    pub link: Vec<String>,
    /// The patterns of the paths to copy.
    pub copy: Vec<String>,
    /// The commit a new worktree will check out, whose files decide what is tracked; none for the checkout as it is.
    #[serde(default)]
    pub commit: Option<String>,
}

impl From<&TakeRequestJson> for TakeRequest {
    fn from(request: &TakeRequestJson) -> TakeRequest {
        TakeRequest {
            folder: PathBuf::from(&request.folder),
            patterns: SetupPatterns {
                link: request.link.clone(),
                copy: request.copy.clone(),
            },
            commit: request.commit.clone(),
        }
    }
}

/// The parameters of `read/worktree-setup.read-what-patterns-take`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupTakesParams {
    /// The repository.
    pub repository: String,
    /// One request per checkout.
    pub requests: Vec<TakeRequestJson>,
}

/// What the patterns take from a checkout (`SetupPreview` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupPreviewJson {
    /// The folders linked.
    pub linked: Vec<String>,
    /// The paths copied.
    pub copied: Vec<String>,
    /// What each link pattern takes.
    pub link_patterns: Vec<PatternTakeJson>,
    /// What each copy pattern takes.
    pub copy_patterns: Vec<PatternTakeJson>,
    /// The paths a pattern names that the worktree has already.
    pub already: Vec<String>,
}

/// What one pattern takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternTakeJson {
    /// The pattern.
    pub pattern: String,
    /// How many paths it takes.
    pub count: usize,
    /// The first of them.
    pub paths: Vec<String>,
}

impl From<&SetupPreview> for SetupPreviewJson {
    fn from(preview: &SetupPreview) -> SetupPreviewJson {
        let takes = |takes: &[PatternTake]| {
            takes
                .iter()
                .map(|take| PatternTakeJson {
                    pattern: take.pattern.clone(),
                    count: take.count,
                    paths: take.paths.clone(),
                })
                .collect()
        };
        SetupPreviewJson {
            linked: preview.linked.clone(),
            copied: preview.copied.clone(),
            link_patterns: takes(&preview.link_patterns),
            copy_patterns: takes(&preview.copy_patterns),
            already: preview.already.clone(),
        }
    }
}

/// The answer of `read/worktree-setup.propose-commands`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandsResult {
    /// The commands proposed, each with the file that calls for it.
    pub commands: Vec<CommandProposalJson>,
}

/// A setup command proposed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandProposalJson {
    /// The command: `npm ci`.
    pub command: String,
    /// The file committed at the top that calls for it: `package-lock.json`.
    pub file: String,
}

impl From<&CommandProposal> for CommandProposalJson {
    fn from(proposal: &CommandProposal) -> CommandProposalJson {
        CommandProposalJson {
            command: proposal.command.clone(),
            file: proposal.file.clone(),
        }
    }
}

/// The parameters of `read/worktree-setup.explain-change`: some paths of the working tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathsParams {
    /// The repository.
    pub repository: String,
    /// The paths, from the top, with `/`.
    pub paths: Vec<String>,
}

/// What a path is for git (`PathStatus` of `model`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathStatusJson {
    /// Whether it is a folder on disk.
    pub is_folder: bool,
    /// What git tracks of it.
    pub tracked: TrackedJson,
    /// How much of it git ignores.
    pub ignored: IgnoredHowJson,
}

/// What git tracks of a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackedJson {
    /// Nothing.
    Nothing,
    /// The file itself.
    It,
    /// Files in the folder.
    FilesIn,
}

/// How much of a path git ignores.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IgnoredHowJson {
    /// All of it.
    Whole,
    /// Some of what is in it.
    Part,
    /// Nothing.
    Nothing,
}

impl From<&PathStatus> for PathStatusJson {
    fn from(status: &PathStatus) -> PathStatusJson {
        PathStatusJson {
            is_folder: status.is_folder,
            tracked: match status.tracked {
                Tracked::Nothing => TrackedJson::Nothing,
                Tracked::It => TrackedJson::It,
                Tracked::FilesIn => TrackedJson::FilesIn,
            },
            ignored: match status.ignored {
                Ignored::Whole => IgnoredHowJson::Whole,
                Ignored::Part => IgnoredHowJson::Part,
                Ignored::Nothing => IgnoredHowJson::Nothing,
            },
        }
    }
}

/// The parameters of `read/github.read-pull-requests`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestsParams {
    /// The repository.
    pub repository: String,
    /// The remote whose repository on GitHub is read; none for the main remote (`github.detect-remote`).
    #[serde(default)]
    pub remote: Option<String>,
}

/// The answer of `read/github.read-pull-requests`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PullRequestsJson {
    /// The open pull requests, by the name of their branch.
    Read {
        /// The pull requests.
        by_branch: BTreeMap<String, PullRequestJson>,
    },
    /// The remote is not on GitHub, or there is none.
    NotOnGitHub,
    /// The host gave no token.
    NoToken,
    /// GitHub could not be asked, or refused.
    Failed {
        /// What went wrong, in words.
        message: String,
        /// Whether GitHub refused the token.
        is_unauthorized: bool,
    },
}

/// An open pull request (`PullRequest` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestJson {
    /// Its number.
    pub number: u64,
    /// Its title.
    pub title: String,
    /// Its page on GitHub.
    pub url: String,
    /// Whether it is a draft.
    pub is_draft: bool,
    /// Its branch.
    pub branch: String,
    /// The checks of its last commit; none when it has none.
    pub checks: Option<ChecksJson>,
}

/// The checks of the last commit of a pull request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(
    missing_docs,
    reason = "each is the state of the checks of model that it is named after"
)]
pub enum ChecksJson {
    Passed,
    Failed,
    Running,
}

impl From<&PullRequests> for PullRequestsJson {
    fn from(pulls: &PullRequests) -> PullRequestsJson {
        PullRequestsJson::Read {
            by_branch: pulls
                .by_branch
                .iter()
                .map(|(branch, pull)| (branch.clone(), PullRequestJson::from(pull)))
                .collect(),
        }
    }
}

impl From<&PullRequest> for PullRequestJson {
    fn from(pull: &PullRequest) -> PullRequestJson {
        PullRequestJson {
            number: pull.number,
            title: pull.title.clone(),
            url: pull.url.clone(),
            is_draft: pull.is_draft,
            branch: pull.branch.clone(),
            checks: pull.checks.map(|checks| match checks {
                Checks::Passed => ChecksJson::Passed,
                Checks::Failed => ChecksJson::Failed,
                Checks::Running => ChecksJson::Running,
            }),
        }
    }
}

/// The answer of `read/repo.common-folder`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonFolderResult {
    /// The git folder the worktrees share, in full.
    pub folder: String,
}

/// The answer of `read/commit.prepared-message`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedMessageResult {
    /// The message git prepared; none when no squash merge waits.
    pub message: Option<String>,
}

/// The parameters of the host question `host/githubToken`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubTokenParams {
    /// The host whose token is asked: `github.com`.
    pub host: String,
    /// Whether the user may be asked to sign in; a read never asks.
    pub ask: bool,
}

/// The answer of the host question `host/githubToken`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubTokenResult {
    /// The token; none when VS Code has none, or the user backed out.
    #[serde(default)]
    pub token: Option<String>,
}
