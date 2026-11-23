//! The state of a repository as last read: everything the window shows of it but its commits.

mod references;
mod shallow;
mod skipped;
mod working_tree;

use std::path::PathBuf;

use crate::CommitId;

pub use references::{Branch, RemoteBranch, Stash, Tag, Upstream, Worktree};
pub use shallow::Shallow;
pub use skipped::{Skipped, SkippedFile};
pub use working_tree::{ChangeKind, ConflictKind, ConflictedFile, FileChange, WorkingTree};

/// What one read of a repository found. It does not change once read: a new read makes a new state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryState {
    /// The top folder of the working tree that was read.
    pub folder: PathBuf,
    /// The folder of git's files for this worktree, in full: `.git` of the main worktree, or
    /// `.git/worktrees/<name>` of another.
    pub git_folder: PathBuf,
    /// What is checked out.
    pub head: Head,
    /// The local branches, by name.
    pub branches: Vec<Branch>,
    /// The remote branches, by name, without `<remote>/HEAD`.
    pub remote_branches: Vec<RemoteBranch>,
    /// The tags, by name, each at the commit it points at.
    pub tags: Vec<Tag>,
    /// The stashes, newest first.
    pub stashes: Vec<Stash>,
    /// The worktrees, the main one first. Empty in a bare repository.
    pub worktrees: Vec<Worktree>,
    /// The names of the remotes.
    pub remotes: Vec<String>,
    /// Who commits are made as.
    pub identity: Identity,
    /// How git's configuration signs commits (`commit.read-signing`).
    pub signing: crate::commit::SigningConfig,
    /// Git's `branch.sort`, when it is set: the order `git branch` lists the branches in, such as `-committerdate`,
    /// which `branches.sortOrder` follows while it is not set (`refs.sort`).
    pub branch_sort: Option<String>,
    /// What the reflog of `HEAD` says, newest first: the subjects of its last entries, such as
    /// `checkout: moving from main to feature`.
    pub reflog_subjects: Vec<String>,
    /// What is not committed.
    pub working_tree: WorkingTree,
    /// The operation that stopped half way, if one did.
    pub operation: Option<Operation>,
    /// Where an interactive rebase stopped to edit or to split a commit, when it did (`rebase.read-stop`).
    pub rebase_stop: Option<crate::rebase::RebaseStop>,
    /// The bisect under way in this worktree, if one is (`bisect.read`).
    pub bisect: Option<crate::bisect::Bisect>,
    /// The submodules, and theirs, each followed by its own (`submodule.read-list`).
    pub submodules: Vec<crate::submodule::Submodule>,
    /// Where the history ends, when the repository is a shallow clone (`repo.read-shallow`).
    pub shallow: Shallow,
    /// The tracked files git is told not to look at, by hand: not those a sparse checkout leaves out
    /// (`changes.read-skipped`).
    pub skipped: Vec<SkippedFile>,
    /// Whether git's scheduled maintenance looks after the repository (`upkeep.read-background`).
    pub is_looked_after: bool,
    /// Whether git keeps its commit-graph file of the repository, and reads it (`upkeep.read-commit-graph`).
    pub commit_graph: crate::upkeep::CommitGraph,
    /// The commits with a note, in the reference of notes git uses (`note.read`).
    pub notes: crate::note::Notes,
    /// Whether Git LFS is set up for the repository and the patterns it stores with it, with the version of Git LFS
    /// once the repository has patterns or is set up (`lfs.read-status`).
    pub lfs: crate::lfs::LfsStatus,
    /// Whether git remembers how conflicts are resolved, and stages what it writes (`rerere.read-state`).
    pub rerere: crate::rerere::RerereState,
    /// The files git resolved as before during the operation under way, while it remembers resolutions
    /// (`rerere.read-resolved`).
    pub resolved_as_before: Vec<crate::rerere::ResolvedAsBefore>,
}

impl RepositoryState {
    /// The local branch checked out, by its short name; none when `HEAD` is detached.
    #[must_use]
    pub fn checked_out_branch(&self) -> Option<&str> {
        self.head
            .branch
            .as_deref()
            .and_then(|full| full.strip_prefix("refs/heads/"))
    }

    /// Whether there is a commit checked out: not before the first commit.
    #[must_use]
    pub fn has_commit(&self) -> bool {
        self.head.commit.is_some()
    }
}

/// What is checked out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Head {
    /// The commit, none before the first commit.
    pub commit: Option<CommitId>,
    /// The branch, by its full name (`refs/heads/main`), none when detached. Before the first commit it is the branch
    /// git will create.
    pub branch: Option<String>,
}

/// Who commits are made as, from git's configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Identity {
    /// `user.name`.
    pub name: Option<String>,
    /// `user.email`.
    pub email: Option<String>,
}

/// An operation git stopped half way, for a conflict or to let the user change something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// `git merge`.
    Merge,
    /// `git rebase`.
    Rebase,
    /// `git cherry-pick`.
    CherryPick,
    /// `git revert`.
    Revert,
    /// `git am`, a patch applied as commits.
    Patch,
}
