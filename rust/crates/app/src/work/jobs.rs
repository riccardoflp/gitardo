//! What each job does on a worker: the reads of git a window asks for, each turned into the message that carries its
//! result back, or the messages of its pages for the reads of the timeline (`work::pages`).

use std::path::{Path, PathBuf};

use git::read::commit_page::{PatchRequest, read_patch};
use git::read::details::{CommitRequest, read_commit};
use git::read::graph::read_position;
use git::read::repo::{RepositoriesRequest, read_repositories};
use git::read::search::{read_matches, read_revision};
use git::{CancelToken, Git};
use model::commit_page::PatchLines;
use model::details::CommitDetail;

use crate::store::search::{SearchJob, SearchResult};
use crate::store::{Found, Job, JobKind, Message};

/// Runs a job, cancelled through its token, and gives `send` the message of its result: the reads of the timeline
/// give one for each page, as git prints them.
pub fn run(job: &Job, cancel: &CancelToken, send: &mut dyn FnMut(Message)) {
    match &job.kind {
        JobKind::ReadRepository {
            folder,
            request,
            kept,
            counted,
        } => {
            let read = super::pages::ReadingRepository {
                folder,
                request,
                kept: kept.as_deref(),
                counted,
            };
            super::pages::read_repository(job.id, &read, cancel, send);
        }
        JobKind::ReadTimeline { folder, request } => {
            super::pages::read_timeline(job.id, (folder, request), cancel, send);
        }
        JobKind::ReadCommits { folder, request, known } => {
            super::pages::read_more(job.id, (folder, request, known), cancel, send);
        }
        JobKind::FileSearch { folder, job: search } => {
            super::file_search::run(job.id, (folder, search), cancel, send);
        }
        _ => send(answer(job, cancel)),
    }
}

/// Every message a job gives, in order.
#[must_use]
pub fn messages(job: &Job, cancel: &CancelToken) -> Vec<Message> {
    let mut messages = Vec::new();
    run(job, cancel, &mut |message| messages.push(message));
    messages
}

/// The message of the result of a job that answers once.
fn answer(job: &Job, cancel: &CancelToken) -> Message {
    match &job.kind {
        JobKind::FindRepositories { folders } => Message::RepositoriesFound {
            job: job.id,
            found: find_repositories(folders, cancel),
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
        JobKind::ReadImage { folder, job: file } => Message::ImageRead {
            job: job.id,
            image: super::file_view::read_image(folder, file, cancel),
        },
        JobKind::DrawImage { path, bytes, .. } => Message::ImageDrawn {
            job: job.id,
            result: render::image::read_image(bytes, path, super::file_view::DRAW_SCALE),
        },
        JobKind::DrawDiagram { job: diagram, .. } => Message::DiagramDrawn {
            job: job.id,
            diagram: super::diagram::draw_diagram(diagram),
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
        // The reads of files, and those `run` sends itself, which never come here.
        _ => files_read(job, cancel),
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
