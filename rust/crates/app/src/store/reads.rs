//! The reads of the repository shown that nobody asked for by hand (`repo.watch-changes`, `repo.schedule-reads`):
//! the changes the watcher sees become reads after their delay, one at a time, and the reads the window starts itself
//! (a repository chosen, the timeline read again for a switch) are counted by the same schedule, so that a read of a
//! change never runs beside them. The files of the working tree are asked of git first, and those it ignores ask for
//! nothing (M15).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use model::Timestamp;
use model::repo::{
    Change, FocusChange, LATER, ReadAction, ReadCause, ReadEvent, ReadSchedule, StartedRead, TreeChange,
    schedule_reads, watch_changes,
};

use git::read::worktree::CountedWorktrees;

use super::changed_files::{self, ChangedFiles};
use super::message::{Effect, Job, JobId, JobKind, Message};
use super::state::State;

/// What the window knows of the reads of the repository shown.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Reads {
    /// The schedule.
    pub schedule: ReadSchedule,
    /// The reads of the repository running, by their job: their number in the schedule, why each runs and how many
    /// reads in a row it is.
    pub scheduled: BTreeMap<JobId, StartedRead>,
    /// The working tree whose changes are watched; none while no repository is read.
    pub watched: Option<PathBuf>,
    /// Whether the window opened with `readOnStartup` off and its repository is not read yet: the timeline says **Not
    /// read yet** with **Read the repository**.
    pub is_not_read: bool,
    /// The files of the working tree that changed and wait for git to say whether it ignores them.
    pub files: ChangedFiles,
    /// What the last read counted in the other worktrees, kept by the next read where nothing changed in their git
    /// folders (`repo.read-state`, M15).
    pub counted: CountedWorktrees,
}

impl Reads {
    /// Why a read of the repository runs, and how many reads in a row it is, when the schedule started it or counts
    /// it.
    #[must_use]
    pub fn reason(&self, job: JobId) -> Option<&StartedRead> {
        self.scheduled.get(&job)
    }
}

/// Changes seen by the watcher of the repository, at a moment counted from when the window opened: each change that
/// counts asks for a read after its delay, a file of the working tree once git said it does not ignore it. Nothing is
/// read while no repository is shown.
pub fn changed(state: &mut State, changes: &[Change], at: Duration) -> Vec<Effect> {
    if state.shown_read().is_none() {
        return Vec::new();
    }
    super::tasks_tab::changed(state, None, changes);
    let files_due = state.reads.files.due_at();
    let mut effects = Vec::new();
    for change in changes {
        let change = &missed_by_focus(state, change);
        let Some(request) = watch_changes(change) else {
            continue;
        };
        if let Change::WorkingTree(path) = change
            && changed_files::sort(state, path, at) != TreeChange::Read
        {
            continue;
        }
        effects.extend(tell(state, &ReadEvent::Changed { request, at }));
    }
    let is_files_due_moved = state.reads.files.due_at() != files_due;
    one_wake(state, effects, is_files_due_moved)
}

/// A change as the store knows it: the focus says whether a change came while the window was minimized, which the
/// schedule knows and the window does not.
fn missed_by_focus(state: &State, change: &Change) -> Change {
    match change {
        Change::Focus(focus) => Change::Focus(FocusChange {
            is_change_missed: focus.is_change_missed || state.reads.schedule.is_change_missed(),
            ..*focus
        }),
        other => other.clone(),
    }
}

/// The moment a wake asked for came: the read waiting starts, or the files waiting are asked of git.
pub fn wake(state: &mut State, at: Duration) -> Vec<Effect> {
    let mut effects = tell(state, &ReadEvent::Tick { at });
    effects.extend(changed_files::wake(state, at));
    one_wake(state, effects, false)
}

/// Which files git ignores came back: for the files watched, when they asked, and for the Changes page otherwise.
pub fn ignored_read(state: &mut State, job: JobId, result: Result<Vec<String>, String>) -> Vec<Effect> {
    if state.reads.files.is_asking(job) {
        files_checked(state, job, result)
    } else {
        super::changes::ignored_read(state, job, result)
    }
}

/// Git said which of the files of a burst it ignores: each of the others asks for a read, as if it had changed when the
/// first of the burst did, and the read starts at once, since its second is over.
pub fn files_checked(state: &mut State, job: JobId, result: Result<Vec<String>, String>) -> Vec<Effect> {
    let Some((paths, since)) = changed_files::answered(state, job, result) else {
        return Vec::new();
    };
    let mut effects = Vec::new();
    for path in paths {
        if let Some(request) = watch_changes(&Change::WorkingTree(path)) {
            effects.extend(tell(state, &ReadEvent::Changed { request, at: since }));
        }
    }
    effects.extend(tell(state, &ReadEvent::Tick { at: since + LATER }));
    one_wake(state, effects, true)
}

/// A read asked for at once, such as for settings that change what is read.
pub fn asked(state: &mut State, cause: ReadCause) -> Vec<Effect> {
    if state.shown_read().is_none() {
        return Vec::new();
    }
    tell(state, &ReadEvent::Asked(cause))
}

/// The window was minimized, or shown again.
pub fn minimized(state: &mut State, is_minimized: bool) -> Vec<Effect> {
    tell(state, &ReadEvent::Minimized(is_minimized))
}

