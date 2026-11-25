//! A repository read in two steps on a worker (`repo.read-first`): the first step of its state, then its commits as
//! git prints them, with the rest of its state read beside them (`repo.read-state`).

use std::thread::{self, ScopedJoinHandle};

use git::read::graph::{CommitPage, read_commits};
use git::read::repo::{StateRequest, read_first, read_rest};
use git::read::worktree::CountedWorktrees;
use git::{CancelToken, Git, GitError};
use model::state::RepositoryState;

use super::pages::{ReadingRepository, Sender, seconds_now, whole};
use crate::store::{JobId, Message, ReadResult, commits_request};
use engine::timeline::is_same_history;

/// What the thread that reads the rest of the state gives.
type Rest = Result<(RepositoryState, CountedWorktrees), GitError>;

/// Reads a repository shown for the first time: the first step of its state, sent with the first page of its commits
/// in [`Message::FirstRead`], and the rest of its state, read beside the commits from what the first step found and
/// sent in [`Message::RestRead`] as soon as it is read and the first page is sent. When the state given is of the
/// same history, as the one of another worktree of the repository whose rows the timeline took
/// (`repo.share-history`), the commits are not read. The rest is not cancelled with the pages: the state shown would
/// stay half read.
pub fn read_in_steps(job: JobId, read: &ReadingRepository<'_>, cancel: &CancelToken, send: &mut dyn FnMut(Message)) {
    let read_at = seconds_now();
    let folder = read.folder.to_path_buf();
    let message = |result: ReadResult| Message::FirstRead {
        job,
        folder: folder.clone(),
        result,
        read_at,
    };
    let git = Git::new(read.folder).with_cancel(cancel.clone());
    let first = match read_first(&git, &StateRequest { folder: folder.clone() }) {
        Ok(first) => first,
        Err(error) => return send(message(Err(error.to_string()))),
    };
    let is_kept = read
        .kept
        .is_some_and(|kept| is_same_history(kept, &first.state, read.request));
    let commits = whole(commits_request(&first.state, read.request));
    let git = git.in_directory(&first.state.folder);
    let rest_message = |result: Rest| Message::RestRead {
        job,
        folder: folder.clone(),
        result: result.map(Box::new).map_err(|error| error.to_string()),
        read_at,
    };
    thread::scope(|scope| {
        let beside = first.clone();
        let mut rest = Some(scope.spawn(|| read_rest(&Git::new(read.folder), beside, read.counted)));
        let mut state = Some((first.state, read.counted.clone()));
        if is_kept {
            let (state, counted) = state.take().unwrap_or_default();
            send(message(Ok(Box::new((state, None, counted)))));
        } else {
            let mut sender = Sender {
                job,
                first: Some(|result: Result<CommitPage, String>| {
                    message(result.map(|page| {
                        let (state, counted) = state.take().unwrap_or_default();
                        Box::new((state, Some(page), counted))
                    }))
                }),
                send: &mut *send,
            };
            let outcome = read_commits(&git, &commits, &mut |page| {
                sender.page(Ok(page));
                if let Some(read) = rest.take_if(|rest| rest.is_finished()) {
                    (sender.send)(rest_message(joined(read)));
                }
            });
            if let Err(error) = outcome {
                sender.failed(&error);
            }
        }
        if let Some(read) = rest {
            send(rest_message(joined(read)));
        }
    });
}

/// What the read of the rest returned. A read that panicked panics here too, as it would have without the thread.
fn joined(reader: ScopedJoinHandle<'_, Rest>) -> Rest {
    reader
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}
