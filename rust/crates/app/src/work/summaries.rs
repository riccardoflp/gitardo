//! The reads in the background of what the repositories open and their worktrees are on (`repo.worktree-summary`): one
//! thread at a time, each folder read in turn, never twice in two seconds, and a folder whose git folder did not change
//! read only when its files did.
//!
//! [`git_folder_changed`] is the one place that says whether the git folder of a worktree changed since it was last
//! read: its own git folder, written when a commit is made, the index is written or a fetch brings commits, and the
//! branches and the remote branches of its repository.

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use git::read::summary::worktree_summary;
use git::read::worktree::{GitFolderStamp, git_folder_changed};
use model::repo::WorktreeSummary;

use super::Deliver;
use crate::store::Message;

/// The least time between two reads of a folder.
pub const LEAST_BETWEEN_READS: Duration = Duration::from_secs(2);

/// A folder to read, with what is known of its last read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryAsk {
    /// The folder of the repository, the worktree or the submodule.
    pub folder: PathBuf,
    /// Its git folder when it was last read; none before its first read.
    pub last: Option<GitFolderStamp>,
    /// When it was last read; none before its first read.
    pub read_at: Option<Instant>,
    /// Whether files of its working tree changed, and so it is read even when its git folder did not change.
    pub is_forced: bool,
}

/// What a read of a folder found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryRead {
    /// The folder.
    pub folder: PathBuf,
    /// Its git folder now.
    pub stamp: GitFolderStamp,
    /// When it was looked at.
    pub read_at: Instant,
    /// The summary read, or why git failed; none when nothing changed and git did not run.
    pub result: Option<Result<WorktreeSummary, String>>,
}

/// Reads each folder in turn, waiting first until two seconds have gone since its last read; a folder neither whose
/// files nor whose git folder changed is not read.
#[must_use]
pub fn read_summaries(asks: &[SummaryAsk]) -> Vec<SummaryRead> {
    asks.iter()
        .map(|ask| {
            if let Some(due) = ask.read_at.map(|at| at + LEAST_BETWEEN_READS) {
                thread::sleep(due.saturating_duration_since(Instant::now()));
            }
            let check = git_folder_changed(&ask.folder, ask.last.as_ref());
            let read = ask.is_forced || check.is_changed || check.have_branches_moved;
            SummaryRead {
                folder: ask.folder.clone(),
                stamp: check.stamp,
                read_at: Instant::now(),
                result: read.then(|| worktree_summary(&ask.folder).map_err(|error| error.to_string())),
            }
        })
        .collect()
}

/// Reads the folders on a thread of its own, which sends [`Message::SummariesRead`] when it is done.
pub fn read_in_background(asks: Vec<SummaryAsk>, deliver: &Deliver) {
    let sender = deliver.clone();
    let spawned = thread::Builder::new()
        .name(String::from("summaries"))
        .spawn(move || sender(Message::SummariesRead(read_summaries(&asks))));
    if spawned.is_err() {
        // With no thread nothing is read: the folders say what they said before, and the next change asks again.
        deliver(Message::SummariesRead(Vec::new()));
    }
}
