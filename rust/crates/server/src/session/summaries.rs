//! What the repositories of the workspace, their worktrees and their submodules are on (`repo.worktree-summary`), for
//! the rows of the extension's Repositories view (milestone M22, D70), as the window of Gitardo Desktop reads them for
//! its own rows. `read/repo.worktree-summary` names the folders to follow; each is read once, on one thread, one at a
//! time, then again only when the extension's watchers saw something change in it (`repository/folderChanged`): a file
//! of its working tree reads it whatever its git folder says, a file of its git folder only when what
//! [`git_folder_changed`] looks at changed, and never more than once in two seconds. A folder asked lists the
//! worktrees of its repository with `git worktree list` alone, and each of them is followed too. Nothing is read
//! while an operation runs or the state of a repository is read. What a read found is said in `repository/summaries`
//! when it is not what was said before; the first read of a folder is said there only when no answer of
//! `read/repo.worktree-summary` under way will hold it.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use git::read::summary::worktree_summary;
use git::read::worktree::{GitFolderStamp, git_folder_changed, parse_worktrees, worktrees_arguments};
use git::{CancelToken, Git};
use model::repo::{Change, WorktreeSummary, watch_changes};
use model::state::Worktree;
use protocol::rpc::{ResponseError, codes, notification_message};
use protocol::state::WorktreeJson;
use protocol::summaries::{
    FolderChangedParams, FolderSummaryJson, SummariesParams, SummariesResult, SummaryJson, methods,
};
use serde_json::Value;

use super::{Session, json, locked};
use crate::repository::change_of;

/// The least time between two reads of a folder.
const LEAST_BETWEEN_READS: Duration = Duration::from_secs(2);

/// How long the thread that reads waits before it looks again, while something else runs or no folder is ready.
const LOOK_AGAIN: Duration = Duration::from_millis(100);

/// The folders followed, and what each said last.
#[derive(Debug, Default)]
pub struct Summaries {
    /// The folders, and whether a thread reads them.
    followed: Mutex<Followed>,
    /// Told each time a read of a folder ended, and when the thread that reads ends.
    read: Condvar,
}

/// The folders followed.
#[derive(Debug, Default)]
struct Followed {
    /// Each folder: those asked, then the worktrees they list.
    entries: Vec<Entry>,
    /// Whether a thread reads the folders that are due.
    is_reading: bool,
    /// How many reads of the extension wait for their answer, which will hold every folder followed.
    answering: usize,
}

/// A folder followed.
#[derive(Debug, Clone)]
struct Entry {
    /// The folder of the repository, the worktree or the submodule.
    folder: PathBuf,
    /// Whether the extension asked for it, and so it lists the worktrees of its repository; otherwise it is followed
    /// because a folder asked lists it.
    is_asked: bool,
    /// What it said last; none before its first read answers.
    summary: Option<WorktreeSummary>,
    /// Whether its last read failed.
    is_stale: bool,
    /// The worktrees of its repository as last listed, when it was asked.
    worktrees: Option<Vec<Worktree>>,
    /// Its git folder at its last read.
    stamp: Option<GitFolderStamp>,
    /// When it was last read.
    read_at: Option<Instant>,
    /// How it is to be read; none while nothing changed in it since its last read.
    due: Option<Due>,
}

/// How a folder is to be read.
#[derive(Debug, Clone, Copy, Default)]
struct Due {
    /// Whether files of its working tree changed, or it was never read, which reads it whatever its git folder says.
    is_forced: bool,
    /// Whether the worktrees of its repository are listed again whatever its own git folder says: one was added or
    /// removed.
    lists_again: bool,
}

/// A folder to read, with what is known of its last read.
#[derive(Debug)]
struct Ask {
    /// The folder.
    folder: PathBuf,
    /// Its git folder when it was last read.
    last: Option<GitFolderStamp>,
    /// Whether it is read even when its git folder did not change.
    is_forced: bool,
    /// Whether the worktrees of its repository are listed with it.
    lists_worktrees: bool,
    /// Whether they are listed even when nothing else is read.
    lists_again: bool,
}

