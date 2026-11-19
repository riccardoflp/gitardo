//! The state of a repository: everything the window shows of it but its commits, read all at once.
//!
//! Each part is a read of its own, its arguments and its parser apart (rule R2.5). The parsers and the arguments are
//! public so that the golden tests record exactly what the read asks git, and parse it without git.

mod branch_sort;
mod common_folder;
mod git_version;
mod head;
mod identity;
mod location;
mod operation;
mod references;
mod reflog_subjects;
mod remotes;
mod repositories;
mod shallow;
mod stashes;
pub(crate) mod status;

use std::path::PathBuf;
use std::thread::{self, ScopedJoinHandle};

use model::state::{Head, Identity, Operation, RepositoryState, Shallow, Stash, WorkingTree, Worktree};

use crate::read::worktree::{self, CountedWorktrees, WorktreeListRequest};
use crate::{Git, GitError};

pub use crate::read::worktree::{parse_worktrees, worktrees_arguments};
pub use branch_sort::{branch_sort_arguments, parse_branch_sort};
pub use common_folder::{CommonFolderRequest, common_folder_arguments, parse_common_folder, read_common_folder};
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
    let (parts, counted) = read_parts(&git, &location, before)?;
    let rebase_stop = if parts.operation == Some(Operation::Rebase) {
        let request = crate::read::rebase::StopRequest {
            git_folder: location.git_folder.clone(),
        };
        crate::read::rebase::read_stop(&git, &request)?
    } else {
        None
    };
    let resolved_as_before = read_resolved_as_before(&git, &location, &parts)?;
    let state = RepositoryState {
        rebase_stop,
        resolved_as_before,
        ..assemble(location, parts)
    };
    Ok((state, counted))
}

