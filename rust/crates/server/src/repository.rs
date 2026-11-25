//! A repository the extension opened: its folder, its engine, which runs its features one at a time on a thread of
//! its own and keeps their journal, the state its last read found, and the burst of changes its watchers saw, which is
//! read once after the burst is over, as `repo.watch-changes` says how long each kind of change waits.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use engine::{Clock, CommandLog, Engine, LogLevel};
use git::Git;
use git::read::worktree::CountedWorktrees;
use model::Timestamp;
use model::repo::{Change, watch_changes};
use model::state::RepositoryState;

use crate::session::locked;

/// A repository the extension opened.
pub struct Repository {
    /// The folder of the top of its working tree, as the extension named it.
    folder: PathBuf,
    /// The work given to the thread that owns its engine.
    jobs: Sender<Job>,
    /// Whether a feature runs, so that a second one is answered `busy`.
    is_running: AtomicBool,
    /// When the repository is read again; none while no change waits.
    burst: Mutex<Burst>,
    /// Whether `repository/close` dropped it, so that a read waiting for it is not made.
    is_closed: AtomicBool,
    /// The state the last read of it found, while no operation ran since (`repo.read-state`, `undo.last`,
    /// `graph.read-commits`).
    kept: Mutex<KeptState>,
    /// What the last read of its state counted in the other worktrees, kept by the next read where nothing changed in
    /// their git folders (`repo.read-state`, M15).
    counted: Mutex<CountedWorktrees>,
    /// The reads of the rest of its state under way, behind a first step answered (`repo.read-first`).
    rests: Mutex<Rests>,
    /// Told when a read of the rest ends.
    rest_ended: Condvar,
}

/// The reads of the rest of a state under way.
#[derive(Debug, Default)]
struct Rests {
    /// How many run.
    under_way: usize,
    /// Whether all of them count what is not committed in every other worktree again.
    count_all: bool,
}

/// A read of the rest of a state under way: when it is dropped, the read ended, and what waits for it goes on.
#[derive(Debug)]
pub struct RestUnderWay(Arc<Repository>);

impl Drop for RestUnderWay {
    fn drop(&mut self) {
        let mut rests = locked(&self.0.rests);
        rests.under_way = rests.under_way.saturating_sub(1);
        drop(rests);
        self.0.rest_ended.notify_all();
    }
}

/// The state of the last read of a repository, kept (M15) so that `journal/last` checks what Undo would take back
/// against it, and the reads of the graph take what is checked out, the references and the stashes from it, instead
/// of reading the repository again. It may be the first step of a state whose rest is still read
/// (`repo.read-first`): what takes it, the journal, the reads of the graph and the worktrees nested in this one,
/// takes only what is checked out, the references, the stashes and the worktrees, all of the first step.
#[derive(Debug, Default)]
struct KeptState {
    /// The state; none before the first read, and once an operation ran since it.
    state: Option<Arc<RepositoryState>>,
    /// Whether it is a whole state, and not the first step of one.
    is_whole: bool,
    /// How many operations ran in the repository: a read that started before one of them ended is not kept.
    operations: u64,
}

/// When a read of the state started: the number of the operations that had run by then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadStarted(u64);

impl fmt::Debug for Repository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Repository")
            .field("folder", &self.folder)
            .field("is_closed", &self.is_closed)
            .finish_non_exhaustive()
    }
}

/// Work for the thread that owns the engine.
type Job = Box<dyn FnOnce(&mut Engine) + Send>;

/// Other work runs with the engine of the repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

