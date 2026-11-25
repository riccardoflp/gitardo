//! The state of a repository: everything the window shows of it but its commits, read all at once, or in two steps for
//! a repository that shows nothing yet: what the timeline needs first (`first`), then the rest from it (`rest`).
//!
//! Each part is a read of its own, its arguments and its parser apart (rule R2.5). The parsers and the arguments are
//! public so that the golden tests record exactly what the read asks git, and parse it without git.

mod branch_sort;
mod common_folder;
mod first;
mod git_version;
mod head;
mod identity;
mod location;
mod operation;
mod references;
mod reflog_subjects;
mod remotes;
mod repositories;
mod rest;
mod shallow;
mod stashes;
pub(crate) mod status;

use std::path::PathBuf;
use std::thread::{self, ScopedJoinHandle};

use model::state::{Head, RepositoryState};

use crate::read::worktree::CountedWorktrees;
use crate::{Git, GitError};

pub use crate::read::worktree::{parse_worktrees, worktrees_arguments};
pub use branch_sort::{branch_sort_arguments, parse_branch_sort};
pub use common_folder::{CommonFolderRequest, common_folder_arguments, parse_common_folder, read_common_folder};
pub use first::{FirstState, read_first};
pub use git_version::{GitVersionRequest, git_version_arguments, parse_git_version, read_git_version};
pub use head::{head_branch_arguments, head_commit_arguments, parse_head_branch, parse_head_commit};
pub use identity::{identity_arguments, parse_identity};
pub use location::{Location, location_arguments, parse_location};
pub use references::{References, parse_references, references_arguments};
pub use reflog_subjects::{parse_reflog_subjects, reflog_subjects_arguments};
pub use remotes::{parse_remotes, remotes_arguments};
pub use repositories::{
    RepositoriesRequest, parse_submodule_paths, parse_top_level, read_repositories, submodule_paths_arguments,
    top_level_arguments,
};
pub use rest::read_rest;
pub use shallow::{ShallowRequest, parse_shallow_file, parse_shallow_location, read_shallow, shallow_arguments};
pub use stashes::{parse_stashes, read_stashes, stashes_arguments};
pub use status::{parse_status, status_arguments, worktree_status_arguments};

pub(crate) use location::read_location;
pub(crate) use operation::read_operation;

/// Which repository to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRequest {
    /// A folder of the working tree: its top, or any folder in it.
    pub folder: PathBuf,
}

/// Everything the window shows of a repository but its commits: what is checked out, the branches and how far each is
/// from its upstream, the remote branches, the tags, the stashes, the worktrees with what is not committed in each,
/// the remotes, who commits are made as, git's order of the branches, where `HEAD` has been, what is not committed here, the operation that
/// stopped half way, the bisect under way (`bisect.read`) and where the history of a shallow clone ends
/// (`repo.read-shallow`). The reads run at once, each in a thread of its own, and
/// none takes a lock.
///
/// While a rebase is under way, where it stopped to edit or to split a commit is read too (`rebase.read-stop`), and
/// so is the bisect in progress (`bisect.read`). The submodules are read at once with the rest, by their own card
/// (`submodule.read-list`); what `conflicts.autoResolve` could resolve (`conflicts.resolve-by-rules`) is read by its
/// own card.
///
/// Feature: repo.read-state
///
/// # Errors
///
/// When the folder is not in a working tree, git cannot be started or fails, or prints what this does not understand.
pub fn read_state(git: &Git, request: &StateRequest) -> Result<RepositoryState, GitError> {
    read_state_counting(git, request, &CountedWorktrees::default()).map(|(state, _counted)| state)
}

/// The state of a repository as [`read_state`] reads it, but each other worktree keeps the count of what is not
/// committed there that `before` has, when nothing changed in its own git folder since the read that counted it
/// (M15): `git status` runs only in the others. Gives what was counted, for the read after this one; an empty
/// `before`, on the first read and on **Refresh**, counts them all.
///
/// # Errors
///
/// As [`read_state`].
pub fn read_state_counting(
    git: &Git,
    request: &StateRequest,
    before: &CountedWorktrees,
) -> Result<(RepositoryState, CountedWorktrees), GitError> {
    let location = location::read_location(&git.in_directory(&request.folder))?;
    let git = git.in_directory(&location.folder);
    // Both steps run at once, so that the whole takes about as long as the slowest read of the two.
    let (found, rest) = thread::scope(|scope| {
        let rest = scope.spawn(|| rest::read_rest_parts(&git, &location, before));
        let found = first::read_first_parts(&git, &location, Some(before));
        (found, joined(rest))
    });
    let (found, counted) = found?;
    rest::complete(&git, first::first_state(location, found), rest?, counted)
}

/// Which parts of the state a read of the graph needs besides what is checked out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimelineParts {
    /// The branches with their upstreams, the remote branches and the tags: for the references ticked one by one,
    /// which are checked against them, and for the upstream of the branch checked out.
    pub references: bool,
    /// The stashes, to place among the commits.
    pub stashes: bool,
}

/// The parts of the state that say which history the timeline lists, for a read of the graph asked without a window
/// that keeps the state (`graph.read-commits`): what is checked out, and the references and the stashes when they are
/// asked for, read at once. The rest is left empty: the list of commits needs neither `git status` nor the distance of
/// each branch from its upstream, which take longer than the page itself on a large repository.
///
/// # Errors
///
/// When git cannot be started or fails, or prints what this does not understand.
pub fn read_timeline_parts(git: &Git, parts: TimelineParts) -> Result<RepositoryState, GitError> {
    thread::scope(|scope| {
        let references = parts
            .references
            .then(|| scope.spawn(|| references::read_references(git)));
        let head_commit = scope.spawn(|| head::read_head_commit(git));
        let head_branch = scope.spawn(|| head::read_head_branch(git));
        let stashes = parts.stashes.then(|| scope.spawn(|| stashes::read_stashes(git)));
        let references = references.map(joined).transpose()?.unwrap_or_default();
        Ok(RepositoryState {
            head: Head {
                commit: joined(head_commit)?,
                branch: joined(head_branch)?,
            },
            branches: references.branches,
            remote_branches: references.remote_branches,
            tags: references.tags,
            stashes: stashes.map(joined).transpose()?.unwrap_or_default(),
            ..RepositoryState::default()
        })
    })
}

/// What a read on a thread of its own returned. A read that panicked panics here too, as it would have without the
/// thread.
pub(crate) fn joined<Value>(reader: ScopedJoinHandle<'_, Value>) -> Value {
    reader
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}