/// The files git resolved as before, while it remembers resolutions and an operation is stopped or files are in
/// conflict (`rerere.read-resolved`); none otherwise, without starting git.
fn read_resolved_as_before(
    git: &Git,
    location: &Location,
    parts: &Parts,
) -> Result<Vec<model::rerere::ResolvedAsBefore>, GitError> {
    let conflicted = &parts.working_tree.conflicted;
    if !parts.rerere.is_on || (parts.operation.is_none() && conflicted.is_empty()) {
        return Ok(Vec::new());
    }
    let request = model::rerere::ResolvedRequest {
        git_folder: location.git_folder.clone(),
        conflicted: conflicted.iter().map(|file| file.path.clone()).collect(),
    };
    crate::read::rerere::read_resolved(git, &request)
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

/// What each read found.
struct Parts {
    references: References,
    head: Head,
    stashes: Vec<Stash>,
    worktrees: Vec<Worktree>,
    remotes: Vec<String>,
    identity: Identity,
    signing: model::commit::SigningConfig,
    branch_sort: Option<String>,
    reflog_subjects: Vec<String>,
    working_tree: WorkingTree,
    operation: Option<Operation>,
    bisect: Option<model::bisect::Bisect>,
    submodules: Vec<model::submodule::Submodule>,
    shallow: Shallow,
    skipped: Vec<model::state::SkippedFile>,
    is_looked_after: bool,
    notes: model::note::Notes,
    lfs: model::lfs::LfsStatus,
    rerere: model::rerere::RerereState,
}

/// Runs every read at once, each in a thread of its own, so that the whole takes about as long as the slowest; the
/// operation in progress is read from the disk meanwhile.
fn read_parts(
    git: &Git,
    location: &Location,
    before: &CountedWorktrees,
) -> Result<(Parts, CountedWorktrees), GitError> {
    let worktree_request = WorktreeListRequest {
        folder: location.folder.clone(),
        uncommitted_here: None,
    };
    thread::scope(|scope| {
        let references = scope.spawn(|| references::read_references(git));
        let head_commit = scope.spawn(|| head::read_head_commit(git));
        let head_branch = scope.spawn(|| head::read_head_branch(git));
        let stashes = scope.spawn(|| stashes::read_stashes(git));
        let worktrees = scope.spawn(|| worktree::read_list_counting(git, &worktree_request, before));
        let remotes = scope.spawn(|| remotes::read_remotes(git));
        let identity = scope.spawn(|| identity::read_identity(git));
        let signing = scope.spawn(|| crate::read::commit::read_signing(git, &crate::read::commit::SigningRequest));
        let branch_sort = scope.spawn(|| branch_sort::read_branch_sort(git));
        let reflog_subjects = scope.spawn(|| reflog_subjects::read_reflog_subjects(git));
        let working_tree = scope.spawn(|| status::read_status(git));
        let bisect_request = crate::read::bisect::BisectRequest {
            git_folder: location.git_folder.clone(),
        };
        let bisect = scope.spawn(move || crate::read::bisect::read(git, &bisect_request));
        let submodules_request = crate::read::submodule::SubmoduleListRequest {
            folder: location.folder.clone(),
        };
        let submodules = scope.spawn(move || crate::read::submodule::read_list(git, &submodules_request));
        let shallow_request = ShallowRequest {
            folder: location.folder.clone(),
        };
        let shallow = scope.spawn(move || read_shallow(git, &shallow_request));
        let kept = scope.spawn(|| read_kept(git, location, before));
        let lfs = scope.spawn(|| crate::read::lfs::read_repository_status(git));
        let rerere_request = model::rerere::RerereStateRequest {
            git_folder: location.git_folder.clone(),
        };
        let rerere = scope.spawn(move || crate::read::rerere::read_state(git, &rerere_request));
        let operation = operation::read_operation(&location.git_folder);
        let (worktrees, mut counted) = joined(worktrees)?;
        let kept = joined(kept)?;
        let parts = Parts {
            references: joined(references)?,
            head: Head {
                commit: joined(head_commit)?,
                branch: joined(head_branch)?,
            },
            stashes: joined(stashes)?,
            worktrees,
            remotes: joined(remotes)?,
            identity: joined(identity)?,
            signing: joined(signing)?,
            branch_sort: joined(branch_sort)?,
            reflog_subjects: joined(reflog_subjects)?,
            working_tree: joined(working_tree)?,
            operation,
            bisect: joined(bisect)?,
            submodules: joined(submodules)?,
            shallow: joined(shallow)?,
            skipped: kept.skipped.files.clone(),
            is_looked_after: kept.background.is_registered,
            notes: kept.notes.clone(),
            lfs: joined(lfs)?,
            rerere: joined(rerere)?,
        };
        counted.skipped = Some(kept.skipped);
        counted.background = Some(kept.background);
        counted.notes = Some(kept.notes);
        Ok((parts, counted))
    })
}

/// What the reads kept from the read before found.
struct Kept {
    skipped: crate::read::changes::SkippedRead,
    background: crate::read::upkeep::BackgroundRead,
    notes: model::note::Notes,
}

/// The reads kept from the read before while what they read did not change: the files hidden from git, while the index
/// did not change (`changes.read-skipped`); whether git's maintenance looks after the repository, while the user's
/// configuration did not (`upkeep.read-background`); and the commits with a note, while the reference of notes did not
/// move (`note.read`).
fn read_kept(git: &Git, location: &Location, before: &CountedWorktrees) -> Result<Kept, GitError> {
    let skipped_request = crate::read::changes::SkippedRequest {
        git_folder: location.git_folder.clone(),
        before: before.skipped.clone(),
    };
    let background_request = crate::read::upkeep::BackgroundRequest {
        folder: location.folder.clone(),
        before: before.background.clone(),
    };
    let notes_request = crate::read::note::NoteRequest {
        before: before.notes.clone(),
    };
    Ok(Kept {
        skipped: crate::read::changes::read_skipped(git, &skipped_request)?,
        background: crate::read::upkeep::read_background_kept(git, &background_request)?,
        notes: crate::read::note::read(git, &notes_request)?,
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
        signing: parts.signing,
        branch_sort: parts.branch_sort,
        reflog_subjects: parts.reflog_subjects,
        working_tree: parts.working_tree,
        operation: parts.operation,
        rebase_stop: None,
        bisect: parts.bisect,
        submodules: parts.submodules,
        shallow: parts.shallow,
        skipped: parts.skipped,
        is_looked_after: parts.is_looked_after,
        notes: parts.notes,
        lfs: parts.lfs,
        rerere: parts.rerere,
        resolved_as_before: Vec::new(),
    }
}

/// What a read on a thread of its own returned. A read that panicked panics here too, as it would have without the
/// thread.
pub(crate) fn joined<Value>(reader: ScopedJoinHandle<'_, Value>) -> Value {
    reader
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}
