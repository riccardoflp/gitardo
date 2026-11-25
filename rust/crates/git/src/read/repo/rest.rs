//! The second step of the state of a repository (`repo.read-state`): what the first step did not read, read from what
//! it found, so that none of its commands starts again.

use std::thread;

use model::state::{Identity, Operation, RepositoryState, Shallow};

use super::first::FirstState;
use super::location::Location;
use super::shallow::{ShallowRequest, read_shallow};
use super::{branch_sort, identity, joined, references, reflog_subjects, remotes};
use crate::read::worktree::{self, CountedWorktrees};
use crate::{Git, GitError};

/// The whole state of a repository from what its first step found (`repo.read-first`): the count of what is not
/// committed in each other worktree, the remotes, who commits are made as, git's order of the branches, where `HEAD`
/// has been, the submodules, the bisect under way, where a shallow history ends, the commit-graph file, the notes,
/// Git LFS and the resolutions remembered, all read at once. None of the commands of the first step starts again;
/// `before` is what the read before counted, as [`super::read_state_counting`] takes it.
///
/// # Errors
///
/// When git cannot be started or fails, or prints what this does not understand.
pub fn read_rest(
    git: &Git,
    first: FirstState,
    before: &CountedWorktrees,
) -> Result<(RepositoryState, CountedWorktrees), GitError> {
    let mut state = first.state;
    let location = Location {
        folder: state.folder.clone(),
        git_folder: state.git_folder.clone(),
    };
    let git = git.in_directory(&location.folder);
    let listed = std::mem::take(&mut state.worktrees);
    let (rest, (worktrees, counted)) = thread::scope(|scope| {
        let rest = scope.spawn(|| read_rest_parts(&git, &location, before));
        let counts = worktree::count_listed(&git, listed, &location.folder, before);
        (joined(rest), counts)
    });
    state.worktrees = worktrees;
    complete(&git, state, rest?, counted)
}

/// What each read of the second step found, but the counts of the other worktrees.
pub(super) struct RestParts {
    remotes: Vec<String>,
    identity: Identity,
    signing: model::commit::SigningConfig,
    branch_sort: Option<String>,
    reflog_subjects: Vec<String>,
    bisect: Option<model::bisect::Bisect>,
    submodules: Vec<model::submodule::Submodule>,
    shallow: Shallow,
    kept: Kept,
    lfs: model::lfs::LfsStatus,
    rerere: model::rerere::RerereState,
}

/// Runs every read of the second step at once, each in a thread of its own.
pub(super) fn read_rest_parts(
    git: &Git,
    location: &Location,
    before: &CountedWorktrees,
) -> Result<RestParts, GitError> {
    thread::scope(|scope| {
        let remotes = scope.spawn(|| remotes::read_remotes(git));
        let identity = scope.spawn(|| identity::read_identity(git));
        let signing = scope.spawn(|| crate::read::commit::read_signing(git, &crate::read::commit::SigningRequest));
        let branch_sort = scope.spawn(|| branch_sort::read_branch_sort(git));
        let reflog_subjects = scope.spawn(|| reflog_subjects::read_reflog_subjects(git));
        let bisect = scope.spawn(|| read_bisect(git, location));
        let submodules = scope.spawn(|| read_submodules(git, location));
        let shallow = scope.spawn(|| read_shallow_of(git, location));
        let kept = scope.spawn(|| read_kept(git, location, before));
        let lfs = scope.spawn(|| crate::read::lfs::read_repository_status(git));
        let rerere = scope.spawn(|| read_rerere(git, location));
        Ok(RestParts {
            remotes: joined(remotes)?,
            identity: joined(identity)?,
            signing: joined(signing)?,
            branch_sort: joined(branch_sort)?,
            reflog_subjects: joined(reflog_subjects)?,
            bisect: joined(bisect)?,
            submodules: joined(submodules)?,
            shallow: joined(shallow)?,
            kept: joined(kept)?,
            lfs: joined(lfs)?,
            rerere: joined(rerere)?,
        })
    })
}

