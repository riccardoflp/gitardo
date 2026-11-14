//! A repository the extension opened: its folder, and the burst of changes its watchers saw, which is read once after
//! the burst is over, as `repo.watch-changes` says how long each kind of change waits.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use model::repo::{Change, watch_changes};

use crate::session::locked;

/// A repository the extension opened.
#[derive(Debug)]
pub struct Repository {
    /// The folder of the top of its working tree, as the extension named it.
    folder: PathBuf,
    /// When the repository is read again; none while no change waits.
    burst: Mutex<Burst>,
    /// Whether `repository/close` dropped it, so that a read waiting for it is not made.
    is_closed: AtomicBool,
}

/// The changes waiting for a read.
#[derive(Debug, Default)]
struct Burst {
    /// When the read starts: as long after the last change as that change waits; none while nothing waits.
    due: Option<Instant>,
    /// Whether a thread waits for that moment already.
    is_waited: bool,
}

/// What the thread that waits for a burst does next.
pub enum Next {
    /// Waits this long more: changes came in the meantime.
    Wait(Duration),
    /// Reads the repository now.
    Read,
    /// Ends: nothing waits any more, or the repository was closed.
    End,
}

impl Repository {
    /// A repository just opened.
    pub fn new(folder: PathBuf) -> Repository {
        Repository {
            folder,
            burst: Mutex::new(Burst::default()),
            is_closed: AtomicBool::new(false),
        }
    }

    /// Its folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Marks it closed: a burst waiting for it is not read.
    pub fn close(&self) {
        self.is_closed.store(true, Ordering::SeqCst);
    }

    /// Takes the paths the watchers saw change: the read waits as long after the last of them as the slowest of them
    /// asks. Says whether a thread must be started to wait for it, which is when none waits yet.
    pub fn changed(&self, paths: &[String]) -> bool {
        let Some(delay) = paths
            .iter()
            .filter_map(|path| watch_changes(&change_of(path)))
            .map(|read| read.delay)
            .max()
        else {
            return false;
        };
        let mut burst = locked(&self.burst);
        let due = Instant::now() + delay;
        burst.due = Some(burst.due.map_or(due, |waiting| waiting.max(due)));
        let starts = !burst.is_waited;
        burst.is_waited = true;
        starts
    }

    /// What the thread that waits for the burst does next.
    pub fn next(&self) -> Next {
        let mut burst = locked(&self.burst);
        if self.is_closed.load(Ordering::SeqCst) {
            *burst = Burst::default();
            return Next::End;
        }
        let Some(due) = burst.due else {
            burst.is_waited = false;
            return Next::End;
        };
        let now = Instant::now();
        if due > now {
            return Next::Wait(due - now);
        }
        burst.due = None;
        Next::Read
    }
}

/// What a path of the extension's watchers is: in the git folder when it starts with `.git/`, in the working tree
/// otherwise.
fn change_of(path: &str) -> Change {
    match path.strip_prefix(".git/") {
        Some(inside) => Change::GitFolder(PathBuf::from(inside)),
        None => Change::WorkingTree(PathBuf::from(path)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{Next, Repository};

    #[test]
    fn a_burst_of_changes_is_waited_for_by_one_thread_and_read_once() {
        let repository = Repository::new(PathBuf::from("/work/app"));
        let paths = [String::from("src/a.rs")];
        assert!(repository.changed(&paths), "the first change starts the wait");
        for _ in 0..9 {
            assert!(!repository.changed(&paths), "the others join it");
        }
        assert!(matches!(repository.next(), Next::Wait(wait) if wait > Duration::from_millis(500)));
    }

    #[test]
    fn what_never_asks_for_a_read_starts_nothing() {
        let repository = Repository::new(PathBuf::from("/work/app"));
        let ignored = [
            String::from(".git/fsmonitor--daemon/cookies/1"),
            String::from(".git/index.lock"),
            String::from(".git/objects/ab/cdef"),
        ];
        assert!(!repository.changed(&ignored));
        assert!(matches!(repository.next(), Next::End));
    }

    #[test]
    fn a_closed_repository_is_not_read() {
        let repository = Repository::new(PathBuf::from("/work/app"));
        assert!(repository.changed(&[String::from(".git/HEAD")]));
        repository.close();
        assert!(matches!(repository.next(), Next::End));
    }
}
