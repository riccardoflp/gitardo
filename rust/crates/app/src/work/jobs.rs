//! What each job does on a worker: the reads of git a window asks for, each turned into the message that carries its
//! result back.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use git::read::commit_page::{PatchRequest, read_patch};
use git::read::details::{CommitRequest, read_commit};
use git::read::graph::{CommitPage, CommitsRequest, read_commits, read_position};
use git::read::repo::{RepositoriesRequest, StateRequest, read_repositories, read_state_counting};
use git::read::search::{read_matches, read_revision};
use git::read::worktree::CountedWorktrees;
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
        JobKind::ReadRepository { .. } => repository_read(job, cancel),
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
        JobKind::ReadFile { folder, job: file } => Message::FileRead {
            job: job.id,
            result: super::file_view::read_file(folder, file, cancel),
        },
        JobKind::ReadComparison { folder, target } => Message::ComparisonRead {
            job: job.id,
            result: super::compare::read_comparison(folder, target, cancel).map(Box::new),
        },
        JobKind::ReadInPlace { folder, request } => Message::InPlaceRead {
            job: job.id,
            result: super::compare::read_in_place(folder, request, cancel),
        },
        JobKind::ReadReview { folder, request } => Message::ReviewRead {
            job: job.id,
            result: super::review::read_review(folder, request, cancel).map(Box::new),
        },
        JobKind::CountConflicts { .. }
        | JobKind::ReadNewWorktrees { .. }
        | JobKind::ReadFiles { .. }
        | JobKind::ReadBlame { .. }
        | JobKind::ReadIgnored { .. } => files_read(job, cancel),
    }
}

/// The message of a read of the Files page, of a blame, of the conflicts counted, of the page of new worktrees, or of
/// the files git ignores among those not saved.
fn files_read(job: &Job, cancel: &CancelToken) -> Message {
    match &job.kind {
        JobKind::ReadBlame { folder, job: blame } => Message::BlameRead {
            job: job.id,
            result: super::blame::read_blame(folder, blame, cancel),
        },
        JobKind::CountConflicts { folder } => Message::ConflictsCounted {
            job: job.id,
            result: count_conflicts(folder, cancel).map_err(|error| error.to_string()),
        },
        JobKind::ReadNewWorktrees { folder, request } => Message::NewWorktreesRead {
            job: job.id,
            result: Box::new(super::new_worktrees::read_setup(folder, request, cancel)),
        },
        JobKind::ReadFiles { folder } => Message::FilesRead {
            job: job.id,
            result: git::read::files::read_files(&Git::new(folder).with_cancel(cancel.clone()))
                .map_err(|error| error.to_string()),
        },
        JobKind::ReadIgnored { folder, paths } => Message::IgnoredRead {
            job: job.id,
            result: git::read::ignored::read_ignored(&Git::new(folder).with_cancel(cancel.clone()), paths)
                .map_err(|error| error.to_string()),
        },
        _ => Message::FilesRead {
            job: job.id,
            result: Err(String::from("not a read of files")),
        },
    }
}

/// The message of a read of the repository, with when it was read.
fn repository_read(job: &Job, cancel: &CancelToken) -> Message {
    let JobKind::ReadRepository {
        folder,
        request,
        counted,
    } = &job.kind
    else {
        return Message::RepositoryRead {
            job: job.id,
            folder: PathBuf::new(),
            result: Err(String::from("not a read of a repository")),
            read_at: seconds_now(),
        };
    };
    Message::RepositoryRead {
        job: job.id,
        folder: folder.clone(),
        result: read_repository_counting(folder, request, counted, cancel)
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
    read_repository_counting(folder, request, &CountedWorktrees::default(), cancel)
        .map(|(state, commits, _counted)| (state, commits))
}

/// A repository as [`read_repository`] reads it, each other worktree keeping the count `counted` has of it when nothing
/// changed in its git folder since (`repo.read-state`, M15); with what was counted, for the read after it.
///
/// # Errors
///
/// What git said when a read failed, as when the folder is not a repository any more.
pub fn read_repository_counting(
    folder: &Path,
    request: &TimelineRequest,
    counted: &CountedWorktrees,
    cancel: &CancelToken,
) -> Result<(RepositoryState, CommitPage, CountedWorktrees), git::GitError> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let request_state = StateRequest {
        folder: folder.to_path_buf(),
    };
    let (state, counted) = read_state_counting(&git, &request_state, counted)?;
    let commits = read_commits(&git.in_directory(&state.folder), &commits_request(&state, request))?;
    Ok((state, commits, counted))
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