/// What a read of a folder found.
#[derive(Debug)]
struct Read {
    /// The folder.
    folder: PathBuf,
    /// Its git folder now.
    stamp: GitFolderStamp,
    /// The summary read, or that git failed; none when nothing changed and git did not run.
    summary: Option<Result<WorktreeSummary, ()>>,
    /// The worktrees of its repository, when they were listed.
    worktrees: Option<Result<Vec<Worktree>, ()>>,
    /// How long git took.
    took: Duration,
}

/// What the thread that reads does next.
enum Next {
    /// Reads this folder.
    Read(Ask),
    /// Looks again in a moment: something else runs, or no folder may be read yet.
    Wait(Duration),
    /// Ends: no folder is due.
    End,
}

impl Entry {
    /// A folder never read, which is read at once.
    fn new(folder: PathBuf, is_asked: bool) -> Entry {
        Entry {
            folder,
            is_asked,
            summary: None,
            is_stale: false,
            worktrees: None,
            stamp: None,
            read_at: None,
            due: Some(Due {
                is_forced: true,
                lists_again: false,
            }),
        }
    }

    /// What it says, as the protocol writes it.
    fn said(&self) -> FolderSummaryJson {
        FolderSummaryJson {
            folder: self.folder.to_string_lossy().into_owned(),
            summary: self.summary.as_ref().map(SummaryJson::from),
            is_stale: self.is_stale,
            worktrees: self
                .worktrees
                .as_ref()
                .map(|worktrees| worktrees.iter().map(WorktreeJson::from).collect()),
        }
    }

    /// When it may be read: two seconds after its last read, at once before its first.
    fn ready_at(&self, now: Instant) -> Instant {
        self.read_at.map_or(now, |read_at| read_at + LEAST_BETWEEN_READS)
    }
}

impl Followed {
    /// The entry of a folder.
    fn entry(&mut self, folder: &Path) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|entry| same_folder(&entry.folder, folder))
    }

    /// Follows these folders in place of those asked before: one followed already keeps what it said, a new one is
    /// read, and one no longer asked nor listed is dropped.
    fn ask(&mut self, folders: &[PathBuf]) {
        for entry in &mut self.entries {
            let is_asked = folders.iter().any(|folder| same_folder(folder, &entry.folder));
            // A worktree followed because another listed it now lists them itself; one no longer asked lists none.
            if is_asked && !entry.is_asked {
                entry.due.get_or_insert_default().lists_again = true;
            } else if !is_asked {
                entry.worktrees = None;
            }
            entry.is_asked = is_asked;
        }
        for folder in folders {
            if self.entry(folder).is_none() {
                self.entries.push(Entry::new(folder.clone(), true));
            }
        }
        self.follow_listed();
    }

    /// Follows every worktree a folder asked lists whose folder is there, and drops those no folder asked lists any
    /// more.
    fn follow_listed(&mut self) {
        let listed: Vec<PathBuf> = self
            .entries
            .iter()
            .filter(|entry| entry.is_asked)
            .flat_map(|entry| entry.worktrees.iter().flatten())
            .filter(|worktree| !worktree.is_missing)
            .map(|worktree| worktree.folder.clone())
            .collect();
        self.entries
            .retain(|entry| entry.is_asked || listed.iter().any(|folder| same_folder(folder, &entry.folder)));
        for folder in listed {
            if self.entry(&folder).is_none() {
                self.entries.push(Entry::new(folder, false));
            }
        }
    }

    /// What the thread that reads does next: the folder due that may be read first, once nothing else runs.
    fn next(&mut self, is_busy: bool) -> Next {
        let now = Instant::now();
        let Some(entry) = self
            .entries
            .iter_mut()
            .filter(|entry| entry.due.is_some())
            .min_by_key(|entry| entry.ready_at(now))
        else {
            self.is_reading = false;
            return Next::End;
        };
        let ready_at = entry.ready_at(now);
        if is_busy {
            return Next::Wait(LOOK_AGAIN);
        }
        if ready_at > now {
            return Next::Wait(ready_at.saturating_duration_since(now).min(LOOK_AGAIN));
        }
        let due = entry.due.take().unwrap_or_default();
        Next::Read(Ask {
            folder: entry.folder.clone(),
            last: entry.stamp.clone(),
            is_forced: due.is_forced,
            lists_worktrees: entry.is_asked,
            lists_again: due.lists_again,
        })
    }

    /// Keeps what a read found: a summary read takes the place of the one before, a read that failed keeps it and
    /// says so, a folder in which nothing changed keeps it as it was; worktrees listed take the place of those before,
    /// and are followed. Gives what the folder says now when it is not what it said before, or it is its first read
    /// and no answer under way will say it.
    fn keep(&mut self, read: Read) -> Option<FolderSummaryJson> {
        let is_answered = self.answering > 0;
        let entry = self.entry(&read.folder)?;
        let before = entry.read_at.map(|_| entry.said());
        entry.stamp = Some(read.stamp);
        entry.read_at = Some(Instant::now());
        match read.summary {
            Some(Ok(summary)) => {
                entry.summary = Some(summary);
                entry.is_stale = false;
            }
            Some(Err(())) => entry.is_stale = true,
            None => {}
        }
        if let Some(Ok(worktrees)) = read.worktrees {
            entry.worktrees = Some(worktrees);
        }
        let said = entry.said();
        self.follow_listed();
        let is_new = before.as_ref().map_or(!is_answered, |before| before != &said);
        is_new.then_some(said)
    }
}