/// The system's clock, which the engine stamps its log with.
struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        crate::session::now()
    }
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
        let (jobs, received) = mpsc::channel::<Job>();
        let engine_folder = folder.clone();
        // The engine stays on this thread, which ends when the repository is dropped.
        let _owner = thread::spawn(move || {
            let mut engine = Engine::new(
                Git::new(&engine_folder),
                CommandLog::new(LogLevel::Info),
                Box::new(SystemClock),
            );
            engine.find_operations_with(features::operation);
            for job in received {
                job(&mut engine);
            }
        });
        Repository {
            jobs,
            is_running: AtomicBool::new(false),
            folder,
            burst: Mutex::new(Burst::default()),
            is_closed: AtomicBool::new(false),
            kept: Mutex::new(KeptState::default()),
            counted: Mutex::new(CountedWorktrees::default()),
            rests: Mutex::new(Rests::default()),
            rest_ended: Condvar::new(),
        }
    }

    /// Notes that a read of its state starts now, so that what it finds is kept only when no operation ended meanwhile.
    pub fn read_starts(&self) -> ReadStarted {
        ReadStarted(locked(&self.kept).operations)
    }

    /// Keeps the state a read found, unless an operation ended since the read started: that read may be half from
    /// before the operation and half from after it.
    /// Says whether it was kept.
    pub fn keep_state(&self, started: ReadStarted, state: Arc<RepositoryState>) -> bool {
        self.keep(started, state, true)
    }

    /// Keeps the first step of a state, as [`Repository::keep_state`] keeps a whole one, until its rest is read.
    pub fn keep_first(&self, started: ReadStarted, state: Arc<RepositoryState>) -> bool {
        self.keep(started, state, false)
    }

    /// Keeps a state, whole or the first step of one, unless an operation ended since its read started.
    fn keep(&self, started: ReadStarted, state: Arc<RepositoryState>, is_whole: bool) -> bool {
        let mut kept = locked(&self.kept);
        let is_kept = kept.operations == started.0;
        if is_kept {
            kept.state = Some(state);
            kept.is_whole = is_whole;
        }
        is_kept
    }

    /// Notes that the rest of a state is read behind its first step, until what this gives is dropped.
    pub fn rest_starts(self: &Arc<Self>, counts_all: bool) -> RestUnderWay {
        let mut rests = locked(&self.rests);
        rests.count_all = counts_all && (rests.under_way == 0 || rests.count_all);
        rests.under_way += 1;
        RestUnderWay(Arc::clone(self))
    }

    /// The whole state the rest under way gives, waited for: none when no rest is under way, when a **Refresh** is
    /// asked and the rest under way does not count every other worktree again, or when the rest failed or an
    /// operation ran meanwhile. A read of the state asked while the first step was just answered reads nothing again.
    pub fn state_after_rest(&self, refresh: bool) -> Option<Arc<RepositoryState>> {
        let mut rests = locked(&self.rests);
        if rests.under_way == 0 || (refresh && !rests.count_all) {
            return None;
        }
        while rests.under_way > 0 {
            rests = self.rest_ended.wait(rests).unwrap_or_else(PoisonError::into_inner);
        }
        drop(rests);
        let kept = locked(&self.kept);
        kept.state.clone().filter(|_| kept.is_whole)
    }

    /// Whether `repository/close` dropped it.
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::SeqCst)
    }

    /// The state the last read found, when no operation ran since it.
    pub fn kept_state(&self) -> Option<Arc<RepositoryState>> {
        locked(&self.kept).state.clone()
    }

    /// What the last read counted in the other worktrees.
    pub fn counted(&self) -> CountedWorktrees {
        locked(&self.counted).clone()
    }

    /// Keeps what a read counted in the other worktrees, for the read after it.
    pub fn keep_counted(&self, counted: CountedWorktrees) {
        *locked(&self.counted) = counted;
    }

    /// Notes that an operation ran: the state kept is dropped, and the reads under way are not kept.
    pub fn operation_ran(&self) {
        let mut kept = locked(&self.kept);
        kept.operations += 1;
        kept.state = None;
    }

    /// Its folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Runs work with its engine on the engine's thread, and gives what it gave; [`Busy`] while other work runs there.
    ///
    /// # Errors
    ///
    /// When other work runs, or the thread of the engine is gone.
    pub fn with_engine<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Engine) -> T + Send + 'static,
    ) -> Result<T, Busy> {
        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(Busy);
        }
        let (given, result) = mpsc::channel();
        let job: Job = Box::new(move |engine| {
            let _given = given.send(work(engine));
        });
        let done = self
            .jobs
            .send(job)
            .map_err(|_| Busy)
            .and_then(|()| result.recv().map_err(|_| Busy));
        self.is_running.store(false, Ordering::SeqCst);
        done
    }

    /// Whether work runs with its engine: a feature, from its first question to its last step.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
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
pub fn change_of(path: &str) -> Change {
    match path.strip_prefix(".git/") {
        Some(inside) => Change::GitFolder(PathBuf::from(inside)),
        None => Change::WorkingTree(PathBuf::from(path)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use std::sync::Arc;

    use model::state::RepositoryState;

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
    fn a_state_read_is_kept_until_an_operation_runs_and_a_read_across_an_operation_is_not_kept() {
        let repository = Repository::new(PathBuf::from("/work/app"));
        assert!(repository.kept_state().is_none());
        let started = repository.read_starts();
        assert!(repository.keep_state(started, Arc::new(RepositoryState::default())));
        assert!(repository.kept_state().is_some());
        let across = repository.read_starts();
        repository.operation_ran();
        assert!(repository.kept_state().is_none(), "an operation drops it");
        assert!(!repository.keep_state(across, Arc::new(RepositoryState::default())));
        assert!(
            repository.kept_state().is_none(),
            "a read that started before the operation is not kept"
        );
    }

    #[test]
    fn a_state_asked_while_the_rest_is_read_waits_for_it_and_none_under_way_gives_nothing() {
        let repository = Arc::new(Repository::new(PathBuf::from("/work/app")));
        assert!(repository.state_after_rest(false).is_none(), "no rest is under way");
        let started = repository.read_starts();
        assert!(repository.keep_first(started, Arc::new(RepositoryState::default())));
        let under_way = repository.rest_starts(false);
        assert!(
            repository.state_after_rest(true).is_none(),
            "a Refresh counts them all itself"
        );
        let reading = Arc::clone(&repository);
        let rest = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let whole = RepositoryState {
                remotes: vec![String::from("origin")],
                ..RepositoryState::default()
            };
            assert!(reading.keep_state(started, Arc::new(whole)));
            drop(under_way);
        });
        let state = repository.state_after_rest(false);
        assert_eq!(
            state.map(|state| state.remotes.clone()),
            Some(vec![String::from("origin")])
        );
        rest.join().unwrap();
        assert!(repository.state_after_rest(false).is_none(), "the rest ended");
    }

    #[test]
    fn a_closed_repository_is_not_read() {
        let repository = Repository::new(PathBuf::from("/work/app"));
        assert!(repository.changed(&[String::from(".git/HEAD")]));
        repository.close();
        assert!(matches!(repository.next(), Next::End));
    }
}
