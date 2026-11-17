//! The reads in the background of what the repositories open and their worktrees are on (`repo.worktree-summary`): one
//! thread at a time, each folder read in turn, never twice in two seconds, and a folder whose git folder did not change
//! read only when its files did. The folder of a repository open in a tab lists its worktrees too, so that the **▾**
//! of a tab never shown has them.
//!
//! [`git_folder_changed`] is the one place that says whether the git folder of a worktree changed since it was last
//! read: its own git folder, written when a commit is made, the index is written or a fetch brings commits, and the
//! branches and the remote branches of its repository.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use git::Git;
use git::read::summary::worktree_summary;
use git::read::worktree::{GitFolderStamp, git_folder_changed, parse_worktrees, worktrees_arguments};
use model::repo::WorktreeSummary;
use model::state::Worktree;

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
    /// Whether it is the folder of a repository open in a tab, whose worktrees are listed with its summary, for the
    /// **▾** of a tab never shown.
    pub lists_worktrees: bool,
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
    /// The worktrees of its repository, the main one first, when it lists them and was read; none otherwise.
    pub worktrees: Option<Result<Vec<Worktree>, String>>,
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
                worktrees: (read && ask.lists_worktrees).then(|| list_worktrees(&ask.folder)),
            }
        })
        .collect()
}

/// The worktrees of the repository of a folder, from `git worktree list` alone: their folders, branches and commits,
/// without counting their files as the read of the folder shown does.
fn list_worktrees(folder: &Path) -> Result<Vec<Worktree>, String> {
    let output = Git::new(folder)
        .run_read(&worktrees_arguments())
        .map_err(|error| error.to_string())?;
    parse_worktrees(&output).map_err(|error| error.to_string())
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