impl Session {
    /// Whether an operation runs in a repository opened, or the state of one is read: the folders followed wait for
    /// them, since on Windows a folder git runs in can be neither moved nor removed.
    fn is_busy(&self) -> bool {
        !self.activities.running().is_empty()
            || locked(&self.repositories)
                .values()
                .any(|repository| repository.is_running())
    }
}

/// The answer of `read/repo.worktree-summary`: these folders are followed in place of those asked before, and once
/// each of them and each worktree they list was read, what each is on.
///
/// # Errors
///
/// When the read is cancelled, or the answer cannot be written.
pub fn follow(session: &Arc<Session>, params: &SummariesParams, cancel: &CancelToken) -> Result<Value, ResponseError> {
    let folders: Vec<PathBuf> = params.folders.iter().map(PathBuf::from).collect();
    {
        let mut followed = locked(&session.summaries.followed);
        followed.ask(&folders);
        followed.answering += 1;
    }
    start_reading(session);
    let mut followed = locked(&session.summaries.followed);
    let mut is_cancelled = false;
    while followed.is_reading && followed.entries.iter().any(|entry| entry.read_at.is_none()) {
        is_cancelled = cancel.is_cancelled() || session.is_shut_down();
        if is_cancelled {
            break;
        }
        followed = session
            .summaries
            .read
            .wait_timeout(followed, LOOK_AGAIN)
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
    followed.answering = followed.answering.saturating_sub(1);
    if is_cancelled {
        return Err(ResponseError::new(codes::CANCELLED, "The read was cancelled."));
    }
    let summaries = followed.entries.iter().map(Entry::said).collect();
    drop(followed);
    json(&SummariesResult { summaries })
}

/// Takes a notification of the extension about the folders followed; any other is left, as JSON-RPC has it.
pub fn notification(session: &Arc<Session>, method: &str, params: Value) {
    if method == methods::FOLDER_CHANGED
        && let Ok(changed) = super::parameters::<FolderChangedParams>(params)
    {
        folder_changed(session, &changed);
    }
}

/// Takes the paths the watchers saw change in a folder followed (`repository/folderChanged`): the folder is read again
/// when one of them asks for a read (`repo.watch-changes`), and whatever changed when the extension names none.
fn folder_changed(session: &Arc<Session>, params: &FolderChangedParams) {
    let changes: Vec<Change> = params
        .paths
        .iter()
        .map(|path| change_of(path))
        .filter(|change| watch_changes(change).is_some())
        .collect();
    if changes.is_empty() && !params.paths.is_empty() {
        return;
    }
    let is_listed = |change: &Change| matches!(change, Change::GitFolder(path) if path.starts_with("worktrees"));
    {
        let mut followed = locked(&session.summaries.followed);
        let Some(entry) = followed.entry(Path::new(&params.folder)) else {
            return;
        };
        let due = entry.due.get_or_insert_default();
        due.is_forced |= changes.is_empty() || changes.iter().any(|change| matches!(change, Change::WorkingTree(_)));
        due.lists_again |= changes.iter().any(is_listed);
    }
    start_reading(session);
}

/// Starts the thread that reads the folders due, unless one reads them already.
fn start_reading(session: &Arc<Session>) {
    {
        let mut followed = locked(&session.summaries.followed);
        if followed.is_reading || !followed.entries.iter().any(|entry| entry.due.is_some()) {
            return;
        }
        followed.is_reading = true;
    }
    let reading = Arc::clone(session);
    let spawned = thread::Builder::new()
        .name(String::from("summaries"))
        .spawn(move || read_due(&reading));
    if spawned.is_err() {
        // With no thread nothing is read: the folders say what they said before, and the next change asks again.
        locked(&session.summaries.followed).is_reading = false;
        session.summaries.read.notify_all();
    }
}

/// Reads the folders due, one at a time, until none is; each read that found something new is said to the extension.
fn read_due(session: &Arc<Session>) {
    loop {
        if session.is_shut_down() {
            locked(&session.summaries.followed).is_reading = false;
            break;
        }
        let is_busy = session.is_busy();
        let next = locked(&session.summaries.followed).next(is_busy);
        let ask = match next {
            Next::Read(ask) => ask,
            Next::Wait(wait) => {
                thread::sleep(wait);
                continue;
            }
            Next::End => break,
        };
        let read = read_folder(&ask);
        if read.summary.is_some() {
            let folder = ask.folder.to_string_lossy();
            session.log_read(&folder, format!("Read what {folder} is on"), read.took);
        }
        let said = locked(&session.summaries.followed).keep(read);
        session.summaries.read.notify_all();
        if let Some(Ok(params)) = said.map(|said| json(&SummariesResult { summaries: vec![said] })) {
            session.send(notification_message(methods::SUMMARIES, params));
        }
    }
    session.summaries.read.notify_all();
}

/// Reads a folder: git runs there only when its files changed, or something of its git folder did since its last
/// read.
fn read_folder(ask: &Ask) -> Read {
    let check = git_folder_changed(&ask.folder, ask.last.as_ref());
    let is_read = ask.is_forced || check.is_changed || check.have_branches_moved;
    let started = Instant::now();
    Read {
        folder: ask.folder.clone(),
        stamp: check.stamp,
        summary: is_read.then(|| worktree_summary(&ask.folder).map_err(|_| ())),
        worktrees: (ask.lists_worktrees && (is_read || ask.lists_again)).then(|| list_worktrees(&ask.folder)),
        took: started.elapsed(),
    }
}

/// The worktrees of the repository of a folder, from `git worktree list` alone: their folders, branches and commits,
/// without counting their files.
fn list_worktrees(folder: &Path) -> Result<Vec<Worktree>, ()> {
    let output = Git::new(folder).run_read(&worktrees_arguments()).map_err(|_| ())?;
    parse_worktrees(&output).map_err(|_| ())
}

/// Whether two folders are the same: on Windows the case and the slashes do not count.
fn same_folder(first: &Path, second: &Path) -> bool {
    folder_key(first) == folder_key(second)
}

/// A folder as it is compared.
fn folder_key(folder: &Path) -> String {
    let text = folder.to_string_lossy().replace('\\', "/");
    let trimmed = text.trim_end_matches('/');
    if cfg!(windows) {
        trimmed.to_lowercase()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests;
