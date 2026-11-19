//! The JSON of the state, made from the state of `model`.

use std::path::Path;

use model::CommitId;
use model::bisect::Bisect;
use model::rebase::{RebaseStop, StopKind};
use model::state::{
    Branch, ChangeKind, ConflictKind, ConflictedFile, FileChange, Operation, RemoteBranch, RepositoryState, Stash, Tag,
    Upstream, WorkingTree, Worktree,
};
use model::submodule::{Checkout, Distance, Submodule};

use super::{
    BisectJson, BranchJson, ChangeKindJson, CheckoutJson, ConflictKindJson, ConflictedFileJson, DistanceJson,
    FileChangeJson, HeadJson, IdentityJson, NotesJson, OperationJson, RebaseStopJson, RemoteBranchJson,
    SkippedFileJson, StashJson, StateJson, StopKindJson, SubmoduleJson, TagJson, UpstreamJson, WorkingTreeJson,
    WorktreeJson,
};

/// A path as the system gives it, as a string.
fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// A commit as its full id.
fn id(commit: &CommitId) -> String {
    commit.as_str().to_owned()
}

/// Commits as their full ids.
fn ids(commits: &[CommitId]) -> Vec<String> {
    commits.iter().map(id).collect()
}

impl From<&RepositoryState> for StateJson {
    fn from(state: &RepositoryState) -> StateJson {
        StateJson {
            folder: path_text(&state.folder),
            git_folder: path_text(&state.git_folder),
            head: HeadJson {
                commit: state.head.commit.as_ref().map(id),
                branch: state.head.branch.clone(),
            },
            branches: state.branches.iter().map(BranchJson::from).collect(),
            remote_branches: state.remote_branches.iter().map(RemoteBranchJson::from).collect(),
            tags: state.tags.iter().map(TagJson::from).collect(),
            stashes: state.stashes.iter().map(StashJson::from).collect(),
            worktrees: state.worktrees.iter().map(WorktreeJson::from).collect(),
            remotes: state.remotes.clone(),
            identity: IdentityJson {
                name: state.identity.name.clone(),
                email: state.identity.email.clone(),
            },
            branch_sort: state.branch_sort.clone(),
            reflog_subjects: state.reflog_subjects.clone(),
            working_tree: WorkingTreeJson::from(&state.working_tree),
            operation: state.operation.map(OperationJson::from),
            rebase_stop: state.rebase_stop.as_ref().map(RebaseStopJson::from),
            bisect: state.bisect.as_ref().map(BisectJson::from),
            submodules: state.submodules.iter().map(SubmoduleJson::from).collect(),
            shallow: ids(&state.shallow.ends),
            skipped: state
                .skipped
                .iter()
                .map(|file| SkippedFileJson {
                    path: file.path.clone(),
                    skip_worktree: file.how.is_skip_worktree(),
                    assume_unchanged: file.how.is_assumed_unchanged(),
                })
                .collect(),
            is_looked_after: state.is_looked_after,
            notes: NotesJson {
                reference: state.notes.reference.clone(),
                tip: state.notes.tip.as_ref().map(id),
                commits: state.notes.noted.keys().map(id).collect(),
            },
        }
    }
}

impl From<&Branch> for BranchJson {
    fn from(branch: &Branch) -> BranchJson {
        BranchJson {
            name: branch.name.clone(),
            commit: id(&branch.commit),
            upstream: branch.upstream.as_ref().map(UpstreamJson::from),
            is_checked_out: branch.is_checked_out,
            date: branch.date.0,
        }
    }
}

impl From<&Upstream> for UpstreamJson {
    fn from(upstream: &Upstream) -> UpstreamJson {
        UpstreamJson {
            reference: upstream.reference.clone(),
            remote: upstream.remote.clone(),
            remote_branch: upstream.remote_branch.clone(),
            ahead: upstream.ahead,
            behind: upstream.behind,
            is_gone: upstream.is_gone,
        }
    }
}

impl From<&RemoteBranch> for RemoteBranchJson {
    fn from(branch: &RemoteBranch) -> RemoteBranchJson {
        RemoteBranchJson {
            name: branch.name.clone(),
            remote: branch.remote.clone(),
            commit: id(&branch.commit),
            date: branch.date.0,
        }
    }
}

impl From<&Tag> for TagJson {
    fn from(tag: &Tag) -> TagJson {
        TagJson {
            name: tag.name.clone(),
            commit: id(&tag.commit),
            date: tag.date.0,
        }
    }
}

impl From<&Stash> for StashJson {
    fn from(stash: &Stash) -> StashJson {
        StashJson {
            commit: id(&stash.commit),
            index: stash.index,
            message: stash.message.clone(),
            date: stash.date.0,
        }
    }
}

