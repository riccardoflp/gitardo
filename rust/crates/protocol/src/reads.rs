//! The reads of version 1.4, from stage 5 of the transition, those of the graph that version 1.5 adds in stage 7, and
//! `repo.watch-changes` with the pages of the commits sent as they are read, which version 1.8 adds in milestone M15:
//! every `read/<card>` of `docs/transition/02-protocol.md` (Reads), each with its parameters and its answer, and the
//! conversion of the engine's types to their JSON. A read never changes the repository; the server answers each on a thread of its own.
//!
//! The parameters of each read carry `repository` and the fields of the card's request; the answers are the JSON of
//! the types the card's function gives.

mod bytes;
mod commit_box;
mod commits;
mod compare;
mod conflicts;
mod files;
mod graph;
mod repository;

pub use bytes::{BytesJson, NotBase64};
pub use commit_box::{AuthorJson, AuthorsResult, CommitSignatureJson, SignatureParams, VerdictJson};
pub use commits::{
    BranchLogJson, BranchLogParams, CommitAskedJson, CommitDetailJson, CommitDetailParams, CommitJson,
    CommitPatchParams, DiffLineJson, FilePatchJson, HunkJson, LineKindJson, PatchJson, RevisionParams, RevisionResult,
    SignatureJson,
};
pub use compare::{
    CompareFilesParams, CompareModeJson, ComparePatchParams, CompareSidesParams, CompareViewJson, ComparedCommitJson,
    ComparisonSideJson, MergeOutlookJson, PatchFormJson,
};
pub use conflicts::{
    ConflictFileJson, ConflictProposalsJson, LeftAloneJson, MergeSidesJson, MergeSidesParams, ProposalJson,
    ProposalLineJson, RuleCaseJson, RuleConflictJson, RulesParams, SideJson, SideNamesJson, SideRoleJson, SpanJson,
    StageEntryJson, proposals_of,
};
pub use files::{
    BlameCommitJson, BlameFileParams, BlameLineJson, BlameLineParams, CommittedLinesJson, CommittedLinesParams,
    DiffSidesJson, DiffSidesParams, FileRevisionParams, FileVersionJson, IgnoredParams, IgnoredResult, LineRangeJson,
    NamesParams, NamesResult, PreviousVersionJson, VersionJson,
};
pub use graph::{
    BranchesShownJson, CommitPageJson, CommitsParams, GraphRowsParams, MatchesJson, MatchesParams, NarrowingJson,
    ParentsJson, PositionParams, PositionResult, RowJson, SearchModeJson, TimelineJson,
};
pub use repository::{
    ChecksJson, CommandProposalJson, CommandsResult, CommonFolderResult, GitHubTokenParams, GitHubTokenResult,
    IdentityKeysJson, IgnoredEntryJson, IgnoredHowJson, IgnoredKindJson, PathStatusJson, PathsParams, PathsResult,
    PatternTakeJson, PreparedMessageResult, PullRequestJson, PullRequestsJson, PullRequestsParams, ReflogParams,
    ReflogPositionJson, RemoteJson, SetupPreviewJson, SetupTakesParams, TakeRequestJson, TrackedJson, WorktreeParams,
};

use model::CommitId;

use crate::feature::NotACommit;

