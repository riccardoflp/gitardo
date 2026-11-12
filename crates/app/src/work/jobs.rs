//! What each job does on a worker: the reads of git a window asks for, each turned into the message that carries its
//! result back.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use git::read::graph::{CommitPage, CommitsRequest, History, Parents, read_commits};
use git::read::repo::{RepositoriesRequest, StateRequest, read_repositories, read_state};
use git::{CancelToken, Git};
use model::graph::PageSize;
use model::state::RepositoryState;

use crate::store::{Found, Job, JobKind, Message};

/// Runs a job, cancelled through its token, and gives the message of its result.
#[must_use]
pub fn run(job: &Job, cancel: &CancelToken) -> Message {
    match &job.kind {
        JobKind::FindRepositories { folders } => Message::RepositoriesFound {
            job: job.id,
            found: find_repositories(folders, cancel),
        },
        JobKind::ReadRepository { folder, page_size } => Message::RepositoryRead {
            job: job.id,
            folder: folder.clone(),
            result: read_repository(folder, *page_size, cancel)
                .map(Box::new)
                .map_err(|error| error.to_string()),
        },
    }
}

/// The repositories of some folders (`repo.read-repositories`); [`Found::AllGone`] when none of the folders is there.
#[must_use]
pub fn find_repositories(folders: &[PathBuf], cancel: &CancelToken) -> Found {
    let existing: Vec<PathBuf> = folders.iter().filter(|folder| folder.is_dir()).cloned().collect();
    let Some(first) = existing.first() else {
        return Found::AllGone;
    };
    let git = Git::new(first).with_cancel(cancel.clone());
    match read_repositories(&git, &RepositoriesRequest { folders: existing }) {
        Ok(repositories) => Found::Repositories(repositories),
        Err(error) => Found::Failed(error.to_string()),
    }
}

/// A repository as the window shows it: its state (`repo.read-state`), then the first page of its commits
/// (`graph.read-commits`).
///
/// # Errors
///
/// What git said when a read failed, as when the folder is not a repository any more.
pub fn read_repository(
    folder: &Path,
    page_size: PageSize,
    cancel: &CancelToken,
) -> Result<(RepositoryState, CommitPage), git::GitError> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let state = read_state(
        &git,
        &StateRequest {
            folder: folder.to_path_buf(),
        },
    )?;
    let request = CommitsRequest {
        history: History {
            selection: everything(&state),
            narrowing: Vec::new(),
            parents: Parents::All,
        },
        page_size,
        pages: 1,
        stashes: state.stashes.clone(),
    };
    let commits = read_commits(&git.in_directory(&state.folder), &request)?;
    Ok((state, commits))
}

/// The history of every branch, remote branch and tag, and of `HEAD` once there is a commit: what the timeline lists
/// when nothing narrows it. `filter.log-arguments` makes it once the branches shown can be chosen (`filter.show-only`,
/// `filter.remember`).
fn everything(state: &RepositoryState) -> Vec<OsString> {
    let mut selection: Vec<OsString> = ["--branches", "--remotes", "--tags"]
        .into_iter()
        .map(OsString::from)
        .collect();
    if state.head.commit.is_some() {
        selection.push(OsString::from("HEAD"));
    }
    selection
}