impl From<&Worktree> for WorktreeJson {
    fn from(worktree: &Worktree) -> WorktreeJson {
        WorktreeJson {
            folder: path_text(&worktree.folder),
            commit: worktree.commit.as_ref().map(id),
            branch: worktree.branch.clone(),
            is_main: worktree.is_main,
            is_current: worktree.is_current,
            is_missing: worktree.is_missing,
            locked: worktree.locked.clone(),
            uncommitted: worktree.uncommitted,
        }
    }
}

impl From<&WorkingTree> for WorkingTreeJson {
    fn from(tree: &WorkingTree) -> WorkingTreeJson {
        WorkingTreeJson {
            staged: tree.staged.iter().map(FileChangeJson::from).collect(),
            unstaged: tree.unstaged.iter().map(FileChangeJson::from).collect(),
            untracked: tree.untracked.clone(),
            conflicted: tree.conflicted.iter().map(ConflictedFileJson::from).collect(),
        }
    }
}

impl From<&FileChange> for FileChangeJson {
    fn from(change: &FileChange) -> FileChangeJson {
        FileChangeJson {
            path: change.path.clone(),
            original_path: change.original_path.clone(),
            kind: ChangeKindJson::from(change.kind),
        }
    }
}

impl From<ChangeKind> for ChangeKindJson {
    fn from(kind: ChangeKind) -> ChangeKindJson {
        match kind {
            ChangeKind::Added => ChangeKindJson::Added,
            ChangeKind::Modified => ChangeKindJson::Modified,
            ChangeKind::Deleted => ChangeKindJson::Deleted,
            ChangeKind::Renamed => ChangeKindJson::Renamed,
            ChangeKind::Copied => ChangeKindJson::Copied,
            ChangeKind::TypeChanged => ChangeKindJson::TypeChanged,
        }
    }
}

impl From<&ConflictedFile> for ConflictedFileJson {
    fn from(file: &ConflictedFile) -> ConflictedFileJson {
        ConflictedFileJson {
            path: file.path.clone(),
            kind: ConflictKindJson::from(file.kind),
        }
    }
}

impl From<ConflictKind> for ConflictKindJson {
    fn from(kind: ConflictKind) -> ConflictKindJson {
        match kind {
            ConflictKind::BothDeleted => ConflictKindJson::BothDeleted,
            ConflictKind::AddedByUs => ConflictKindJson::AddedByUs,
            ConflictKind::DeletedByThem => ConflictKindJson::DeletedByThem,
            ConflictKind::AddedByThem => ConflictKindJson::AddedByThem,
            ConflictKind::DeletedByUs => ConflictKindJson::DeletedByUs,
            ConflictKind::BothAdded => ConflictKindJson::BothAdded,
            ConflictKind::BothModified => ConflictKindJson::BothModified,
        }
    }
}

impl From<Operation> for OperationJson {
    fn from(operation: Operation) -> OperationJson {
        match operation {
            Operation::Merge => OperationJson::Merge,
            Operation::Rebase => OperationJson::Rebase,
            Operation::CherryPick => OperationJson::CherryPick,
            Operation::Revert => OperationJson::Revert,
            Operation::Patch => OperationJson::Patch,
        }
    }
}

impl From<&RebaseStop> for RebaseStopJson {
    fn from(stop: &RebaseStop) -> RebaseStopJson {
        RebaseStopJson {
            kind: match stop.kind {
                StopKind::Edit => StopKindJson::Edit,
                StopKind::Split => StopKindJson::Split,
            },
            commit: id(&stop.commit),
            subject: stop.subject.clone(),
        }
    }
}

impl From<&Bisect> for BisectJson {
    fn from(bisect: &Bisect) -> BisectJson {
        BisectJson {
            bad: bisect.bad.as_ref().map(id),
            good: ids(&bisect.good),
            skipped: ids(&bisect.skipped),
            remaining: bisect.remaining,
        }
    }
}

impl From<&Submodule> for SubmoduleJson {
    fn from(submodule: &Submodule) -> SubmoduleJson {
        SubmoduleJson {
            name: submodule.name.clone(),
            folder: submodule.folder.clone(),
            parent: submodule.parent.clone(),
            url: submodule.url.clone(),
            branch: submodule.branch.clone(),
            recorded: submodule.recorded.as_ref().map(id),
            staged: submodule.staged.as_ref().map(id),
            checkout: submodule.checkout.as_ref().map(CheckoutJson::from),
        }
    }
}

impl From<&Checkout> for CheckoutJson {
    fn from(checkout: &Checkout) -> CheckoutJson {
        CheckoutJson {
            commit: checkout.commit.as_ref().map(id),
            branch: checkout.branch.clone(),
            uncommitted: checkout.uncommitted,
            distance: checkout.distance.map(DistanceJson::from),
        }
    }
}

impl From<Distance> for DistanceJson {
    fn from(distance: Distance) -> DistanceJson {
        DistanceJson {
            ahead: distance.ahead,
            behind: distance.behind,
        }
    }
}