/// The names of the methods of the reads, `read/<identifier of the card>`.
pub mod methods {
    /// `search.read-revision`: the commit a name stands for.
    pub const SEARCH_REVISION: &str = "read/search.read-revision";
    /// `details.read-commit`: the details of a commit, a stash or the uncommitted changes.
    pub const DETAILS_COMMIT: &str = "read/details.read-commit";
    /// `commit-page.read-patch`: the lines a commit changed.
    pub const COMMIT_PAGE_PATCH: &str = "read/commit-page.read-patch";
    /// `changes.read-working-tree`: what is not committed.
    pub const CHANGES_WORKING_TREE: &str = "read/changes.read-working-tree";
    /// `changes.read-ignored`: which of some paths git ignores.
    pub const CHANGES_IGNORED: &str = "read/changes.read-ignored";
    /// `diff.read-sides`: the two sides of a file of a Diff page.
    pub const DIFF_SIDES: &str = "read/diff.read-sides";
    /// `file-view.read-revision`: a file as a commit has it.
    pub const FILE_VIEW_REVISION: &str = "read/file-view.read-revision";
    /// `merge-view.read-sides`: the versions of a file in conflict.
    pub const MERGE_VIEW_SIDES: &str = "read/merge-view.read-sides";
    /// `conflicts.read-files`: the files in conflict, with their stages.
    pub const CONFLICTS_FILES: &str = "read/conflicts.read-files";
    /// `conflicts.side-names`: what the two sides of the conflicts are.
    pub const CONFLICTS_SIDE_NAMES: &str = "read/conflicts.side-names";
    /// `conflicts.resolve-by-rules`: what the rules make of the conflicts.
    pub const CONFLICTS_RULES: &str = "read/conflicts.resolve-by-rules";
    /// `compare.read-sides`: what two sides of a comparison have that the other has not.
    pub const COMPARE_SIDES: &str = "read/compare.read-sides";
    /// `compare.read-files`: the files that differ between two commits.
    pub const COMPARE_FILES: &str = "read/compare.read-files";
    /// `compare.read-patch`: the lines that differ in a comparison.
    pub const COMPARE_PATCH: &str = "read/compare.read-patch";
    /// `compare.resolve-revision`: the commit a side of a comparison names.
    pub const COMPARE_REVISION: &str = "read/compare.resolve-revision";
    /// `blame.read-line`: who last changed a line.
    pub const BLAME_LINE: &str = "read/blame.read-line";
    /// `blame.read-file`: who last changed each line of a file.
    pub const BLAME_FILE: &str = "read/blame.read-file";
    /// `file-history.read-names`: every name a file had.
    pub const FILE_HISTORY_NAMES: &str = "read/file-history.read-names";
    /// `file-history.read-committed-lines`: where some lines are in the commit checked out.
    pub const FILE_HISTORY_COMMITTED_LINES: &str = "read/file-history.read-committed-lines";
    /// `reflog.read`: the last positions of a reference.
    pub const REFLOG: &str = "read/reflog.read";
    /// `remote.read-remotes`: the remotes with their URLs.
    pub const REMOTES: &str = "read/remote.read-remotes";
    /// `identity.read`: who commits are made as.
    pub const IDENTITY: &str = "read/identity.read";
    /// `submodule.read-list`: the submodules.
    pub const SUBMODULES: &str = "read/submodule.read-list";
    /// `worktree.read-folder-links`: the links to folders in a worktree.
    pub const WORKTREE_FOLDER_LINKS: &str = "read/worktree.read-folder-links";
    /// `worktree-setup.read-ignored`: what git ignores in a worktree, with its sizes.
    pub const SETUP_IGNORED: &str = "read/worktree-setup.read-ignored";
    /// `worktree-setup.read-what-patterns-take`: what link and copy patterns take.
    pub const SETUP_TAKES: &str = "read/worktree-setup.read-what-patterns-take";
    /// `worktree-setup.propose-commands`: the setup commands the files at the top call for.
    pub const SETUP_COMMANDS: &str = "read/worktree-setup.propose-commands";
    /// `worktree-setup.explain-change`: what some paths are for git.
    pub const SETUP_PATHS: &str = "read/worktree-setup.explain-change";
    /// `github.read-pull-requests`: the open pull requests.
    pub const PULL_REQUESTS: &str = "read/github.read-pull-requests";
    /// `branch.read-log`: the commits of what is checked out.
    pub const BRANCH_LOG: &str = "read/branch.read-log";
    /// `repo.common-folder`: the git folder the worktrees share.
    pub const COMMON_FOLDER: &str = "read/repo.common-folder";
    /// `commit.prepared-message`: the message git prepared.
    pub const PREPARED_MESSAGE: &str = "read/commit.prepared-message";
    /// `graph.read-commits`: a page of the commits the timeline lists (version 1.5).
    pub const GRAPH_COMMITS: &str = "read/graph.read-commits";
    /// `graph.read-position`: where a commit is in the timeline (version 1.5).
    pub const GRAPH_POSITION: &str = "read/graph.read-position";
    /// `search.read-matches`: what a search of the whole history finds (version 1.5).
    pub const SEARCH_MATCHES: &str = "read/search.read-matches";
    /// `repo.watch-changes`: which files of a burst ask for a read, those git ignores and those of nested worktrees
    /// left out (version 1.8).
    pub const WATCH_CHANGES: &str = "read/repo.watch-changes";
    /// `details.read-signature`: what git says of the signature of a commit (version 1.13).
    pub const DETAILS_SIGNATURE: &str = "read/details.read-signature";
    /// `commit.read-authors`: the people who made commits in the repository (version 1.13).
    pub const COMMIT_AUTHORS: &str = "read/commit.read-authors";
    /// The notification of a page of `read/graph.read-commits` asked with `stream`, as it is read (version 1.8).
    pub const GRAPH_ROWS: &str = "read/graph.rows";

    /// Every read of version 1.13, in the order of the table of the protocol.
    pub const ALL: [&str; 38] = [
        GRAPH_COMMITS,
        GRAPH_POSITION,
        SEARCH_MATCHES,
        SEARCH_REVISION,
        DETAILS_COMMIT,
        COMMIT_PAGE_PATCH,
        CHANGES_WORKING_TREE,
        CHANGES_IGNORED,
        DIFF_SIDES,
        FILE_VIEW_REVISION,
        MERGE_VIEW_SIDES,
        CONFLICTS_FILES,
        CONFLICTS_SIDE_NAMES,
        CONFLICTS_RULES,
        COMPARE_SIDES,
        COMPARE_FILES,
        COMPARE_PATCH,
        COMPARE_REVISION,
        BLAME_LINE,
        BLAME_FILE,
        FILE_HISTORY_NAMES,
        FILE_HISTORY_COMMITTED_LINES,
        REFLOG,
        REMOTES,
        IDENTITY,
        SUBMODULES,
        WORKTREE_FOLDER_LINKS,
        SETUP_IGNORED,
        SETUP_TAKES,
        SETUP_COMMANDS,
        SETUP_PATHS,
        PULL_REQUESTS,
        BRANCH_LOG,
        COMMON_FOLDER,
        PREPARED_MESSAGE,
        WATCH_CHANGES,
        DETAILS_SIGNATURE,
        COMMIT_AUTHORS,
    ];

    /// The host question `read/github.read-pull-requests` asks: the token of GitHub that VS Code keeps.
    pub const HOST_GITHUB_TOKEN: &str = "host/githubToken";
}

/// A commit as its full id.
pub(crate) fn id(commit: &CommitId) -> String {
    commit.as_str().to_owned()
}

/// Commits as their full ids.
pub(crate) fn ids(commits: &[CommitId]) -> Vec<String> {
    commits.iter().map(id).collect()
}

/// The commit of a full id, or what says it is not one.
///
/// # Errors
///
/// When the text is not the full id of a commit.
pub fn commit_of(text: &str) -> Result<CommitId, NotACommit> {
    CommitId::parse(text).ok_or_else(|| NotACommit(text.to_owned()))
}
