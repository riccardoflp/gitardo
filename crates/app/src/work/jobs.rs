//! What each job does on a worker: the reads of git a window asks for, each turned into the message that carries its
//! result back.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use git::read::commit_page::{PatchRequest, read_patch};
use git::read::details::{CommitRequest, read_commit};
use git::read::graph::{CommitPage, CommitsRequest, read_commits, read_position};
use git::read::repo::{RepositoriesRequest, StateRequest, read_repositories, read_state};
use git::read::search::{read_matches, read_revision};
use git::{CancelToken, Git};
use model::commit_page::PatchLines;
use model::details::CommitDetail;
use model::graph::Row;
use model::state::RepositoryState;

use crate::store::search::{SearchJob, SearchResult};
use crate::store::{Found, Job, JobKind, Known, Message, MorePages, TimelineRequest, commits_request};

/// Runs a job, cancelled through its token, and gives the message of its result.
#[must_use]
pub fn run(job: &Job, cancel: &CancelToken) -> Message {
    match &job.kind {
        JobKind::FindRepositories { folders } => Message::RepositoriesFound {
            job: job.id,
            found: find_repositories(folders, cancel),
        },
        JobKind::ReadRepository { folder, request } => repository_read(job, folder, request, cancel),
        JobKind::ReadTimeline { folder, request } => Message::TimelineRead {
            job: job.id,
            result: read_commits(&Git::new(folder).with_cancel(cancel.clone()), request)
                .map_err(|error| error.to_string()),
        },
        JobKind::ReadCommits { folder, request, known } => Message::CommitsRead {
            job: job.id,
            result: read_more(folder, request, known, cancel).map_err(|error| error.to_string()),
        },
        JobKind::ReadDetails { folder, request } => Message::DetailsRead {
            job: job.id,
            result: read_commit(&Git::new(folder).with_cancel(cancel.clone()), request)
                .map(|detail| detail.map(Box::new))
                .map_err(|error| error.to_string()),
        },
        JobKind::ReadCommitPage { folder, commit } => Message::CommitPageRead {
            job: job.id,
            result: read_commit_page(folder, commit, cancel).map_err(|error| error.to_string()),
        },
        JobKind::Search(search) => Message::Searched {
            job: job.id,
            result: read_search(search, cancel),
        },
        JobKind::ReadDiff { folder, job: diff } => Message::DiffRead {
            job: job.id,
            result: super::diff::read_diff(folder, diff, cancel),
        },
        JobKind::ReadMerge { folder, job: merge } => Message::MergeRead {
            job: job.id,
            result: super::merge_view::read_merge(folder, merge, cancel),
        },
        JobKind::ReadComparison { folder, target } => Message::ComparisonRead {
            job: job.id,
            result: super::compare::read_comparison(folder, target, cancel).map(Box::new),
        },
        JobKind::ReadInPlace { folder, request } => Message::InPlaceRead {
            job: job.id,
            result: super::compare::read_in_place(folder, request, cancel),
        },
        JobKind::CountConflicts { folder } => Message::ConflictsCounted {
            job: job.id,
            result: count_conflicts(folder, cancel).map_err(|error| error.to_string()),
        },
        JobKind::ReadReview { folder, request } => Message::ReviewRead {
            job: job.id,
            result: super::review::read_review(folder, request, cancel).map(Box::new),
        },
    }
}

/// The message of a read of the repository, with when it was read.
fn repository_read(job: &Job, folder: &Path, request: &TimelineRequest, cancel: &CancelToken) -> Message {
    Message::RepositoryRead {
        job: job.id,
        folder: folder.to_path_buf(),
        result: read_repository(folder, request, cancel)
            .map(Box::new)
            .map_err(|error| error.to_string()),
        read_at: seconds_now(),
    }
}

/// How many conflicts the files in conflict have, and how many of them the rules resolve (`conflicts.banner`).
///
/// # Errors
///
/// What git said when a read failed.
pub fn count_conflicts(folder: &Path, cancel: &CancelToken) -> Result<model::conflicts::RuleCounts, git::GitError> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let texts = git::read::conflicts::read_texts(&git, &git::read::conflicts::ConflictFilesRequest)?;
    Ok(features::conflicts::count_by_rules(&texts))
}

/// What a read of the search gives: the commits found (`search.read-matches`), the commit of a name
/// (`search.read-revision`), or where a commit is in the history (`graph.read-position`).
#[must_use]
pub fn read_search(job: &SearchJob, cancel: &CancelToken) -> SearchResult {
    let git = Git::new(job.folder()).with_cancel(cancel.clone());
    let failed = |error: git::GitError| error.to_string();
    match job {
        SearchJob::Matches { request, .. } => SearchResult::Matches(read_matches(&git, request).map_err(failed)),
        SearchJob::Revision { request, .. } => SearchResult::Revision(read_revision(&git, request).map_err(failed)),
        SearchJob::Position { request, .. } => SearchResult::Position(read_position(&git, request).map_err(failed)),
    }
}

/// The commit of a page and the lines it changed in each of its files; none for a commit that is not in the
/// repository.
///
/// # Errors
///
/// What git said when a read failed.
pub fn read_commit_page(
    folder: &Path,
    commit: &model::CommitId,
    cancel: &CancelToken,
) -> Result<Option<Box<(CommitDetail, PatchLines)>>, git::GitError> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let Some(detail) = read_commit(&git, &CommitRequest::Commit(commit.clone()))? else {
        return Ok(None);
    };
    let request = PatchRequest {
        commit: detail.id.clone().unwrap_or_else(|| commit.clone()),
        first_parent: detail.first_parent().cloned(),
        files: detail.files.clone(),
    };
    let patch = read_patch(&git, &request)?;
    Ok(Some(Box::new((detail, patch))))
}

/// The moment, in seconds since 1970.
fn seconds_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
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

/// A repository as the window shows it: its state (`repo.read-state`), then the pages of its commits the timeline has
/// loaded (`graph.read-commits`).
///
/// # Errors
///
/// What git said when a read failed, as when the folder is not a repository any more.
pub fn read_repository(
    folder: &Path,
    request: &TimelineRequest,
    cancel: &CancelToken,
) -> Result<(RepositoryState, CommitPage), git::GitError> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let state = read_state(
        &git,
        &StateRequest {
            folder: folder.to_path_buf(),
        },
    )?;
    let commits = read_commits(&git.in_directory(&state.folder), &commits_request(&state, request))?;
    Ok((state, commits))
}

/// The next pages of the timeline, read from the last commit the window has: only the rows after it when the history
/// is the same up to it, every row of the pages otherwise (`graph.load-more`).
///
/// # Errors
///
/// What git said when the read failed.
pub fn read_more(
    folder: &Path,
    request: &CommitsRequest,
    known: &Known,
    cancel: &CancelToken,
) -> Result<MorePages, git::GitError> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let (Some(last), Some(skip)) = (&known.last, known.commits.checked_sub(1)) else {
        return Ok(MorePages::Replaced(read_commits(&git, request)?));
    };
    let after = CommitsRequest {
        skip,
        ..request.clone()
    };
    let CommitPage { mut rows, has_more } = read_commits(&git, &after)?;
    let first = rows.iter().position(|row| matches!(row, Row::Commit(_)));
    match first.filter(|index| rows.get(*index).and_then(Row::id) == Some(last)) {
        Some(index) => Ok(MorePages::Appended {
            rows: rows.split_off(index + 1),
            has_more,
        }),
        None => Ok(MorePages::Replaced(read_commits(&git, request)?)),
    }
}
