//! The reads of the timeline on a worker (`graph.read-commits`, `graph.load-more`): one `git log` read as git prints
//! it, its first page sent in the message of the read, each page after it in a message of its own, up to the 100,000
//! commits of one read.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use git::read::graph::{CommitPage, CommitsRequest, READ_LIMIT, read_commits, read_commits_at_once};
use git::read::repo::{StateRequest, read_state, read_state_counting};
use git::read::worktree::CountedWorktrees;
use git::{CancelToken, Git, GitError};
use model::graph::Row;
use model::state::RepositoryState;

use crate::store::{JobId, Known, Message, MorePages, ReadResult, TimelineRequest, commits_request};
use engine::timeline::is_same_history;

/// What the first page of a read is sent in, and the pages after it.
pub(super) struct Sender<'a, First> {
    /// The job.
    pub(super) job: JobId,
    /// Makes the message of the first page; none once it is sent.
    pub(super) first: Option<First>,
    /// Where the messages go.
    pub(super) send: &'a mut dyn FnMut(Message),
}

impl<First: FnOnce(Result<CommitPage, String>) -> Message> Sender<'_, First> {
    /// Sends a page: in the message of the read when it is the first, in a message of its own otherwise.
    pub(super) fn page(&mut self, result: Result<CommitPage, String>) {
        let message = match self.first.take() {
            Some(first) => first(result),
            None => Message::PagesRead { job: self.job, result },
        };
        (self.send)(message);
    }

    /// Sends what git said of a read that failed, in the message of the read when no page was sent yet.
    pub(super) fn failed(&mut self, error: &GitError) {
        self.page(Err(error.to_string()));
    }
}

/// The commits of a read of the window: every commit up to 100,000, a page at a time.
pub(super) fn whole(request: CommitsRequest) -> CommitsRequest {
    CommitsRequest {
        count: READ_LIMIT,
        skip: 0,
        ..request
    }
}

/// What a read of a repository is given: its folder, the history the timeline shows, the state of the read before when
/// the timeline may keep its rows, and what the read before counted in the other worktrees.
pub struct ReadingRepository<'a> {
    /// The folder of the repository.
    pub folder: &'a Path,
    /// The history the timeline shows.
    pub request: &'a TimelineRequest,
    /// The state of the read before, when the timeline may keep its rows (`repo.schedule-reads`).
    pub kept: Option<&'a RepositoryState>,
    /// What the read before counted in the other worktrees; empty to count them all again (M15).
    pub counted: &'a CountedWorktrees,
}

/// Reads a repository as the window shows it: its state (`repo.read-state`), each other worktree keeping the count of
/// the read before when nothing changed in its git folder since, then its commits as git prints them
/// (`graph.read-commits`), the first page with the state and what was counted. When the state of the read before is
/// given and the history is the same, the commits are not read: the timeline keeps its rows (`repo.schedule-reads`).
pub fn read_repository(job: JobId, read: &ReadingRepository<'_>, cancel: &CancelToken, send: &mut dyn FnMut(Message)) {
    let read_at = seconds_now();
    let git = Git::new(read.folder).with_cancel(cancel.clone());
    let folder_read = read.folder.to_path_buf();
    let message = |result: ReadResult| Message::RepositoryRead {
        job,
        folder: folder_read.clone(),
        result,
        read_at,
    };
    let request = StateRequest {
        folder: read.folder.to_path_buf(),
    };
    let (state, counted) = match read_state_counting(&git, &request, read.counted) {
        Ok(found) => found,
        Err(error) => return send(message(Err(error.to_string()))),
    };
    if read
        .kept
        .is_some_and(|kept| is_same_history(kept, &state, read.request))
    {
        return send(message(Ok(Box::new((state, None, counted)))));
    }
    let commits = whole(commits_request(&state, read.request));
    let git = git.in_directory(&state.folder);
    let mut first = Some((state, counted));
    let mut sender = Sender {
        job,
        first: Some(|result: Result<CommitPage, String>| {
            message(result.map(|page| {
                let (state, counted) = first.take().unwrap_or_default();
                Box::new((state, Some(page), counted))
            }))
        }),
        send,
    };
    if let Err(error) = read_commits(&git, &commits, &mut |page| sender.page(Ok(page))) {
        sender.failed(&error);
    }
}