/// A read the window started itself, after cancelling the reads running: the schedule forgets the read it counted on
/// and counts this one, so that the reads of changes wait for it.
pub fn adopt(state: &mut State, job: JobId, cause: ReadCause) {
    state.reads.files.read_started();
    let (schedule, actions) = schedule_reads(&state.reads.schedule, &ReadEvent::Restarted(cause));
    state.reads.schedule = schedule;
    for action in actions {
        match action {
            ReadAction::Start(started) => {
                let _counted = state.reads.scheduled.insert(job, started);
            }
            ReadAction::Discard(number) => {
                state.reads.scheduled.retain(|_, read| read.number != number);
            }
            ReadAction::WakeAt(_) | ReadAction::Show(_) | ReadAction::StopBar => {}
        }
    }
}

/// The reads running were cancelled, and nothing replaces them.
pub fn stopped(state: &mut State) {
    let (schedule, _actions) = schedule_reads(&state.reads.schedule, &ReadEvent::Stopped);
    state.reads.schedule = schedule;
    state.reads.scheduled.clear();
    state.reads.files.read_started();
}

/// A read of the repository ended: whether what it found is shown, and the read that follows it, if one does.
pub fn finished(state: &mut State, job: JobId) -> (bool, Vec<Effect>) {
    let Some(read) = state.reads.scheduled.remove(&job) else {
        return (true, Vec::new());
    };
    let (schedule, actions) = schedule_reads(&state.reads.schedule, &ReadEvent::Finished(read.number));
    state.reads.schedule = schedule;
    let is_shown = !actions.contains(&ReadAction::Discard(read.number));
    let effects = carry_out(state, actions);
    (is_shown, effects)
}

/// The working tree to watch now, when it is not the one watched: the repository shown once read, none otherwise.
pub fn watch(state: &mut State) -> Option<Effect> {
    let folder = state.shown_read().map(|read| read.folder.clone());
    if folder == state.reads.watched {
        return None;
    }
    state.reads.watched.clone_from(&folder);
    Some(Effect::WatchRepository(folder))
}

/// The debug line of a read of the repository that took a while, in the Command log at `log.level` `debug`.
pub fn read_took(state: &mut State, (line, took, time): (String, Duration, Timestamp)) {
    state.log.record_read(time, line, took);
}

/// Tells the schedule an event and carries out what it asks.
fn tell(state: &mut State, event: &ReadEvent) -> Vec<Effect> {
    let (schedule, actions) = schedule_reads(&state.reads.schedule, event);
    state.reads.schedule = schedule;
    carry_out(state, actions)
}

/// What the schedule asked: reads started, reads thrown away, wakes asked for.
fn carry_out(state: &mut State, actions: Vec<ReadAction>) -> Vec<Effect> {
    let mut effects = Vec::new();
    for action in actions {
        match action {
            ReadAction::Start(started) => effects.extend(start(state, started)),
            ReadAction::Discard(number) => {
                let jobs: Vec<JobId> = state
                    .reads
                    .scheduled
                    .iter()
                    .filter(|(_, read)| read.number == number)
                    .map(|(job, _)| *job)
                    .collect();
                for job in jobs {
                    let _forgotten = state.reads.scheduled.remove(&job);
                    if state.jobs.remove(&job).is_some() {
                        effects.push(Effect::Cancel(job));
                    }
                }
            }
            ReadAction::WakeAt(at) => effects.push(Effect::WakeAt(at)),
            ReadAction::Show(_) | ReadAction::StopBar => {}
        }
    }
    effects
}

/// Starts a read the schedule asked for: the repository shown, its whole history, which a read for a change of files
/// does not read again when it is the same. With no repository shown any more, the schedule is told the read stopped.
fn start(state: &mut State, started: StartedRead) -> Option<Effect> {
    let Some(folder) = state.shown_read().map(|read| read.folder.clone()) else {
        stopped(state);
        return None;
    };
    state.reads.files.read_started();
    let request = super::timeline::read_request(state);
    let kept = super::timeline::kept_for(state, &folder).filter(|_| started.cause.may_keep_commits());
    // **Refresh** counts what is not committed in every other worktree again; the other reads keep the counts of the
    // worktrees whose git folders did not change.
    let counted = if started.cause == ReadCause::Refresh {
        CountedWorktrees::default()
    } else {
        state.reads.counted.clone()
    };
    let id = JobId(state.next_job);
    state.next_job += 1;
    let kind = JobKind::ReadRepository {
        folder,
        request,
        kept,
        counted,
    };
    let _new = state.jobs.insert(id, kind.clone());
    let _counted = state.reads.scheduled.insert(id, started);
    Some(Effect::Start(Job { id, kind }))
}

/// Keeps one wake of the effects, at the first moment something waits for: the read the schedule waits to start, or
/// the files waiting to be asked of git. One timer wakes the store, and each wake asked for replaces the one before.
/// It is asked when the effects asked for one, or when `is_moved` says the moment of the files changed.
fn one_wake(state: &State, effects: Vec<Effect>, is_moved: bool) -> Vec<Effect> {
    let is_asked = is_moved || effects.iter().any(|effect| matches!(effect, Effect::WakeAt(_)));
    let mut effects: Vec<Effect> = effects
        .into_iter()
        .filter(|effect| !matches!(effect, Effect::WakeAt(_)))
        .collect();
    let first = [state.reads.schedule.due_at(), state.reads.files.due_at()]
        .into_iter()
        .flatten()
        .min();
    if let Some(at) = first.filter(|_| is_asked) {
        effects.push(Effect::WakeAt(at));
    }
    effects
}

/// Notes what asks nothing of the window: the summaries of the repositories read, and how long a read took.
pub(super) fn noted(state: &mut State, message: Message) -> Vec<Effect> {
    match message {
        Message::SummariesRead(reads) => super::tabs::summaries_read(state, reads),
        Message::ReadTook { line, took, time } => read_took(state, (line, took, time)),
        _ => {} // The other messages are reduced elsewhere.
    }
    Vec::new()
}
