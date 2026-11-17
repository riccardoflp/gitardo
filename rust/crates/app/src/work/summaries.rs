//! The reads in the background of what the repositories open and their worktrees are on (`repo.worktree-summary`): one
//! thread at a time, each folder read in turn, never twice in two seconds, and a folder whose git folder did not change
//! read only when its files did.
//!
//! [`git_folder_changed`] is the one place that says whether the git folder of a worktree changed since it was last
//! read: what it looks at is the files git changes when a branch moves, a commit is made, the index is written or a
//! fetch brings commits.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use git::read::summary::worktree_summary;
use model::repo::WorktreeSummary;

use super::Deliver;
use crate::store::Message;

/// The least time between two reads of a folder.
pub const LEAST_BETWEEN_READS: Duration = Duration::from_secs(2);

/// When the files of a git folder that say what a worktree is on were last changed: its `HEAD` and its index, and the
/// common folder's `packed-refs`, `FETCH_HEAD` and folders of refs. A file that is not there counts as such.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitFolderStamp(Vec<Option<SystemTime>>);

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
    pub stamp: Option<GitFolderStamp>,
    /// When it was looked at.
    pub read_at: Instant,
    /// The summary read, or why git failed; none when nothing changed and git did not run.
    pub result: Option<Result<WorktreeSummary, String>>,
}

/// The git folder of a worktree, and the common one of its repository: `.git` itself, or the folder its `gitdir:` line
/// names, whose `commondir` names the common one.
fn git_folders(folder: &Path) -> Option<(PathBuf, PathBuf)> {
    let dot_git = folder.join(".git");
    let own = if dot_git.is_dir() {
        dot_git
    } else {
        let text = fs::read_to_string(&dot_git).ok()?;
        let named = PathBuf::from(text.strip_prefix("gitdir:")?.trim());
        if named.is_absolute() { named } else { folder.join(named) }
    };
    let common = fs::read_to_string(own.join("commondir"))
        .ok()
        .map_or_else(|| own.clone(), |text| own.join(text.trim()));
    Some((own, common))
}

/// The stamp of the git folder of a worktree now; none when it has none, its folder being gone.
#[must_use]
pub fn git_folder_stamp(folder: &Path) -> Option<GitFolderStamp> {
    let (own, common) = git_folders(folder)?;
    let changed = |path: PathBuf| fs::metadata(path).and_then(|metadata| metadata.modified()).ok();
    let mut times = vec![changed(own.join("HEAD")), changed(own.join("index"))];
    times.extend(["packed-refs", "FETCH_HEAD", "refs/heads", "refs/remotes"].map(|name| changed(common.join(name))));
    if let Ok(remotes) = fs::read_dir(common.join("refs").join("remotes")) {
        let mut folders: Vec<PathBuf> = remotes.filter_map(Result::ok).map(|entry| entry.path()).collect();
        folders.sort();
        times.extend(folders.into_iter().map(changed));
    }
    Some(GitFolderStamp(times))
}

/// Whether the git folder of a worktree changed since the stamp of its last read: gives the stamp now when it did, or
/// when there was none; none when it is the same.
#[must_use]
pub fn git_folder_changed(folder: &Path, last: Option<&GitFolderStamp>) -> Option<Option<GitFolderStamp>> {
    let now = git_folder_stamp(folder);
    match (last, &now) {
        (Some(last), Some(now)) if last == now => None,
        _ => Some(now),
    }
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
            let changed = git_folder_changed(&ask.folder, ask.last.as_ref());
            let read = ask.is_forced || changed.is_some();
            SummaryRead {
                folder: ask.folder.clone(),
                stamp: changed.unwrap_or_else(|| ask.last.clone()),
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
