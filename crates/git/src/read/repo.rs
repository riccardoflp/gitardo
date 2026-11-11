//! The state of a repository: everything the window shows of it but its commits, read all at once.
//!
//! Each part is a read of its own, its arguments and its parser apart (rule R2.5). The parsers and the arguments are
//! public so that the golden tests record exactly what the read asks git, and parse it without git.

mod fields;
mod head;
mod identity;
mod location;
mod operation;
mod references;
mod reflog_subjects;
mod remotes;
mod stashes;
mod status;
mod worktrees;

use std::path::PathBuf;
use std::thread::{self, ScopedJoinHandle};

use model::state::{Head, Identity, Operation, RepositoryState, Stash, WorkingTree, Worktree};

use crate::{Git, GitError};

pub use head::{head_branch_arguments, head_commit_arguments, parse_head_branch, parse_head_commit};
pub use identity::{identity_arguments, parse_identity};
pub use location::{Location, location_arguments, parse_location};
pub use references::{References, parse_references, references_arguments};
pub use reflog_subjects::{parse_reflog_subjects, reflog_subjects_arguments};
pub use remotes::{parse_remotes, remotes_arguments};
pub use stashes::{parse_stashes, stashes_arguments};
pub use status::{parse_status, status_arguments, worktree_status_arguments};
pub use worktrees::{parse_worktrees, worktrees_arguments};

/// Which repository to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRequest {
    /// A folder of the working tree: its top, or any folder in it.
    pub folder: PathBuf,
}

/// Everything the window shows of a repository but its commits: what is checked out, the branches and how far each is
/// from its upstream, the remote branches, the tags, the stashes, the worktrees with what is not committed in each,
/// the remotes, who commits are made as, where `HEAD` has been, what is not committed here, and the operation that
/// stopped half way. The reads run at once, each in a thread of its own, and none takes a lock.
///
/// The submodules (`submodule.read-list`), the bisect in progress (`bisect.start`), why a rebase stopped
/// (`conflicts.continue-operation`, `split.commit`) and what `conflicts.autoResolve` could resolve
/// (`conflicts.resolve-by-rules`) are read by their own cards.
///
/// Feature: repo.read-state
///
/// # Errors
///
/// When the folder is not in a working tree, git cannot be started or fails, or prints what this does not understand.
pub fn read_state(git: &Git, request: &StateRequest) -> Result<RepositoryState, GitError> {
    let location = location::read_location(&git.in_directory(&request.folder))?;
    let git = git.in_directory(&location.folder);
    let parts = read_parts(&git, &location)?;
    Ok(assemble(location, parts))
}

/// What each read found.
struct Parts {
    references: References,
    head: Head,
    stashes: Vec<Stash>,
    worktrees: Vec<Worktree>,
    remotes: Vec<String>,
    identity: Identity,
    reflog_subjects: Vec<String>,
    working_tree: WorkingTree,
    operation: Option<Operation>,
}

/// Runs every read at once, each in a thread of its own, so that the whole takes about as long as the slowest; the
/// operation in progress is read from the disk meanwhile.
fn read_parts(git: &Git, location: &Location) -> Result<Parts, GitError> {
    thread::scope(|scope| {
        let references = scope.spawn(|| references::read_references(git));
        let head_commit = scope.spawn(|| head::read_head_commit(git));
        let head_branch = scope.spawn(|| head::read_head_branch(git));
        let stashes = scope.spawn(|| stashes::read_stashes(git));
        let worktrees = scope.spawn(|| worktrees::read_worktrees(git, &location.folder));
        let remotes = scope.spawn(|| remotes::read_remotes(git));
        let identity = scope.spawn(|| identity::read_identity(git));
        let reflog_subjects = scope.spawn(|| reflog_subjects::read_reflog_subjects(git));
        let working_tree = scope.spawn(|| status::read_status(git));
        let operation = operation::read_operation(&location.git_folder);
        Ok(Parts {
            references: joined(references)?,
            head: Head {
                commit: joined(head_commit)?,
                branch: joined(head_branch)?,
            },
            stashes: joined(stashes)?,
            worktrees: joined(worktrees)?,
            remotes: joined(remotes)?,
            identity: joined(identity)?,
            reflog_subjects: joined(reflog_subjects)?,
            working_tree: joined(working_tree)?,
            operation,
        })
    })
}

/// The state, from where the repository is and what each read found. The worktree read counts what this read found
/// not committed, and each remote branch belongs to the longest remote its name starts with.
fn assemble(location: Location, parts: Parts) -> RepositoryState {
    let uncommitted = parts.working_tree.file_count();
    let worktrees = parts
        .worktrees
        .into_iter()
        .map(|worktree| Worktree {
            uncommitted: if worktree.is_current {
                Some(uncommitted)
            } else {
                worktree.uncommitted
            },
            ..worktree
        })
        .collect();
    RepositoryState {
        folder: location.folder,
        git_folder: location.git_folder,
        head: parts.head,
        branches: parts.references.branches,
        remote_branches: references::with_remotes(parts.references.remote_branches, &parts.remotes),
        tags: parts.references.tags,
        stashes: parts.stashes,
        worktrees,
        remotes: parts.remotes,
        identity: parts.identity,
        reflog_subjects: parts.reflog_subjects,
        working_tree: parts.working_tree,
        operation: parts.operation,
    }
}

/// What a read on a thread of its own returned. A read that panicked panics here too, as it would have without the
/// thread.
fn joined<Value>(reader: ScopedJoinHandle<'_, Value>) -> Value {
    reader
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}
