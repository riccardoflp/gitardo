//! The first step of the state of a repository: what the timeline, the branch rail and the Changes page show, read
//! before the rest so that a repository chosen shows its first commits after a few reads (`repo.read-first`).

use std::thread;

use model::state::{Head, Operation, RepositoryState, Stash, WorkingTree, Worktree};

use super::location::{Location, read_location};
use super::references::References;
use super::{StateRequest, head, joined, operation, references, stashes, status};
use crate::read::worktree::{self, CountedWorktrees, WorktreeListRequest};
use crate::{Git, GitError};

/// What the first step of the state found: where the repository is, its references, what is checked out, the
/// stashes, the worktrees, what is not committed here and the operation stopped half way. The rest of
/// [`RepositoryState`] is as empty as before any read, and no other worktree has a count: `repo.read-state` reads
/// them from this.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FirstState {
    /// The state, with the parts of the first step only.
    pub state: RepositoryState,
}

/// What the timeline, the branch rail and the Changes page need of a repository, read before the rest of its state:
/// where it is, then at once its references, the commit and the branch checked out, the stashes, the worktrees
/// without the count of the others, what is not committed in this one and the operation in progress. Seven processes
/// of git, none in another worktree, none taking a lock.
///
/// Feature: repo.read-first
///
/// # Errors
///
/// When the folder is not in a working tree, git cannot be started or fails, or prints what this does not understand.
pub fn read_first(git: &Git, request: &StateRequest) -> Result<FirstState, GitError> {
    let location = read_location(&git.in_directory(&request.folder))?;
    let git = git.in_directory(&location.folder);
    let (found, _nothing_counted) = read_first_parts(&git, &location, None)?;
    Ok(FirstState {
        state: first_state(location, found),
    })
}

/// What each read of the first step found.
pub(super) struct FirstParts {
    references: References,
    head: Head,
    stashes: Vec<Stash>,
    worktrees: Vec<Worktree>,
    working_tree: WorkingTree,
    operation: Option<Operation>,
}

/// Runs the reads of the first step at once, each in a thread of its own; the operation in progress is read from the
/// disk meanwhile. Given what the read before counted, the other worktrees are counted as soon as they are listed,
/// as a whole read of the state does; given none, they are listed only.
pub(super) fn read_first_parts(
    git: &Git,
    location: &Location,
    counting: Option<&CountedWorktrees>,
) -> Result<(FirstParts, CountedWorktrees), GitError> {
    let request = WorktreeListRequest {
        folder: location.folder.clone(),
        uncommitted_here: None,
    };
    thread::scope(|scope| {
        let references = scope.spawn(|| references::read_references(git));
        let head_commit = scope.spawn(|| head::read_head_commit(git));
        let head_branch = scope.spawn(|| head::read_head_branch(git));
        let stashes = scope.spawn(|| stashes::read_stashes(git));
        let worktrees = scope.spawn(|| match counting {
            Some(before) => worktree::read_list_counting(git, &request, before),
            None => worktree::read_listed(git, &request).map(|listed| (listed, CountedWorktrees::default())),
        });
        let working_tree = scope.spawn(|| status::read_status(git));
        let operation = operation::read_operation(&location.git_folder);
        let (worktrees, counted) = joined(worktrees)?;
        let found = FirstParts {
            references: joined(references)?,
            head: Head {
                commit: joined(head_commit)?,
                branch: joined(head_branch)?,
            },
            stashes: joined(stashes)?,
            worktrees,
            working_tree: joined(working_tree)?,
            operation,
        };
        Ok((found, counted))
    })
}

/// The state with what the first step found, the rest left empty. The worktree read counts what this read found not
/// committed; each remote branch is of the remote its name starts with up to the first `/`, until the remotes are
/// read.
pub(super) fn first_state(location: Location, found: FirstParts) -> RepositoryState {
    let uncommitted = found.working_tree.file_count();
    let worktrees = found
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
        head: found.head,
        branches: found.references.branches,
        remote_branches: found.references.remote_branches,
        tags: found.references.tags,
        stashes: found.stashes,
        worktrees,
        working_tree: found.working_tree,
        operation: found.operation,
        ..RepositoryState::default()
    }
}