/// The whole state, from the state of the first step with its worktrees counted and what the second step found: each
/// remote branch belongs to the longest remote its name starts with; where a rebase under way stopped to edit or to
/// split a commit (`rebase.read-stop`) and the files git resolved as before (`rerere.read-resolved`) are read last,
/// from both.
pub(super) fn complete(
    git: &Git,
    first: RepositoryState,
    rest: RestParts,
    mut counted: CountedWorktrees,
) -> Result<(RepositoryState, CountedWorktrees), GitError> {
    let rebase_stop = if first.operation == Some(Operation::Rebase) {
        let request = crate::read::rebase::StopRequest {
            git_folder: first.git_folder.clone(),
        };
        crate::read::rebase::read_stop(git, &request)?
    } else {
        None
    };
    let resolved_as_before = read_resolved_as_before(git, &first, rest.rerere)?;
    let state = RepositoryState {
        remote_branches: references::with_remotes(first.remote_branches, &rest.remotes),
        remotes: rest.remotes,
        identity: rest.identity,
        signing: rest.signing,
        branch_sort: rest.branch_sort,
        reflog_subjects: rest.reflog_subjects,
        rebase_stop,
        bisect: rest.bisect,
        submodules: rest.submodules,
        shallow: rest.shallow,
        skipped: rest.kept.skipped.files.clone(),
        is_looked_after: rest.kept.background.is_registered,
        commit_graph: rest.kept.commit_graph.graph,
        notes: rest.kept.notes.clone(),
        lfs: rest.lfs,
        rerere: rest.rerere,
        resolved_as_before,
        ..first
    };
    counted.skipped = Some(rest.kept.skipped);
    counted.background = Some(rest.kept.background);
    counted.commit_graph = Some(rest.kept.commit_graph);
    counted.notes = Some(rest.kept.notes);
    Ok((state, counted))
}

/// The files git resolved as before, while it remembers resolutions and an operation is stopped or files are in
/// conflict (`rerere.read-resolved`); none otherwise, without starting git.
fn read_resolved_as_before(
    git: &Git,
    first: &RepositoryState,
    rerere: model::rerere::RerereState,
) -> Result<Vec<model::rerere::ResolvedAsBefore>, GitError> {
    let conflicted = &first.working_tree.conflicted;
    if !rerere.is_on || (first.operation.is_none() && conflicted.is_empty()) {
        return Ok(Vec::new());
    }
    let request = model::rerere::ResolvedRequest {
        git_folder: first.git_folder.clone(),
        conflicted: conflicted.iter().map(|file| file.path.clone()).collect(),
    };
    crate::read::rerere::read_resolved(git, &request)
}

/// What the reads kept from the read before found.
struct Kept {
    skipped: crate::read::changes::SkippedRead,
    background: crate::read::upkeep::BackgroundRead,
    commit_graph: crate::read::upkeep::CommitGraphRead,
    notes: model::note::Notes,
}

/// The bisect under way in the repository at `location`, if any.
fn read_bisect(git: &Git, location: &Location) -> Result<Option<model::bisect::Bisect>, GitError> {
    let request = crate::read::bisect::BisectRequest {
        git_folder: location.git_folder.clone(),
    };
    crate::read::bisect::read(git, &request)
}

/// The submodules of the repository at `location`, and theirs.
fn read_submodules(git: &Git, location: &Location) -> Result<Vec<model::submodule::Submodule>, GitError> {
    let request = crate::read::submodule::SubmoduleListRequest {
        folder: location.folder.clone(),
    };
    crate::read::submodule::read_list(git, &request)
}

/// Where the history of the repository at `location` ends, when it is a shallow clone.
fn read_shallow_of(git: &Git, location: &Location) -> Result<Shallow, GitError> {
    let request = ShallowRequest {
        folder: location.folder.clone(),
    };
    read_shallow(git, &request)
}

/// Whether git remembers how conflicts are resolved in the repository at `location`.
fn read_rerere(git: &Git, location: &Location) -> Result<model::rerere::RerereState, GitError> {
    let request = model::rerere::RerereStateRequest {
        git_folder: location.git_folder.clone(),
    };
    crate::read::rerere::read_state(git, &request)
}

/// The reads kept from the read before while what they read did not change: the files hidden from git, while the index
/// did not change (`changes.read-skipped`); whether git's maintenance looks after the repository, while the user's
/// configuration did not (`upkeep.read-background`); whether git keeps its commit-graph file, while the object folder
/// and the configuration did not change (`upkeep.read-commit-graph`); and the commits with a note, while the reference
/// of notes did not move (`note.read`).
fn read_kept(git: &Git, location: &Location, before: &CountedWorktrees) -> Result<Kept, GitError> {
    let skipped_request = crate::read::changes::SkippedRequest {
        git_folder: location.git_folder.clone(),
        before: before.skipped.clone(),
    };
    let background_request = crate::read::upkeep::BackgroundRequest {
        folder: location.folder.clone(),
        before: before.background.clone(),
    };
    let commit_graph_request = crate::read::upkeep::CommitGraphRequest {
        folder: location.folder.clone(),
        before: before.commit_graph.clone(),
    };
    let notes_request = crate::read::note::NoteRequest {
        before: before.notes.clone(),
    };
    Ok(Kept {
        skipped: crate::read::changes::read_skipped(git, &skipped_request)?,
        background: crate::read::upkeep::read_background_kept(git, &background_request)?,
        commit_graph: crate::read::upkeep::read_commit_graph_kept(git, &commit_graph_request)?,
        notes: crate::read::note::read(git, &notes_request)?,
    })
}