/// Reads another history of the timeline, the state of the repository staying as last read (`filter.show-only`).
pub fn read_timeline(
    job: JobId,
    (folder, request): (&Path, &CommitsRequest),
    cancel: &CancelToken,
    send: &mut dyn FnMut(Message),
) {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let mut sender = Sender {
        job,
        first: Some(|result| Message::TimelineRead { job, result }),
        send,
    };
    if let Err(error) = read_commits(&git, &whole(request.clone()), &mut |page| sender.page(Ok(page))) {
        sender.failed(&error);
    }
}

/// Reads the commits after the last one the window has, past those one read lists (`graph.load-more`): from that
/// commit again, so that only the rows after it are sent when the history is the same up to it, which a read of that
/// one commit tells first; every row of the history again otherwise.
pub fn read_more(
    job: JobId,
    (folder, request, known): (&PathBuf, &CommitsRequest, &Known),
    cancel: &CancelToken,
    send: &mut dyn FnMut(Message),
) {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let first = |result: Result<MorePages, String>| Message::CommitsRead { job, result };
    let (Some(last), Some(skip)) = (&known.last, known.commits.checked_sub(1)) else {
        return read_again(job, (&git, request), first, send);
    };
    let after = CommitsRequest {
        count: READ_LIMIT,
        skip,
        stashes: Vec::new(),
        ..request.clone()
    };
    let check = CommitsRequest {
        count: 1,
        ..after.clone()
    };
    match read_commits_at_once(&git, &check) {
        Ok(page) if page.rows.first().and_then(Row::id) == Some(last) => {}
        Ok(_) => return read_again(job, (&git, request), first, send),
        Err(error) => return send(first(Err(error.to_string()))),
    }
    let mut sender = Sender {
        job,
        first: Some(|result: Result<CommitPage, String>| first(result.map(appended))),
        send,
    };
    if let Err(error) = read_commits(&git, &after, &mut |page| sender.page(Ok(page))) {
        sender.failed(&error);
    }
}

/// The first page read from the last commit known, as the rows after it.
fn appended(mut page: CommitPage) -> MorePages {
    MorePages::Appended {
        rows: page.rows.split_off(1.min(page.rows.len())),
        has_more: page.has_more,
        follows: page.follows,
    }
}

/// Reads the whole history again, the history having changed under the commits known.
fn read_again(
    job: JobId,
    (git, request): (&Git, &CommitsRequest),
    first: impl FnOnce(Result<MorePages, String>) -> Message,
    send: &mut dyn FnMut(Message),
) {
    let mut sender = Sender {
        job,
        first: Some(|result: Result<CommitPage, String>| first(result.map(MorePages::Replaced))),
        send,
    };
    if let Err(error) = read_commits(git, &whole(request.clone()), &mut |page| sender.page(Ok(page))) {
        sender.failed(&error);
    }
}

/// The state and every commit of a repository at once, as the window reads it: for the tests that time it.
///
/// # Errors
///
/// What git said when a read failed, as when the folder is not a repository any more.
pub fn read_repository_at_once(
    folder: &Path,
    request: &TimelineRequest,
    cancel: &CancelToken,
) -> Result<(RepositoryState, CommitPage), GitError> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    let state = read_state(
        &git,
        &StateRequest {
            folder: folder.to_path_buf(),
        },
    )?;
    let commits = read_commits_at_once(
        &git.in_directory(&state.folder),
        &whole(commits_request(&state, request)),
    )?;
    Ok((state, commits))
}

/// The moment, in seconds since 1970.
pub(super) fn seconds_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
}
